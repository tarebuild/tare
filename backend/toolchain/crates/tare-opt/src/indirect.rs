//! Conservative value flow for the v0 callx ABI. Unknown input is never a code pointer proof.
use super::*;
use std::rc::Rc;

const MAX_VALUES: usize = 16;
const MAX_CELLS: usize = 128;
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Atom {
    Number(u64, bool),
    Stack(usize, i64),
}
#[derive(Clone, Debug, Eq, PartialEq)]
enum Value {
    Unknown,
    Set(Vec<Atom>),
}
impl Value {
    fn number(n: u64) -> Self {
        Self::Set(vec![Atom::Number(n, false)])
    }
    fn pointer(n: u64) -> Self {
        Self::Set(vec![Atom::Number(n, true)])
    }
    fn stack(f: usize, n: i64) -> Self {
        Self::Set(vec![Atom::Stack(f, n)])
    }
    fn merge(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::Set(a), Self::Set(b)) => {
                let mut v = a.clone();
                v.extend(b.iter().cloned());
                v.sort();
                v.dedup();
                if v.len() > MAX_VALUES {
                    Self::Unknown
                } else {
                    Self::Set(v)
                }
            }
            _ => Self::Unknown,
        }
    }
    fn binary(&self, other: &Self, op: u8) -> Self {
        let (Self::Set(a), Self::Set(b)) = (self, other) else {
            return Self::Unknown;
        };
        let mut out = Vec::new();
        for x in a {
            for y in b {
                let value = match (x, y) {
                    (Atom::Number(x, xp), Atom::Number(y, yp)) => Atom::Number(
                        match op {
                            ebpf::ADD64_IMM | ebpf::ADD64_REG => x.wrapping_add(*y),
                            ebpf::SUB64_IMM | ebpf::SUB64_REG => x.wrapping_sub(*y),
                            ebpf::AND64_IMM | ebpf::AND64_REG => x & y,
                            ebpf::OR64_IMM | ebpf::OR64_REG => x | y,
                            ebpf::XOR64_IMM | ebpf::XOR64_REG => x ^ y,
                            ebpf::LSH64_IMM | ebpf::LSH64_REG => x.wrapping_shl(*y as u32),
                            ebpf::RSH64_IMM | ebpf::RSH64_REG => x.wrapping_shr(*y as u32),
                            _ => return Self::Unknown,
                        },
                        (*xp && !*yp
                            && *y == 0
                            && matches!(
                                op,
                                ebpf::ADD64_IMM
                                    | ebpf::ADD64_REG
                                    | ebpf::SUB64_IMM
                                    | ebpf::SUB64_REG
                            ))
                            || (!*xp && *yp && *x == 0 && op == ebpf::ADD64_REG),
                    ),
                    (Atom::Stack(f, x), Atom::Number(y, false))
                        if matches!(op, ebpf::ADD64_IMM | ebpf::ADD64_REG) =>
                    {
                        Atom::Stack(*f, x.wrapping_add(*y as i64))
                    }
                    (Atom::Stack(f, x), Atom::Number(y, false))
                        if matches!(op, ebpf::SUB64_IMM | ebpf::SUB64_REG) =>
                    {
                        Atom::Stack(*f, x.wrapping_sub(*y as i64))
                    }
                    _ => return Self::Unknown,
                };
                out.push(value);
            }
        }
        out.sort();
        out.dedup();
        if out.len() > MAX_VALUES {
            Self::Unknown
        } else {
            Self::Set(out)
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
struct State {
    regs: [Value; 11],
    memory: Rc<BTreeMap<(usize, i64), Value>>,
}
impl State {
    fn entry(function: usize) -> Self {
        let mut s = Self {
            regs: std::array::from_fn(|_| Value::Unknown),
            memory: Rc::new(BTreeMap::new()),
        };
        s.regs[10] = Value::stack(function, 4096);
        s
    }
    fn merge(&mut self, other: &Self) -> bool {
        if self == other {
            return false;
        }
        let before = self.clone();
        for (a, b) in self.regs.iter_mut().zip(&other.regs) {
            *a = a.merge(b);
        }
        let keys: BTreeSet<_> = self
            .memory
            .keys()
            .chain(other.memory.keys())
            .copied()
            .collect();
        let mut mem = BTreeMap::new();
        for key in keys {
            let v = self
                .memory
                .get(&key)
                .unwrap_or(&Value::Unknown)
                .merge(other.memory.get(&key).unwrap_or(&Value::Unknown));
            if v != Value::Unknown {
                mem.insert(key, v);
            }
        }
        self.memory = Rc::new(mem);
        *self != before
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct Analysis {
    pub sites: BTreeMap<usize, Option<BTreeSet<usize>>>,
    pub live: BTreeSet<usize>,
    pub limited: bool,
    pub limit_pc: Option<usize>,
    pub limit_reason: Option<&'static str>,
}
impl Analysis {
    pub fn unresolved(&self) -> bool {
        self.limited || self.sites.values().any(Option::is_none)
    }
}
struct Program<'a> {
    view: &'a ElfView,
    bytes: &'a [u8],
    relocated: BTreeMap<u64, u8>,
    pointer_cells: BTreeSet<u64>,
    constants: BTreeMap<usize, u64>,
    pure_syscalls: BTreeSet<usize>,
    terminating_syscalls: BTreeSet<usize>,
    memory_syscalls: BTreeMap<usize, String>,
}
impl<'a> Program<'a> {
    fn new(view: &'a ElfView, bytes: &'a [u8]) -> Result<Self> {
        let mut p = Self {
            view,
            bytes,
            relocated: BTreeMap::new(),
            pointer_cells: BTreeSet::new(),
            constants: BTreeMap::new(),
            pure_syscalls: BTreeSet::new(),
            terminating_syscalls: BTreeSet::new(),
            memory_syscalls: BTreeMap::new(),
        };
        for section in &view.sections {
            if section.kind != 9 {
                continue;
            }
            let symbols = &view.sections[section.link];
            for at in (section.range.start..section.range.end).step_by(16) {
                let site = u64_at(bytes, at)? as usize;
                let info = u64_at(bytes, at + 8)?;
                let kind = info as u32;
                if kind == 10 && view.text.contains(&site) {
                    let sp = symbols.range.start + (info >> 32) as usize * 24;
                    if u16_at(bytes, sp + 6)? == 0 {
                        let strings = &view.sections[symbols.link];
                        let begin = strings.range.start + u32_at(bytes, sp)? as usize;
                        let tail = &bytes[begin..strings.range.end];
                        let end = tail
                            .iter()
                            .position(|b| *b == 0)
                            .context("syscall symbol name")?;
                        let name = std::str::from_utf8(&tail[..end])?;
                        if matches!(name, "sol_memcpy_" | "sol_memmove_" | "sol_memset_") {
                            p.memory_syscalls
                                .insert((site - view.text.start) / 8, name.to_owned());
                        }
                        if matches!(name, "abort" | "sol_panic_") {
                            p.terminating_syscalls.insert((site - view.text.start) / 8);
                        }
                        if matches!(
                            name,
                            "sol_log_"
                                | "sol_log_64_"
                                | "sol_log_compute_units_"
                                | "sol_log_pubkey"
                                | "sol_log_data"
                                | "sol_set_return_data"
                        ) {
                            p.pure_syscalls.insert((site - view.text.start) / 8);
                        }
                    }
                }
                if !matches!(kind, 1 | 8) {
                    continue;
                }
                let text = view.text.contains(&site);
                let raw = if kind == 1 {
                    u64_at(bytes, symbols.range.start + (info >> 32) as usize * 24 + 8)?
                        .wrapping_add(u32_at(bytes, site + 4)? as u64)
                } else if text {
                    u32_at(bytes, site + 4)? as u64 | ((u32_at(bytes, site + 12)? as u64) << 32)
                } else {
                    u32_at(bytes, site + 4)? as u64
                };
                let value = if raw < ebpf::MM_BYTECODE_START {
                    raw + ebpf::MM_BYTECODE_START
                } else {
                    raw
                };
                if text {
                    p.constants.insert((site - view.text.start) / 8, value);
                } else {
                    for s in &view.sections {
                        if s.range.contains(&site) {
                            let addr = Self::based(s.addr) + (site - s.range.start) as u64;
                            p.pointer_cells.insert(addr);
                            for (n, b) in value.to_le_bytes().into_iter().enumerate() {
                                p.relocated.insert(addr + n as u64, b);
                            }
                            break;
                        }
                    }
                }
            }
        }
        Ok(p)
    }
    fn based(addr: u64) -> u64 {
        if addr < ebpf::MM_BYTECODE_START {
            addr + ebpf::MM_BYTECODE_START
        } else {
            addr
        }
    }
    fn load(&self, address: u64, size: usize) -> Value {
        for s in &self.view.sections {
            if !matches!(s.name.as_str(), ".rodata" | ".data.rel.ro") {
                continue;
            }
            let base = Self::based(s.addr);
            let Some(off) = address.checked_sub(base) else {
                continue;
            };
            if off as usize + size > s.range.len() {
                continue;
            }
            let mut data = [0; 8];
            for n in 0..size {
                data[n] = self
                    .relocated
                    .get(&(address + n as u64))
                    .copied()
                    .unwrap_or(self.bytes[s.range.start + off as usize + n]);
            }
            let value = u64::from_le_bytes(data);
            return if size == 8 && self.pointer_cells.contains(&address) {
                Value::pointer(value)
            } else {
                Value::number(value)
            };
        }
        Value::Unknown
    }
    fn target(&self, address: u64) -> Option<usize> {
        let offset = address.wrapping_sub(Self::based(self.view.text_addr));
        let pc = offset / 8;
        (pc < (self.view.text.len() / 8) as u64).then_some(pc as usize)
    }
}
fn enqueue(
    nodes: &mut BTreeMap<(usize, usize), State>,
    queue: &mut VecDeque<(usize, usize)>,
    key: (usize, usize),
    state: State,
) {
    let changed = if let Some(old) = nodes.get_mut(&key) {
        old.merge(&state)
    } else {
        nodes.insert(key, state);
        true
    };
    if changed {
        queue.push_back(key)
    }
}

pub(super) fn analyze(view: &ElfView, bytes: &[u8], metadata: &Metadata) -> Result<Analysis> {
    let program = Program::new(view, bytes)?;
    let text = &bytes[view.text.clone()];
    let slots = text.len() / 8;
    let entry = view.pc(view.entry).context("entry outside text")?;
    let mut out = Analysis::default();
    let mut nodes = BTreeMap::new();
    let mut queue = VecDeque::new();
    // A call-site context keeps unrelated vtable arguments separate. Reusing a
    // source call-site within one context joins values monotonically.
    let mut contexts = vec![entry];
    let mut callers: BTreeMap<usize, BTreeSet<(usize, usize)>> = BTreeMap::new();
    let mut context_ids: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    let mut exits: BTreeMap<usize, State> = BTreeMap::new();
    enqueue(&mut nodes, &mut queue, (0, entry), State::entry(0));
    let mut steps = 0;
    while let Some((context, pc)) = queue.pop_front() {
        steps += 1;
        if steps > slots.saturating_mul(256).max(16384) || contexts.len() > 32768 {
            out.limited = true;
            out.limit_pc = Some(pc);
            out.limit_reason = Some("bounded instruction/context analysis");
            break;
        }
        if pc >= slots {
            continue;
        }
        out.live.insert(pc);
        let mut state = nodes[&(context, pc)].clone();
        let i = ebpf::get_insn(text, pc);
        let dst = i.dst as usize;
        let src = i.src as usize;
        if i.opc == ebpf::EXIT {
            let changed = if let Some(old) = exits.get_mut(&context) {
                old.merge(&state)
            } else {
                exits.insert(context, state.clone());
                true
            };
            if changed {
                for &(caller, site) in callers.get(&context).into_iter().flatten() {
                    let mut result = state.clone();
                    Rc::make_mut(&mut result.memory).retain(|(frame, _), _| *frame != context);
                    let saved = &nodes[&(caller, site)];
                    for r in 6..11 {
                        result.regs[r] = saved.regs[r].clone();
                    }
                    enqueue(&mut nodes, &mut queue, (caller, site + 1), result);
                }
            }
            continue;
        }
        if matches!(i.opc, ebpf::CALL_IMM | ebpf::CALL_REG) {
            let targets = if i.opc == ebpf::CALL_IMM {
                Some(
                    metadata
                        .calls
                        .get(&pc)
                        .copied()
                        .or_else(|| (i.imm != -1).then(|| target(pc, i.imm, slots)).flatten())
                        .into_iter()
                        .collect::<BTreeSet<_>>(),
                )
            } else {
                let value = &state.regs[i.imm as usize];
                let found = match value {
                    Value::Set(values) => {
                        let mut found = BTreeSet::new();
                        let mut known = true;
                        for atom in values {
                            if let Atom::Number(n, provenance) = atom {
                                if let Some(t) = program.target(*n) {
                                    if *provenance {
                                        found.insert(t);
                                    } else {
                                        known = false;
                                    }
                                }
                            } else {
                                known = false;
                            }
                        }
                        known.then_some(found)
                    }
                    Value::Unknown => None,
                };
                let old = out.sites.entry(pc).or_insert_with(|| Some(BTreeSet::new()));
                match (&mut *old, &found) {
                    (Some(a), Some(b)) => a.extend(b),
                    (a, _) => *a = None,
                };
                found
            };
            if i.opc == ebpf::CALL_IMM && targets.as_ref().is_some_and(BTreeSet::is_empty) {
                if program.terminating_syscalls.contains(&pc) {
                    continue;
                }
                state.regs[0] = Value::Unknown;
                if let Some(name) = program.memory_syscalls.get(&pc) {
                    summarize_memory(&mut state, name, &program);
                } else if !program.pure_syscalls.contains(&pc) {
                    state.memory = Rc::new(BTreeMap::new());
                }
                enqueue(&mut nodes, &mut queue, (context, pc + 1), state);
                continue;
            }
            let Some(targets) = targets else {
                for reg in &mut state.regs[..6] {
                    *reg = Value::Unknown
                }
                state.memory = Rc::new(BTreeMap::new());
                enqueue(&mut nodes, &mut queue, (context, pc + 1), state);
                continue;
            };
            for to in targets {
                let mut ancestors = vec![context];
                let mut seen = BTreeSet::new();
                let mut recursive = false;
                while let Some(id) = ancestors.pop() {
                    if contexts[id] == to {
                        recursive = true;
                        break;
                    }
                    if seen.insert(id) {
                        for &(parent, _) in callers.get(&id).into_iter().flatten() {
                            ancestors.push(parent)
                        }
                    }
                }
                if recursive {
                    out.limited = true;
                    out.limit_pc = Some(pc);
                    out.limit_reason = Some("recursive stack aliases");
                    continue;
                }
                let key = (0, to);
                let child = if let Some(&id) = context_ids.get(&key) {
                    id
                } else {
                    let id = contexts.len();
                    contexts.push(to);
                    context_ids.insert(key, id);
                    id
                };
                callers.entry(child).or_default().insert((context, pc));
                let mut entry_state = state.clone();
                Rc::make_mut(&mut entry_state.memory).retain(|(frame, _), _| *frame != child);
                entry_state.regs[10] = Value::stack(child, 4096);
                enqueue(&mut nodes, &mut queue, (child, to), entry_state);
                if let Some(returned) = exits.get(&child) {
                    let mut result = returned.clone();
                    Rc::make_mut(&mut result.memory).retain(|(frame, _), _| *frame != child);
                    for r in 6..11 {
                        result.regs[r] = state.regs[r].clone();
                    }
                    enqueue(&mut nodes, &mut queue, (context, pc + 1), result);
                }
            }
            continue;
        }
        if is_jump(&i) {
            if let Some(to) = target(pc, i.off as i64, slots) {
                enqueue(&mut nodes, &mut queue, (context, to), state.clone());
            }
            if i.opc == ebpf::JA {
                continue;
            }
        } else if i.opc == ebpf::LD_DW_IMM {
            state.regs[dst] = if let Some(&value) = program.constants.get(&pc) {
                Value::pointer(value)
            } else {
                Value::number(
                    (i.imm as u32 as u64)
                        | ((ebpf::get_insn(text, pc + 1).imm as u32 as u64) << 32),
                )
            };
        } else if i.opc == ebpf::MOV64_REG {
            state.regs[dst] = state.regs[src].clone();
        } else if i.opc == ebpf::MOV64_IMM {
            state.regs[dst] = Value::number(i.imm as u64);
        } else if matches!(
            i.opc,
            ebpf::LD_B_REG | ebpf::LD_H_REG | ebpf::LD_W_REG | ebpf::LD_DW_REG
        ) {
            let size = match i.opc {
                ebpf::LD_B_REG => 1,
                ebpf::LD_H_REG => 2,
                ebpf::LD_W_REG => 4,
                _ => 8,
            };
            let address =
                state.regs[src].binary(&Value::number(i.off as i64 as u64), ebpf::ADD64_IMM);
            let mut value = None;
            if let Value::Set(addresses) = address {
                for a in addresses {
                    let loaded = match a {
                        Atom::Number(n, _) => program.load(n, size),
                        Atom::Stack(f, o) => {
                            if size == 8 {
                                state.memory.get(&(f, o)).cloned().unwrap_or(Value::Unknown)
                            } else {
                                Value::Unknown
                            }
                        }
                    };
                    value = Some(value.map_or(loaded.clone(), |v: Value| v.merge(&loaded)));
                }
            }
            state.regs[dst] = value.unwrap_or(Value::Unknown);
        } else if matches!(
            i.opc,
            ebpf::ST_B_REG
                | ebpf::ST_H_REG
                | ebpf::ST_W_REG
                | ebpf::ST_DW_REG
                | ebpf::ST_B_IMM
                | ebpf::ST_H_IMM
                | ebpf::ST_W_IMM
                | ebpf::ST_DW_IMM
        ) {
            let size = match i.opc & 0x18 {
                0x10 => 1,
                0x08 => 2,
                0 => 4,
                _ => 8,
            };
            let address =
                state.regs[dst].binary(&Value::number(i.off as i64 as u64), ebpf::ADD64_IMM);
            let value = if i.opc & 7 == 3 {
                state.regs[src].clone()
            } else {
                Value::number(i.imm as u64)
            };
            if let Value::Set(ref addresses) = address {
                if addresses.len() == 1 {
                    if let Atom::Stack(f, o) = addresses[0] {
                        let mem = Rc::make_mut(&mut state.memory);
                        mem.retain(|(mf, mo), _| *mf != f || *mo + 8 <= o || *mo >= o + size);
                        if size == 8 && value != Value::Unknown && mem.len() < MAX_CELLS {
                            mem.insert((f, o), value);
                        }
                    } else {
                        state.memory = Rc::new(BTreeMap::new());
                    }
                } else {
                    state.memory = Rc::new(BTreeMap::new());
                }
            } else {
                state.memory = Rc::new(BTreeMap::new());
            }
        } else if i.opc & 7 == 7 {
            let other = if i.opc & 8 != 0 {
                state.regs[src].clone()
            } else {
                Value::number(i.imm as u64)
            };
            state.regs[dst] = state.regs[dst].binary(&other, i.opc);
        } else if i.opc & 7 == 3 {
            state.memory = Rc::new(BTreeMap::new());
        } else if matches!(i.opc & 7, 0 | 1 | 4 | 6) {
            state.regs[dst] = Value::Unknown;
        }
        enqueue(
            &mut nodes,
            &mut queue,
            (context, pc + if i.opc == ebpf::LD_DW_IMM { 2 } else { 1 }),
            state,
        );
    }
    Ok(out)
}

fn summarize_memory(state: &mut State, name: &str, program: &Program<'_>) {
    let (Value::Set(dst), Value::Set(src), Value::Set(length)) =
        (&state.regs[1], &state.regs[2], &state.regs[3])
    else {
        state.memory = Rc::new(BTreeMap::new());
        return;
    };
    let ([Atom::Stack(frame, offset)], [source], [Atom::Number(size, _)]) =
        (dst.as_slice(), src.as_slice(), length.as_slice())
    else {
        state.memory = Rc::new(BTreeMap::new());
        return;
    };
    let Ok(size) = i64::try_from(*size) else {
        state.memory = Rc::new(BTreeMap::new());
        return;
    };
    if size > 65536 {
        state.memory = Rc::new(BTreeMap::new());
        return;
    }
    let mut copies = Vec::new();
    if name != "sol_memset_" {
        for delta in (0..size).step_by(8) {
            if delta + 8 > size {
                break;
            }
            let value = match source {
                Atom::Stack(f, o) => state
                    .memory
                    .get(&(*f, o.wrapping_add(delta)))
                    .cloned()
                    .unwrap_or(Value::Unknown),
                Atom::Number(n, _) => program.load(n.wrapping_add(delta as u64), 8),
            };
            if value != Value::Unknown {
                copies.push(((*frame, offset.wrapping_add(delta)), value));
            }
        }
    } else if let Atom::Number(n, _) = source {
        let word = (*n as u8 as u64) * 0x0101010101010101;
        for delta in (0..size).step_by(8) {
            if delta + 8 <= size {
                copies.push(((*frame, offset.wrapping_add(delta)), Value::number(word)));
            }
        }
    }
    let mem = Rc::make_mut(&mut state.memory);
    mem.retain(|(f, o), _| *f != *frame || *o + 8 <= *offset || *o >= offset.saturating_add(size));
    for (key, value) in copies {
        if mem.len() < MAX_CELLS {
            mem.insert(key, value);
        }
    }
}
