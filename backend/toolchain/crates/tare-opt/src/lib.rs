//! Relocation-aware optimization and compaction of linked sBPF-v0 ELF files.
mod compact;
mod indirect;
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use solana_sbpf::{
    ebpf,
    program::SBPFVersion,
    verifier::{RequisiteVerifier, Verifier},
    vm::Config,
};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    ops::Range,
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Stats {
    pub input_bytes: usize,
    pub output_bytes: usize,
    pub text_instructions: usize,
    pub branch_targets_threaded: usize,
    pub call_targets_threaded: usize,
    pub stack_loads_forwarded: usize,
    pub arithmetic_strength_reductions: usize,
    pub constant_branches_simplified: usize,
    pub identity_runs_skipped: usize,
    pub identity_instructions_bypassed: usize,
    pub unreachable_instructions_neutralized: usize,
    pub unreachable_functions_neutralized: usize,
    pub indirect_calls: usize,
    pub indirect_call_sites: Vec<IndirectCallSite>,
    pub unresolved_indirect_calls: usize,
    pub retained_for_indirect_calls: usize,
    pub relocation_sites_preserved: usize,
    pub root_pcs: Vec<usize>,
    pub modified_pcs: Vec<usize>,
    pub text_slots_removed: usize,
    pub text_bytes_removed: usize,
    pub relocations_removed: usize,
    pub relocations_prelinked: usize,
    pub non_allocated_bytes_removed: usize,
    pub identical_functions_merged: usize,
    pub pc_map: Vec<[usize; 3]>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndirectCallSite {
    pub pc: usize,
    pub targets: Vec<usize>,
    pub resolved: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Optimization {
    pub bytes: Vec<u8>,
    pub stats: Stats,
}

/// Admit with the real native Agave loader and syscall registry.
pub fn validate(elf: &[u8]) -> Result<()> {
    let view = ElfView::parse(elf)?;
    ensure!(
        view.flags == 0,
        "tare-opt supports sBPF v0; ELF e_flags={} is not supported",
        view.flags
    );
    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(solana_address::Address::from([0x54; 32]), elf)
        .context("Agave rejected the SBF ELF")?;
    Ok(())
}

pub fn optimize(elf: &[u8]) -> Result<Optimization> {
    validate(elf)?;
    let view = ElfView::parse(elf)?;
    let mut bytes = elf.to_vec();
    let metadata = view.metadata(elf)?;
    let (optimized, mut stats) = optimize_text(&elf[view.text.clone()], &metadata)?;
    bytes[view.text].copy_from_slice(&optimized);
    let bytes = compact::compact(&bytes, &mut stats)?;
    stats.input_bytes = elf.len();
    stats.output_bytes = bytes.len();
    validate(&bytes).context("optimized ELF failed Agave admission")?;
    Ok(Optimization { bytes, stats })
}

#[derive(Clone, Debug, Default)]
struct Metadata {
    roots: BTreeSet<usize>,
    protected: BTreeSet<usize>,
    functions: BTreeSet<usize>,
    calls: BTreeMap<usize, usize>,
}
#[derive(Clone, Debug)]
struct Section {
    kind: u32,
    addr: u64,
    range: Range<usize>,
    link: usize,
    entry_size: usize,
    name: String,
}
struct ElfView {
    flags: u32,
    entry: u64,
    text: Range<usize>,
    text_addr: u64,
    sections: Vec<Section>,
}

fn u16_at(b: &[u8], p: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        b.get(p..p + 2).context("truncated ELF u16")?.try_into()?,
    ))
}
fn u32_at(b: &[u8], p: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        b.get(p..p + 4).context("truncated ELF u32")?.try_into()?,
    ))
}
fn u64_at(b: &[u8], p: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(
        b.get(p..p + 8).context("truncated ELF u64")?.try_into()?,
    ))
}
fn range(bytes: &[u8], start: u64, size: u64) -> Result<Range<usize>> {
    let start = usize::try_from(start)?;
    let end = start
        .checked_add(usize::try_from(size)?)
        .context("ELF section size overflow")?;
    ensure!(end <= bytes.len(), "ELF section is outside the file");
    Ok(start..end)
}
impl ElfView {
    fn parse(bytes: &[u8]) -> Result<Self> {
        ensure!(
            bytes.len() >= 64 && &bytes[..4] == b"\x7fELF",
            "expected an ELF file"
        );
        ensure!(
            bytes[4] == 2 && bytes[5] == 1 && bytes[6] == 1 && bytes[7] == 0,
            "expected a little-endian ELF64 SBF program"
        );
        ensure!(u16_at(bytes, 16)? == 3, "expected linked ET_DYN ELF");
        ensure!(
            [247, 263].contains(&u16_at(bytes, 18)?),
            "ELF machine is not SBF"
        );
        let flags = u32_at(bytes, 48)?;
        let shoff = usize::try_from(u64_at(bytes, 40)?)?;
        let shsize = usize::from(u16_at(bytes, 58)?);
        let shnum = usize::from(u16_at(bytes, 60)?);
        let names_index = usize::from(u16_at(bytes, 62)?);
        ensure!(
            shsize == 64 && shnum > 0 && names_index < shnum,
            "unsupported ELF section headers"
        );
        let table_size = shnum.checked_mul(shsize).context("ELF table overflow")?;
        ensure!(
            shoff
                .checked_add(table_size)
                .is_some_and(|end| end <= bytes.len()),
            "truncated ELF section table"
        );
        let header = |index: usize| shoff + index * shsize;
        let names_header = header(names_index);
        let names_range = range(
            bytes,
            u64_at(bytes, names_header + 24)?,
            u64_at(bytes, names_header + 32)?,
        )?;
        let names = &bytes[names_range];
        let mut sections = Vec::new();
        for i in 0..shnum {
            let h = header(i);
            let kind = u32_at(bytes, h + 4)?;
            ensure!(
                kind != 4,
                "tare-opt does not implement SHT_RELA relocations"
            );
            let name_index = usize::try_from(u32_at(bytes, h)?)?;
            let rest = names
                .get(name_index..)
                .context("invalid ELF section name")?;
            let end = rest
                .iter()
                .position(|byte| *byte == 0)
                .context("unterminated ELF section name")?;
            let name = std::str::from_utf8(&rest[..end])?.to_owned();
            let data = if kind == 8 {
                0..0
            } else {
                range(bytes, u64_at(bytes, h + 24)?, u64_at(bytes, h + 32)?)?
            };
            sections.push(Section {
                kind,
                addr: u64_at(bytes, h + 16)?,
                range: data,
                link: usize::try_from(u32_at(bytes, h + 40)?)?,
                entry_size: usize::try_from(u64_at(bytes, h + 56)?)?,
                name,
            });
        }
        let texts: Vec<_> = sections.iter().filter(|s| s.name == ".text").collect();
        ensure!(texts.len() == 1, "expected exactly one .text section");
        let text = texts[0];
        ensure!(
            !text.range.is_empty() && text.range.len() % 8 == 0,
            "invalid SBF .text length"
        );
        Ok(Self {
            flags,
            entry: u64_at(bytes, 24)?,
            text: text.range.clone(),
            text_addr: text.addr,
            sections,
        })
    }
    fn pc(&self, address: u64) -> Option<usize> {
        let address = if address >= ebpf::MM_BYTECODE_START {
            address - ebpf::MM_BYTECODE_START
        } else {
            address
        };
        let offset = address.checked_sub(self.text_addr)?;
        (offset < self.text.len() as u64 && offset % 8 == 0).then_some((offset / 8) as usize)
    }
    fn metadata(&self, bytes: &[u8]) -> Result<Metadata> {
        let mut m = Metadata::default();
        m.roots
            .insert(self.pc(self.entry).context("entrypoint not in .text")?);
        for s in &self.sections {
            if s.kind == 2 || s.kind == 11 {
                ensure!(
                    s.entry_size == 24 && s.range.len() % 24 == 0,
                    "invalid ELF symbol table"
                );
                for p in (s.range.start..s.range.end).step_by(24) {
                    if bytes[p + 4] & 0xf == 2 {
                        if let Some(pc) = self.pc(u64_at(bytes, p + 8)?) {
                            m.functions.insert(pc);
                            if s.kind == 11 {
                                m.roots.insert(pc);
                            }
                        }
                    }
                }
            }
            if s.kind != 9 {
                continue;
            }
            ensure!(
                s.entry_size == 16 && s.range.len() % 16 == 0,
                "invalid ELF REL table"
            );
            let symbols = self
                .sections
                .get(s.link)
                .context("REL section has invalid symbol table link")?;
            ensure!(
                [2, 11].contains(&symbols.kind) && symbols.entry_size == 24,
                "REL section links an unsupported symbol table"
            );
            for p in (s.range.start..s.range.end).step_by(16) {
                let site = usize::try_from(u64_at(bytes, p)?)?;
                let info = u64_at(bytes, p + 8)?;
                let kind = info as u32;
                ensure!(
                    [0, 1, 8, 10].contains(&kind),
                    "tare-opt does not implement relocation type {kind}"
                );
                let index = (info >> 32) as usize;
                let sp = symbols
                    .range
                    .start
                    .checked_add(index.checked_mul(24).context("symbol index overflow")?)
                    .context("symbol offset overflow")?;
                ensure!(
                    sp + 24 <= symbols.range.end,
                    "relocation symbol outside table"
                );
                let value = u64_at(bytes, sp + 8)?;
                let in_text = self.text.contains(&site);
                if in_text {
                    ensure!(
                        (site - self.text.start) % 8 == 0,
                        "tare-opt does not implement unaligned text relocations"
                    );
                    let pc = (site - self.text.start) / 8;
                    m.protected.insert(pc);
                    if kind == 1 || kind == 8 {
                        m.protected.insert(pc + 1);
                    }
                    if kind == 10 {
                        if let Some(target) = self.pc(value) {
                            m.calls.insert(pc, target);
                            m.functions.insert(target);
                        }
                    }
                }
                let address = match kind {
                    1 => value.checked_add(u64::from(u32_at(bytes, site + 4)?)),
                    8 if in_text => Some(
                        u64::from(u32_at(bytes, site + 4)?)
                            | (u64::from(u32_at(bytes, site + 12)?) << 32),
                    ),
                    8 => Some(u64::from(u32_at(bytes, site + 4)?)),
                    _ => None,
                };
                if let Some(pc) = address.and_then(|address| self.pc(address)) {
                    m.roots.insert(pc);
                    m.functions.insert(pc);
                }
            }
        }
        let text = &bytes[self.text.clone()];
        for pc in starts(text) {
            let insn = ebpf::get_insn(text, pc);
            if insn.opc == ebpf::LD_DW_IMM {
                let upper = ebpf::get_insn(text, pc + 1).imm as u32;
                let address = (u64::from(upper) << 32) | u64::from(insn.imm as u32);
                if let Some(target) = self.pc(address) {
                    m.roots.insert(target);
                    m.functions.insert(target);
                }
            }
        }
        Ok(m)
    }
}
fn starts(text: &[u8]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut pc = 0;
    while pc < text.len() / 8 {
        out.push(pc);
        pc += if ebpf::get_insn(text, pc).opc == ebpf::LD_DW_IMM {
            2
        } else {
            1
        };
    }
    out
}
fn is_jump(i: &ebpf::Insn) -> bool {
    i.opc & 7 == 5 && ![ebpf::CALL_IMM, ebpf::CALL_REG, ebpf::EXIT].contains(&i.opc)
}
fn target(pc: usize, delta: i64, n: usize) -> Option<usize> {
    let to = pc as i64 + 1 + delta;
    (to >= 0 && to < n as i64).then_some(to as usize)
}
fn write(text: &mut [u8], pc: usize, i: ebpf::Insn) {
    text[pc * 8..pc * 8 + 8].copy_from_slice(&i.to_array());
}
fn insn(opc: u8, dst: u8, src: u8, off: i16, imm: i64) -> ebpf::Insn {
    ebpf::Insn {
        ptr: 0,
        opc,
        dst,
        src,
        off,
        imm,
    }
}

fn optimize_text(text: &[u8], m: &Metadata) -> Result<(Vec<u8>, Stats)> {
    RequisiteVerifier::verify(text, &Config::default(), SBPFVersion::V0)
        .context("invalid SBF-v0 instructions")?;
    let mut out = text.to_vec();
    let pcs = starts(text);
    let n = text.len() / 8;
    let indirect = pcs
        .iter()
        .filter(|pc| ebpf::get_insn(text, **pc).opc == ebpf::CALL_REG)
        .count();
    let mut stats = Stats {
        text_instructions: pcs.len(),
        indirect_calls: indirect,
        relocation_sites_preserved: m.protected.len(),
        root_pcs: m.roots.iter().copied().collect(),
        ..Stats::default()
    };
    let live = reachability(text, m, indirect != 0);
    if indirect == 0 {
        for pc in &pcs {
            if !live.contains(pc) && !m.protected.contains(pc) {
                let old = ebpf::get_insn(&out, *pc);
                if old.opc != ebpf::EXIT {
                    write(&mut out, *pc, insn(ebpf::EXIT, 0, 0, 0, 0));
                    stats.unreachable_instructions_neutralized += 1;
                    if old.opc == ebpf::LD_DW_IMM && !m.protected.contains(&(pc + 1)) {
                        write(&mut out, pc + 1, insn(ebpf::EXIT, 0, 0, 0, 0));
                    }
                }
            }
        }
        stats.unreachable_functions_neutralized = m
            .functions
            .iter()
            .filter(|pc| {
                !live.contains(pc)
                    && ebpf::get_insn(text, **pc).opc != ebpf::EXIT
                    && ebpf::get_insn(&out, **pc).opc == ebpf::EXIT
            })
            .count();
    }
    let incoming = entry_targets(&out, m);
    if indirect == 0 {
        for pc in starts(&out) {
            if pc + 1 >= n || incoming.contains(&(pc + 1)) || m.protected.contains(&(pc + 1)) {
                continue;
            }
            let store = ebpf::get_insn(&out, pc);
            let load = ebpf::get_insn(&out, pc + 1);
            if store.opc == ebpf::ST_DW_REG
                && load.opc == ebpf::LD_DW_REG
                && store.dst == 10
                && load.src == 10
                && store.off == load.off
            {
                write(
                    &mut out,
                    pc + 1,
                    insn(ebpf::MOV64_REG, load.dst, store.src, 0, 0),
                );
                stats.stack_loads_forwarded += 1;
            }
        }
    }
    for pc in starts(&out) {
        if m.protected.contains(&pc) {
            continue;
        }
        let i = ebpf::get_insn(&out, pc);
        if let Some(replacement) = strength_reduce(&i) {
            write(&mut out, pc, replacement);
            stats.arithmetic_strength_reductions += 1;
        } else if let Some(taken) = constant_branch(&i) {
            write(
                &mut out,
                pc,
                insn(ebpf::JA, 0, 0, if taken { i.off } else { 0 }, 0),
            );
            stats.constant_branches_simplified += 1;
        }
    }
    for pc in starts(&out) {
        if m.protected.contains(&pc) {
            continue;
        }
        let mut i = ebpf::get_insn(&out, pc);
        if is_jump(&i) {
            let old = target(pc, i64::from(i.off), n).context("branch target out of .text")?;
            let next = thread_target(&out, old);
            let delta = next as i64 - pc as i64 - 1;
            if old != next && i16::try_from(delta).is_ok() && next != pc {
                i.off = delta as i16;
                write(&mut out, pc, i);
                stats.branch_targets_threaded += 1;
            }
        } else if i.opc == ebpf::CALL_IMM && i.imm != -1 {
            let old = target(pc, i.imm, n).context("call target out of .text")?;
            let next = thread_target(&out, old);
            // -1 is the ELF loader's unresolved-symbol sentinel, not a relative self-call.
            if old != next && next != pc {
                i.imm = next as i64 - pc as i64 - 1;
                write(&mut out, pc, i);
                stats.call_targets_threaded += 1;
            }
        }
    }
    let mut pc = 0;
    while pc < n {
        let original = ebpf::get_insn(&out, pc);
        if m.protected.contains(&pc) || !identity(&original) {
            pc += if original.opc == ebpf::LD_DW_IMM {
                2
            } else {
                1
            };
            continue;
        }
        let mut end = pc + 1;
        while end < n
            && end - pc <= i16::MAX as usize
            && !m.protected.contains(&end)
            && identity(&ebpf::get_insn(&out, end))
        {
            end += 1;
        }
        if end - pc >= 2 && end < n {
            write(&mut out, pc, insn(ebpf::JA, 0, 0, (end - pc - 1) as i16, 0));
            stats.identity_runs_skipped += 1;
            stats.identity_instructions_bypassed += end - pc - 1;
        }
        pc = end;
    }
    RequisiteVerifier::verify(&out, &Config::default(), SBPFVersion::V0)
        .context("rewrite generated invalid SBF-v0 instructions")?;
    stats.modified_pcs = (0..n)
        .filter(|pc| text[pc * 8..pc * 8 + 8] != out[pc * 8..pc * 8 + 8])
        .collect();
    Ok((out, stats))
}
fn strength_reduce(i: &ebpf::Insn) -> Option<ebpf::Insn> {
    let value = u32::try_from(i.imm).ok()?;
    if value == 0 || !value.is_power_of_two() || value > (1 << 30) {
        return None;
    }
    let shift = i64::from(value.trailing_zeros());
    let (opcode, immediate) = match i.opc {
        ebpf::MUL64_IMM => (ebpf::LSH64_IMM, shift),
        ebpf::DIV64_IMM => (ebpf::RSH64_IMM, shift),
        ebpf::MOD64_IMM => (ebpf::AND64_IMM, i.imm - 1),
        ebpf::DIV32_IMM => (ebpf::RSH32_IMM, shift),
        ebpf::MOD32_IMM => (ebpf::AND32_IMM, i.imm - 1),
        _ => return None,
    };
    Some(insn(opcode, i.dst, 0, 0, immediate))
}
fn constant_branch(i: &ebpf::Insn) -> Option<bool> {
    if i.opc == ebpf::JSET64_IMM && i.imm == 0 {
        return Some(false);
    }
    if i.dst != i.src {
        return None;
    }
    match i.opc {
        ebpf::JEQ64_REG
        | ebpf::JGE64_REG
        | ebpf::JLE64_REG
        | ebpf::JSGE64_REG
        | ebpf::JSLE64_REG => Some(true),
        ebpf::JNE64_REG
        | ebpf::JGT64_REG
        | ebpf::JLT64_REG
        | ebpf::JSGT64_REG
        | ebpf::JSLT64_REG => Some(false),
        _ => None,
    }
}
fn identity(i: &ebpf::Insn) -> bool {
    match i.opc {
        ebpf::MOV64_REG => i.dst == i.src,
        ebpf::ADD64_IMM
        | ebpf::SUB64_IMM
        | ebpf::OR64_IMM
        | ebpf::XOR64_IMM
        | ebpf::LSH64_IMM
        | ebpf::RSH64_IMM
        | ebpf::ARSH64_IMM => i.imm == 0,
        ebpf::MUL64_IMM | ebpf::DIV64_IMM => i.imm == 1,
        ebpf::AND64_IMM => i.imm == -1,
        _ => false,
    }
}
fn thread_target(text: &[u8], start: usize) -> usize {
    let mut seen = BTreeSet::new();
    let mut pc = start;
    while seen.insert(pc) {
        let i = ebpf::get_insn(text, pc);
        if i.opc != ebpf::JA {
            return pc;
        }
        let Some(next) = target(pc, i64::from(i.off), text.len() / 8) else {
            return start;
        };
        pc = next;
    }
    start
}
fn entry_targets(text: &[u8], m: &Metadata) -> BTreeSet<usize> {
    let mut out = m.roots.clone();
    out.extend(m.functions.iter().copied());
    out.extend(m.calls.values().copied());
    for pc in starts(text) {
        let i = ebpf::get_insn(text, pc);
        let delta = if is_jump(&i) {
            Some(i64::from(i.off))
        } else if i.opc == ebpf::CALL_IMM && i.imm != -1 {
            Some(i.imm)
        } else {
            None
        };
        if let Some(to) = delta.and_then(|d| target(pc, d, text.len() / 8)) {
            out.insert(to);
        }
    }
    out
}
fn reachability(text: &[u8], m: &Metadata, indirect: bool) -> BTreeSet<usize> {
    if indirect {
        return starts(text).into_iter().collect();
    }
    let n = text.len() / 8;
    let mut seen = BTreeSet::new();
    let mut work: VecDeque<_> = m.roots.iter().copied().collect();
    while let Some(pc) = work.pop_front() {
        if pc >= n || !seen.insert(pc) {
            continue;
        }
        let i = ebpf::get_insn(text, pc);
        if i.opc == ebpf::EXIT {
            continue;
        }
        if is_jump(&i) {
            if let Some(to) = target(pc, i64::from(i.off), n) {
                work.push_back(to);
            }
            if i.opc != ebpf::JA {
                work.push_back(pc + 1);
            }
        } else {
            if i.opc == ebpf::CALL_IMM {
                if let Some(to) = m.calls.get(&pc).copied().or_else(|| {
                    if i.imm != -1 {
                        target(pc, i.imm, n)
                    } else {
                        None
                    }
                }) {
                    work.push_back(to);
                }
            }
            work.push_back(pc + if i.opc == ebpf::LD_DW_IMM { 2 } else { 1 });
        }
    }
    seen
}

#[cfg(test)]
mod tests;
