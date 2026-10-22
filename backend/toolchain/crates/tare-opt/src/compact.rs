//! Relocation-aware ELF64/SBF-v0 layout. All file and virtual addresses are rebuilt.
use super::*;

#[derive(Clone)]
struct Header {
    words: [u64; 10],
    name: String,
    old: Range<usize>,
    bytes: Vec<u8>,
    offset: usize,
}
impl Header {
    fn kind(&self) -> u32 {
        self.words[1] as u32
    }
    fn alloc(&self) -> bool {
        self.words[2] & 2 != 0
    }
    fn addr(&self) -> u64 {
        self.words[3]
    }
    fn link(&self) -> usize {
        self.words[6] as usize
    }
}
#[derive(Clone)]
struct Segment {
    values: [u64; 8],
}
struct Image {
    header: Vec<u8>,
    sections: Vec<Header>,
    segments: Vec<Segment>,
    text: usize,
    entry: u64,
}
pub(super) fn put16(b: &mut [u8], at: usize, v: u16) {
    b[at..at + 2].copy_from_slice(&v.to_le_bytes());
}
pub(super) fn put32(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_le_bytes());
}
pub(super) fn put64(b: &mut [u8], at: usize, v: u64) {
    b[at..at + 8].copy_from_slice(&v.to_le_bytes());
}
fn align(value: usize, alignment: usize) -> Result<usize> {
    let alignment = alignment.max(1);
    ensure!(
        alignment.is_power_of_two(),
        "ELF section alignment is not a power of two"
    );
    Ok(value
        .checked_add(alignment - 1)
        .context("ELF layout overflow")?
        & !(alignment - 1))
}
impl Image {
    fn parse(bytes: &[u8]) -> Result<Self> {
        let view = ElfView::parse(bytes)?;
        let phoff = usize::try_from(u64_at(bytes, 32)?)?;
        let phnum = usize::from(u16_at(bytes, 56)?);
        ensure!(
            phnum == 0 || u16_at(bytes, 54)? == 56,
            "unsupported ELF program headers"
        );
        let mut segments = Vec::new();
        for i in 0..phnum {
            let p = phoff + i * 56;
            let mut values = [0; 8];
            values[0] = u32_at(bytes, p)? as u64;
            values[1] = u32_at(bytes, p + 4)? as u64;
            for j in 2..8 {
                values[j] = u64_at(bytes, p + 8 + (j - 2) * 8)?;
            }
            ensure!(
                values[5] == values[6],
                "SBF segment with uninitialized memory is not supported"
            );
            segments.push(Segment { values });
        }
        let shoff = usize::try_from(u64_at(bytes, 40)?)?;
        let mut sections = Vec::new();
        let mut text = None;
        for (i, section) in view.sections.iter().enumerate() {
            let p = shoff + i * 64;
            let mut words = [0; 10];
            words[0] = u32_at(bytes, p)? as u64;
            words[1] = u32_at(bytes, p + 4)? as u64;
            for j in 2..6 {
                words[j] = u64_at(bytes, p + 8 + (j - 2) * 8)?;
            }
            words[6] = u32_at(bytes, p + 40)? as u64;
            words[7] = u32_at(bytes, p + 44)? as u64;
            words[8] = u64_at(bytes, p + 48)?;
            words[9] = u64_at(bytes, p + 56)?;
            ensure!(
                section.kind != 8 || words[5] == 0,
                "nonempty SBF NOBITS section is unsupported"
            );
            ensure!(
                section.name != ".eh_frame" || words[5] == 0,
                "ELF compaction does not implement .eh_frame pointer encodings"
            );
            if section.name == ".text" {
                text = Some(i);
            }
            if words[2] & 2 != 0 {
                ensure!(
                    words[3] == words[4],
                    "SBF-v0 allocated section address must equal its file offset"
                );
            }
            sections.push(Header {
                words,
                name: section.name.clone(),
                old: section.range.clone(),
                bytes: bytes[section.range.clone()].to_vec(),
                offset: 0,
            });
        }
        Ok(Self {
            header: bytes[..64].to_vec(),
            sections,
            segments,
            text: text.context("missing .text")?,
            entry: view.entry,
        })
    }
    fn section_for_address(&self, address: u64) -> Option<(usize, usize, bool)> {
        let based = address >= ebpf::MM_BYTECODE_START;
        let address = if based {
            address - ebpf::MM_BYTECODE_START
        } else {
            address
        };
        self.sections.iter().enumerate().find_map(|(i, s)| {
            (s.alloc() && address >= s.addr() && address < s.addr() + s.old.len() as u64)
                .then_some((i, address.saturating_sub(s.addr()) as usize, based))
        })
    }
    fn address(&self, address: u64, map: &[Option<usize>]) -> Result<u64> {
        let Some((index, offset, based)) = self.section_for_address(address) else {
            return Ok(address);
        };
        let mapped = if index == self.text {
            let pc = offset / 8;
            let inside = offset % 8;
            map.get(pc)
                .and_then(|x| *x)
                .context("live reference points into removed SBF text")?
                * 8
                + inside
        } else {
            offset
        };
        Ok(self.sections[index].offset as u64
            + mapped as u64
            + if based { ebpf::MM_BYTECODE_START } else { 0 })
    }
    fn location(&self, old: usize, map: &[Option<usize>]) -> Option<(usize, usize)> {
        self.sections.iter().enumerate().find_map(|(i, s)| {
            if !s.old.contains(&old) {
                return None;
            }
            let offset = old - s.old.start;
            if i == self.text {
                map.get(offset / 8)
                    .and_then(|x| *x)
                    .map(|pc| (i, pc * 8 + offset % 8))
            } else {
                Some((i, offset))
            }
        })
    }
    fn old_relocations(&self) -> Result<Vec<(usize, usize, u64, u64)>> {
        let mut result = Vec::new();
        for (index, s) in self.sections.iter().enumerate() {
            if s.kind() != 9 {
                continue;
            }
            ensure!(
                s.words[9] == 16 && s.bytes.len() % 16 == 0,
                "invalid ELF relocation section"
            );
            for p in (0..s.bytes.len()).step_by(16) {
                result.push((
                    index,
                    s.link(),
                    u64_at(&s.bytes, p)?,
                    u64_at(&s.bytes, p + 8)?,
                ));
            }
        }
        Ok(result)
    }
    fn layout(&mut self) -> Result<()> {
        let mut offset = 64 + self.segments.len() * 56;
        for (i, s) in self.sections.iter_mut().enumerate() {
            if i == 0 {
                s.offset = 0;
                continue;
            }
            offset = align(offset, usize::try_from(s.words[8])?)?;
            s.offset = offset;
            offset = offset
                .checked_add(s.bytes.len())
                .context("ELF layout overflow")?;
        }
        Ok(())
    }
    fn emit(&self, map: &[Option<usize>]) -> Result<Vec<u8>> {
        let end = self
            .sections
            .iter()
            .map(|s| s.offset + s.bytes.len())
            .max()
            .unwrap_or(64);
        let shoff = align(end, 8)?;
        let mut out = vec![0; shoff + self.sections.len() * 64];
        out[..64].copy_from_slice(&self.header);
        put64(&mut out, 24, self.address(self.entry, map)?);
        put64(&mut out, 32, 64);
        put64(&mut out, 40, shoff as u64);
        put16(&mut out, 54, 56);
        put16(&mut out, 56, self.segments.len() as u16);
        for (i, seg) in self.segments.iter().enumerate() {
            let mut values = seg.values;
            if values[5] != 0 {
                let old_start = values[2] as usize;
                let old_end = old_start + values[5] as usize;
                let covered: Vec<_> = self
                    .sections
                    .iter()
                    .filter(|s| {
                        s.alloc()
                            && !s.old.is_empty()
                            && s.old.start >= old_start
                            && s.old.end <= old_end
                    })
                    .collect();
                ensure!(
                    !covered.is_empty(),
                    "ELF segment does not cover complete allocated sections"
                );
                let start = covered.first().unwrap().offset;
                let end = covered.last().unwrap().offset + covered.last().unwrap().bytes.len();
                values[2] = start as u64;
                values[3] = start as u64;
                values[4] = start as u64;
                values[5] = (end - start) as u64;
                values[6] = values[5];
            }
            let at = 64 + i * 56;
            put32(&mut out, at, values[0] as u32);
            put32(&mut out, at + 4, values[1] as u32);
            for j in 2..8 {
                put64(&mut out, at + 8 + (j - 2) * 8, values[j]);
            }
        }
        for (i, s) in self.sections.iter().enumerate() {
            out[s.offset..s.offset + s.bytes.len()].copy_from_slice(&s.bytes);
            let at = shoff + i * 64;
            let mut w = s.words;
            w[4] = s.offset as u64;
            w[5] = s.bytes.len() as u64;
            if s.alloc() {
                w[3] = s.offset as u64;
            }
            put32(&mut out, at, w[0] as u32);
            put32(&mut out, at + 4, w[1] as u32);
            for j in 2..6 {
                put64(&mut out, at + 8 + (j - 2) * 8, w[j]);
            }
            put32(&mut out, at + 40, w[6] as u32);
            put32(&mut out, at + 44, w[7] as u32);
            put64(&mut out, at + 48, w[8]);
            put64(&mut out, at + 56, w[9]);
        }
        Ok(out)
    }
}

pub(super) fn compact(bytes: &[u8], stats: &mut Stats) -> Result<Vec<u8>> {
    let mut image = Image::parse(bytes)?;
    let view = ElfView::parse(bytes)?;
    let metadata = view.metadata(bytes)?;
    let text = image.sections[image.text].bytes.clone();
    let positions = starts(&text);
    let slots = text.len() / 8;
    let analysis = indirect::analyze(&view, bytes, &metadata)?;
    stats.indirect_call_sites = analysis
        .sites
        .iter()
        .map(|(&pc, targets)| IndirectCallSite {
            pc,
            targets: targets
                .as_ref()
                .map(|v| v.iter().copied().collect())
                .unwrap_or_default(),
            resolved: targets.is_some(),
            reason: targets
                .is_none()
                .then(|| "Target depends on unknown input, memory, or call result".to_owned()),
        })
        .collect();
    stats.unresolved_indirect_calls = stats
        .indirect_call_sites
        .iter()
        .filter(|s| !s.resolved)
        .count();
    if analysis.limited {
        stats.indirect_call_sites.push(IndirectCallSite {
            pc: analysis.limit_pc.unwrap_or(0),
            targets: Vec::new(),
            resolved: false,
            reason: Some(format!(
                "{} requires conservative retention",
                analysis.limit_reason.unwrap_or("incomplete analysis")
            )),
        });
        stats.unresolved_indirect_calls += 1;
    }
    let live = if analysis.unresolved() {
        // v0 callx accepts every in-range instruction, including non-symbol
        // entries. Never shrink text while an executed call target is unknown.
        stats.retained_for_indirect_calls = positions.len().saturating_sub(analysis.live.len());
        positions.iter().copied().collect()
    } else {
        analysis.live.clone()
    };
    let mut keep = vec![false; slots];
    for &pc in &live {
        keep[pc] = true;
        if ebpf::get_insn(&text, pc).opc == ebpf::LD_DW_IMM {
            keep[pc + 1] = true;
        }
    }
    let relocations = image.old_relocations()?;
    let mut relocation_keys = BTreeMap::new();
    for (_section, symbols, site, info) in &relocations {
        if !view.text.contains(&(*site as usize)) {
            continue;
        }
        let pc = (*site as usize - view.text.start) / 8;
        let kind = *info as u32;
        let mut key = kind.to_le_bytes().to_vec();
        let symbol_index = (*info >> 32) as usize;
        match kind {
            8 => {
                let address = u32_at(bytes, *site as usize + 4)? as u64
                    | ((u32_at(bytes, *site as usize + 12)? as u64) << 32);
                key.extend_from_slice(&address.to_le_bytes());
            }
            1 => {
                let symbol = u64_at(&image.sections[*symbols].bytes, symbol_index * 24 + 8)?;
                let address = symbol
                    .checked_add(u32_at(bytes, *site as usize + 4)? as u64)
                    .context("symbol address overflow")?;
                key.extend_from_slice(&address.to_le_bytes());
            }
            10 => {
                let table = &image.sections[*symbols];
                let symbol = symbol_index * 24;
                let value = u64_at(&table.bytes, symbol + 8)?;
                if table.bytes[symbol + 4] & 15 == 2 && value != 0 {
                    key.extend_from_slice(&value.to_le_bytes());
                } else {
                    let strings = &image.sections[table.link()].bytes;
                    let name = u32_at(&table.bytes, symbol)? as usize;
                    let tail = strings.get(name..).context("invalid symbol name")?;
                    let len = tail
                        .iter()
                        .position(|b| *b == 0)
                        .context("unterminated symbol name")?;
                    key.extend_from_slice(&tail[..len]);
                }
            }
            _ => anyhow::bail!("ELF compaction does not implement relocation type {kind}"),
        }
        relocation_keys.insert(pc, key);
    }
    let aliases = if analysis.unresolved() {
        BTreeMap::new()
    } else {
        merge_functions(&text, &metadata, &keep, &relocation_keys)?
    };
    for (&from, &to) in &aliases {
        if from != to {
            keep[from] = false;
        }
    }
    let mut map = vec![None; slots];
    let mut count = 0;
    for pc in 0..slots {
        if keep[pc] {
            map[pc] = Some(count);
            count += 1;
        }
    }
    for (&from, &to) in &aliases {
        map[from] = map[to];
    }
    let mut compacted = Vec::with_capacity(count * 8);
    for pc in 0..slots {
        if keep[pc] {
            compacted.extend_from_slice(&text[pc * 8..pc * 8 + 8]);
        }
    }
    for &pc in &positions {
        if !keep[pc] {
            continue;
        }
        let new_pc = map[pc].unwrap();
        let mut i = ebpf::get_insn(&text, pc);
        if is_jump(&i) {
            let old =
                target(pc, i.off as i64, slots).context("invalid branch during compaction")?;
            let next = map[old].context("branch targets deleted instruction")?;
            i.off = i16::try_from(next as i64 - new_pc as i64 - 1)
                .context("compacted branch exceeds sBPF displacement")?;
        } else if i.opc == ebpf::CALL_IMM && i.imm != -1 && !metadata.protected.contains(&pc) {
            let old = target(pc, i.imm, slots).context("invalid call during compaction")?;
            let next = map[old].context("call targets deleted instruction")?;
            i.imm = next as i64 - new_pc as i64 - 1;
            ensure!(
                i.imm != -1,
                "compaction would create the unresolved-call sentinel"
            );
        }
        write(&mut compacted, new_pc, i);
    }
    image.sections[image.text].bytes = compacted;
    let mut kept_relocs: BTreeMap<usize, Vec<(usize, usize, u64)>> = BTreeMap::new();
    for (section, _symbols, site, info) in &relocations {
        if view.text.contains(&(*site as usize)) && !keep[(*site as usize - view.text.start) / 8] {
            stats.relocations_removed += 1;
            continue;
        }
        if let Some((target, offset)) = image.location(*site as usize, &map) {
            if matches!(*info as u32, 1 | 8) {
                stats.relocations_prelinked += 1;
            } else {
                kept_relocs
                    .entry(*section)
                    .or_default()
                    .push((target, offset, *info));
            }
        } else {
            stats.relocations_removed += 1;
        }
    }
    for (index, section) in image.sections.iter_mut().enumerate() {
        if section.kind() == 9 {
            section
                .bytes
                .resize(kept_relocs.get(&index).map_or(0, |v| v.len() * 16), 0);
        }
        if section.kind() == 1
            && !section.alloc()
            && (section.name.starts_with(".debug") || section.name == ".comment")
        {
            stats.non_allocated_bytes_removed += section.bytes.len();
            section.bytes.clear();
        }
    }
    image.layout()?;
    let mut updated_symbols = BTreeMap::new();
    for (index, section) in image.sections.iter().enumerate() {
        if ![2, 11].contains(&section.kind()) {
            continue;
        }
        ensure!(
            section.words[9] == 24 && section.bytes.len() % 24 == 0,
            "invalid ELF symbols"
        );
        let mut data = section.bytes.clone();
        for at in (0..data.len()).step_by(24) {
            let value = u64_at(&section.bytes, at + 8)?;
            let size = u64_at(&section.bytes, at + 16)?;
            let defining = u16_at(&section.bytes, at + 6)? as usize;
            if defining == 0 || defining >= image.sections.len() {
                continue;
            }
            if defining == image.text
                && image
                    .section_for_address(value)
                    .and_then(|(_, off, _)| map.get(off / 8).and_then(|m| *m))
                    .is_none()
            {
                data[at..at + 24].fill(0);
                continue;
            }
            put64(&mut data, at + 8, image.address(value, &map)?);
            if defining == image.text && size > 0 {
                let start = value
                    .checked_sub(image.sections[image.text].addr())
                    .context("text symbol outside section")? as usize
                    / 8;
                let end = (start + size as usize / 8).min(slots);
                let new_size = (start..end).filter(|pc| map[*pc].is_some()).count() * 8;
                put64(&mut data, at + 16, new_size as u64);
            }
        }
        updated_symbols.insert(index, data);
    }
    // Only relocation records identify image-relative pointer operands. An
    // ordinary numeric immediate may coincide with an allocated address range
    // (for example a wide-division comparison threshold) and stays unchanged.
    for (_section, symbols, site, info) in &relocations {
        if view.text.contains(&(*site as usize)) && !keep[(*site as usize - view.text.start) / 8] {
            continue;
        }
        let Some((target_section, offset)) = image.location(*site as usize, &map) else {
            continue;
        };
        match *info as u32 {
            8 => {
                let address = if target_section == image.text {
                    u32_at(bytes, *site as usize + 4)? as u64
                        | ((u32_at(bytes, *site as usize + 12)? as u64) << 32)
                } else {
                    u32_at(bytes, *site as usize + 4)? as u64
                };
                let absolute = absolute_v0(image.address(address, &map)?)?;
                if target_section == image.text {
                    put32(
                        &mut image.sections[target_section].bytes,
                        offset + 4,
                        absolute as u32,
                    );
                    put32(
                        &mut image.sections[target_section].bytes,
                        offset + 12,
                        (absolute >> 32) as u32,
                    );
                } else {
                    put64(&mut image.sections[target_section].bytes, offset, absolute);
                }
            }
            1 => {
                ensure!(
                    target_section == image.text,
                    "R_BPF_64_64 outside text is unsupported"
                );
                let symbol = u64_at(
                    &image.sections[*symbols].bytes,
                    (*info >> 32) as usize * 24 + 8,
                )?;
                let address = symbol
                    .checked_add(u32_at(bytes, *site as usize + 4)? as u64)
                    .context("symbol address overflow")?;
                let absolute = absolute_v0(image.address(address, &map)?)?;
                put32(
                    &mut image.sections[target_section].bytes,
                    offset + 4,
                    absolute as u32,
                );
                put32(
                    &mut image.sections[target_section].bytes,
                    offset + 12,
                    (absolute >> 32) as u32,
                );
            }
            10 => {}
            kind => anyhow::bail!("ELF compaction does not implement relocation type {kind}"),
        }
    }
    for (index, data) in updated_symbols {
        image.sections[index].bytes = data;
    }
    for (index, relocs) in kept_relocs {
        let mut data = Vec::with_capacity(relocs.len() * 16);
        for (section, offset, info) in relocs {
            data.extend_from_slice(
                &((image.sections[section].offset + offset) as u64).to_le_bytes(),
            );
            data.extend_from_slice(&info.to_le_bytes());
        }
        image.sections[index].bytes = data;
    }
    let mut dynamics = Vec::new();
    for (index, section) in image.sections.iter().enumerate() {
        if section.kind() != 6 {
            continue;
        }
        ensure!(section.bytes.len() % 16 == 0, "invalid ELF dynamic table");
        let mut data = section.bytes.clone();
        for at in (0..data.len()).step_by(16) {
            let tag = u64_at(&section.bytes, at)?;
            let value = u64_at(&section.bytes, at + 8)?;
            let changed = match tag {
                0 | 11 | 19 | 22 | 30 => value,
                5 | 6 => image.address(value, &map)?,
                17 => {
                    if image
                        .sections
                        .iter()
                        .any(|s| s.kind() == 9 && !s.bytes.is_empty())
                    {
                        image.address(value, &map)?
                    } else {
                        0
                    }
                }
                10 => image
                    .sections
                    .get(section.link())
                    .map_or(value, |s| s.bytes.len() as u64),
                18 => image
                    .sections
                    .iter()
                    .find(|s| s.kind() == 9)
                    .map_or(0, |s| s.bytes.len() as u64),
                0x6fff_fffa => 0,
                _ => anyhow::bail!("ELF compaction does not implement dynamic tag {tag:#x}"),
            };
            put64(&mut data, at + 8, changed);
        }
        dynamics.push((index, data));
    }
    for (index, data) in dynamics {
        image.sections[index].bytes = data;
    }
    stats.text_slots_removed = slots - count;
    stats.text_bytes_removed = (slots - count) * 8;
    for (old, new) in map
        .iter()
        .enumerate()
        .filter_map(|(old, new)| new.map(|new| (old, new)))
    {
        if let Some(last) = stats.pc_map.last_mut() {
            if last[0] + last[2] == old && last[1] + last[2] == new {
                last[2] += 1;
                continue;
            }
        }
        stats.pc_map.push([old, new, 1]);
    }
    let mut functions = metadata.functions.clone();
    for &pc in &positions {
        let i = ebpf::get_insn(&text, pc);
        if i.opc == ebpf::CALL_IMM && i.imm != -1 {
            functions.insert(target(pc, i.imm, slots).context("invalid call")?);
        }
    }
    stats.identical_functions_merged = functions
        .iter()
        .filter(|pc| aliases.contains_key(pc))
        .count();
    image.emit(&map)
}
fn absolute_v0(address: u64) -> Result<u64> {
    if address < ebpf::MM_BYTECODE_START {
        address
            .checked_add(ebpf::MM_BYTECODE_START)
            .context("prelinked address overflow")
    } else {
        Ok(address)
    }
}

fn merge_functions(
    text: &[u8],
    metadata: &Metadata,
    keep: &[bool],
    relocations: &BTreeMap<usize, Vec<u8>>,
) -> Result<BTreeMap<usize, usize>> {
    let slots = text.len() / 8;
    let mut entries = metadata.functions.clone();
    entries.extend(metadata.roots.iter().copied());
    for pc in starts(text) {
        let i = ebpf::get_insn(text, pc);
        if i.opc == ebpf::CALL_IMM && i.imm != -1 {
            entries.insert(target(pc, i.imm, slots).context("invalid call")?);
        }
    }
    let entries: Vec<_> = entries.into_iter().filter(|pc| *pc < slots).collect();
    let mut aliases = BTreeMap::new();
    let mut known: BTreeMap<Vec<u8>, Vec<usize>> = BTreeMap::new();
    for (position, &start) in entries.iter().enumerate() {
        let end = entries.get(position + 1).copied().unwrap_or(slots);
        if metadata.roots.contains(&start) || !keep[start] {
            continue;
        }
        let pcs: Vec<_> = starts(&text[start * 8..end * 8])
            .into_iter()
            .map(|pc| pc + start)
            .filter(|pc| keep[*pc])
            .collect();
        let body: Vec<_> = (start..end).filter(|pc| keep[*pc]).collect();
        if pcs.is_empty() {
            continue;
        }
        let mut key = Vec::new();
        let mut safe = true;
        for &pc in &pcs {
            let mut i = ebpf::get_insn(text, pc);
            let rel = relocations.get(&pc);
            if is_jump(&i) {
                let to = target(pc, i.off as i64, slots).context("invalid branch")?;
                if to < start || to >= end {
                    safe = false;
                    break;
                }
                let from = body.binary_search(&pc).unwrap();
                let target = body
                    .binary_search(&to)
                    .map_err(|_| anyhow::anyhow!("branch targets dead instruction"))?;
                i.off = i16::try_from(target as i64 - from as i64 - 1)?;
            }
            if rel.is_some() {
                i.imm = 0;
            } else if i.opc == ebpf::CALL_IMM && i.imm != -1 {
                let to = target(pc, i.imm, slots).context("invalid call")?;
                i.imm = 0;
                key.extend_from_slice(
                    &(if to == start {
                        usize::MAX
                    } else {
                        *aliases.get(&to).unwrap_or(&to)
                    })
                    .to_le_bytes(),
                );
            }
            key.extend_from_slice(&i.to_array());
            if let Some(rel) = rel {
                key.extend_from_slice(&(rel.len() as u64).to_le_bytes());
                key.extend_from_slice(rel);
            } else {
                key.extend_from_slice(&0u64.to_le_bytes());
            }
            if i.opc == ebpf::LD_DW_IMM {
                let mut high = ebpf::get_insn(text, pc + 1);
                if rel.is_some() {
                    high.imm = 0;
                }
                key.extend_from_slice(&high.to_array());
            }
        }
        if !safe || ebpf::get_insn(text, *pcs.last().unwrap()).opc != ebpf::EXIT {
            continue;
        }
        if let Some(canonical) = known.get(&key) {
            if canonical.len() != body.len() {
                continue;
            }
            for (&old, &new) in body.iter().zip(canonical) {
                aliases.insert(old, new);
            }
        } else {
            known.insert(key, body);
        }
    }
    Ok(aliases)
}
