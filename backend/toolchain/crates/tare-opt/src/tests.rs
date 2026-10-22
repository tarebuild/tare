use super::*;
use solana_sbpf::{
    aligned_memory::AlignedMemory,
    elf::Executable,
    memory_region::{MemoryMapping, MemoryRegion},
    program::{BuiltinProgram, FunctionRegistry},
    vm::{CallFrame, ContextObject, EbpfVm, ExecutionMode},
};
use std::{ptr::NonNull, sync::Arc};

struct State {
    remaining: u64,
    mapping: MemoryMapping,
}
impl ContextObject for State {
    fn consume(&mut self, amount: u64) {
        self.remaining = self.remaining.saturating_sub(amount);
    }
    fn get_remaining(&self) -> u64 {
        self.remaining
    }
    fn active_mapping_ptr(&mut self) -> NonNull<MemoryMapping> {
        NonNull::from(&mut self.mapping)
    }
}
#[derive(Debug, PartialEq)]
struct Snapshot {
    registers: [u64; 12],
    input: Vec<u8>,
    stack: Vec<u8>,
    heap: Vec<u8>,
    result: String,
}
fn encode(instructions: &[ebpf::Insn]) -> Vec<u8> {
    instructions.iter().flat_map(|i| i.to_array()).collect()
}
fn metadata() -> Metadata {
    Metadata {
        roots: BTreeSet::from([0]),
        ..Metadata::default()
    }
}

// Use the production interpreter, not an implementation of the rewrite's algebra.
// Raw linked CALL immediates are resolved exactly as the v0 ELF loader resolves them.
fn execute(text: &[u8], seed: u64) -> (Snapshot, u64) {
    let config = Config {
        enable_register_tracing: true,
        ..Config::default()
    };
    let loader = Arc::new(BuiltinProgram::new_loader(config.clone()));
    let mut registry = FunctionRegistry::default();
    let mut relocated = text.to_vec();
    for pc in starts(text) {
        let mut i = ebpf::get_insn(text, pc);
        if i.opc == ebpf::CALL_IMM && i.imm != -1 {
            let destination = target(pc, i.imm, text.len() / 8).unwrap();
            let hash = ebpf::hash_symbol_name(&destination.to_le_bytes());
            registry
                .register_function(hash, format!("fn_{destination}"), destination)
                .unwrap();
            i.imm = i64::from(hash);
            write(&mut relocated, pc, i);
        }
    }
    let executable =
        Executable::<State>::from_text_bytes(&relocated, loader.clone(), SBPFVersion::V0, registry)
            .unwrap();
    executable.verify::<RequisiteVerifier>().unwrap();
    execute_loaded(executable, seed)
}
fn execute_loaded(executable: Executable<State>, seed: u64) -> (Snapshot, u64) {
    let loader = executable.get_loader().clone();
    let config = loader.get_config().clone();
    let mut stack = AlignedMemory::<{ ebpf::HOST_ALIGN }>::zero_filled(config.stack_size());
    let mut heap = vec![seed as u8; 64];
    let mut input: Vec<u8> = (0..128)
        .map(|i| (seed.wrapping_add(i * 37) & 255) as u8)
        .collect();
    for (i, b) in stack.as_slice_mut().iter_mut().enumerate() {
        *b = (seed.wrapping_add(i as u64 * 13) & 255) as u8;
    }
    let stack_len = stack.len();
    let regions = vec![
        executable.get_ro_region(),
        MemoryRegion::new_gapped(
            &mut stack,
            ebpf::MM_STACK_START,
            config.stack_frame_size as u64,
        ),
        MemoryRegion::new(
            std::ptr::slice_from_raw_parts_mut(heap.as_mut_ptr(), heap.len()),
            ebpf::MM_HEAP_START,
        ),
        MemoryRegion::new(
            std::ptr::slice_from_raw_parts_mut(input.as_mut_ptr(), input.len()),
            ebpf::MM_INPUT_START,
        ),
    ];
    let mapping = unsafe { MemoryMapping::new(regions, &config, SBPFVersion::V0).unwrap() };
    let mut state = State {
        remaining: 1_000_000,
        mapping,
    };
    let mut vm = EbpfVm::new(loader, SBPFVersion::V0, &mut state, stack_len);
    let mut random = seed;
    for register in &mut vm.registers[..10] {
        random = random.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = random;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        *register = z ^ (z >> 31);
    }
    // Known edge values are deliberately present in the observed machine state.
    vm.registers[0] = seed;
    vm.registers[1] = ebpf::MM_INPUT_START;
    vm.registers[2] = seed;
    let mut frames = vec![CallFrame::default(); config.max_call_depth];
    let (count, result) =
        vm.execute_program(&executable, &mut ExecutionMode::Interpreted, &mut frames);
    let registers = *vm.register_trace.last().unwrap();
    let result = format!("{result:?}");
    drop(vm);
    (
        Snapshot {
            registers,
            input,
            stack: stack.as_slice().to_vec(),
            heap,
            result,
        },
        count,
    )
}
fn equivalent(before: &[u8], after: &[u8], must_save: bool) {
    let edges = [
        0,
        1,
        u64::MAX,
        1 << 63,
        u32::MAX as u64,
        0xffff_ffff_0000_0000,
    ];
    let mut savings = false;
    for seed in edges
        .into_iter()
        .chain((0..48).map(|n| n * 0x0123_4567_89ab_cdefu64))
    {
        let (old, old_count) = execute(before, seed);
        let (new, new_count) = execute(after, seed);
        if old != new {
            panic!("machine-state divergence for seed {seed}, first instruction {:?}: registers {:?} -> {:?}; input difference {:?}; stack difference {:?}; results {} -> {}",
                ebpf::get_insn(before,0),old.registers,new.registers,
                old.input.iter().zip(&new.input).position(|(a,b)|a!=b),
                old.stack.iter().zip(&new.stack).position(|(a,b)|a!=b),old.result,new.result);
        }
        assert!(
            new_count <= old_count,
            "rewrite increased instruction count"
        );
        savings |= new_count < old_count;
    }
    assert!(
        !must_save || savings,
        "rewrite did not avoid an executed instruction"
    );
}

#[test]
fn identity_runs_preserve_all_machine_state_and_skip_work() {
    let identities = [
        insn(ebpf::MOV64_REG, 2, 2, 0, 0),
        insn(ebpf::ADD64_IMM, 2, 0, 0, 0),
        insn(ebpf::SUB64_IMM, 2, 0, 0, 0),
        insn(ebpf::OR64_IMM, 2, 0, 0, 0),
        insn(ebpf::XOR64_IMM, 2, 0, 0, 0),
        insn(ebpf::LSH64_IMM, 2, 0, 0, 0),
        insn(ebpf::RSH64_IMM, 2, 0, 0, 0),
        insn(ebpf::ARSH64_IMM, 2, 0, 0, 0),
        insn(ebpf::MUL64_IMM, 2, 0, 0, 1),
        insn(ebpf::DIV64_IMM, 2, 0, 0, 1),
        insn(ebpf::AND64_IMM, 2, 0, 0, -1),
    ];
    for identity in identities {
        let before = encode(&[
            identity,
            insn(ebpf::MOV64_REG, 0, 0, 0, 0),
            insn(ebpf::ST_DW_REG, 1, 2, 0, 0),
            insn(ebpf::EXIT, 0, 0, 0, 0),
        ]);
        let (after, stats) = optimize_text(&before, &metadata()).unwrap();
        assert!(stats.identity_runs_skipped > 0);
        equivalent(&before, &after, true);
    }
}

#[test]
fn branch_threading_preserves_taken_and_untaken_paths() {
    let before = encode(&[
        insn(ebpf::JEQ64_IMM, 2, 0, 2, 0),
        insn(ebpf::ADD64_IMM, 0, 0, 0, 7),
        insn(ebpf::JA, 0, 0, 0, 0),
        insn(ebpf::JA, 0, 0, 1, 0),
        insn(ebpf::ADD64_IMM, 0, 0, 0, 123),
        insn(ebpf::ST_DW_REG, 1, 0, 0, 0),
        insn(ebpf::EXIT, 0, 0, 0, 0),
    ]);
    let (after, stats) = optimize_text(&before, &metadata()).unwrap();
    assert!(stats.branch_targets_threaded > 0);
    equivalent(&before, &after, true);
}

#[test]
fn call_trampolines_preserve_registers_memory_and_frame_return() {
    let before = encode(&[
        insn(ebpf::CALL_IMM, 0, 0, 0, 2),
        insn(ebpf::ST_DW_REG, 1, 0, 0, 0),
        insn(ebpf::EXIT, 0, 0, 0, 0),
        insn(ebpf::JA, 0, 0, 1, 0),
        insn(ebpf::ADD64_IMM, 0, 0, 0, 99),
        insn(ebpf::ADD64_IMM, 0, 0, 0, 7),
        insn(ebpf::EXIT, 0, 0, 0, 0),
    ]);
    let (after, stats) = optimize_text(&before, &metadata()).unwrap();
    assert_eq!(stats.call_targets_threaded, 1);
    equivalent(&before, &after, true);
}

#[test]
fn stack_reload_forwarding_preserves_full_width_and_faults() {
    for offset in [-8, -4096, 0] {
        let before = encode(&[
            insn(ebpf::ST_DW_REG, 10, 2, offset, 0),
            insn(ebpf::LD_DW_REG, 3, 10, offset, 0),
            insn(ebpf::ST_DW_REG, 1, 3, 0, 0),
            insn(ebpf::EXIT, 0, 0, 0, 0),
        ]);
        let (after, stats) = optimize_text(&before, &metadata()).unwrap();
        assert_eq!(stats.stack_loads_forwarded, 1);
        equivalent(&before, &after, false);
    }
}

#[test]
fn unreachable_code_is_neutralized_without_touching_live_results() {
    let before = encode(&[
        insn(ebpf::JA, 0, 0, 2, 0),
        insn(ebpf::MOV64_IMM, 0, 0, 0, 99),
        insn(ebpf::ST_DW_REG, 1, 0, 0, 0),
        insn(ebpf::ADD64_IMM, 0, 0, 0, 7),
        insn(ebpf::EXIT, 0, 0, 0, 0),
    ]);
    let mut roots = metadata();
    roots.functions.insert(1);
    let (after, stats) = optimize_text(&before, &roots).unwrap();
    assert_eq!(stats.unreachable_instructions_neutralized, 2);
    assert_eq!(stats.unreachable_functions_neutralized, 1);
    equivalent(&before, &after, false);
}

#[test]
fn indirect_calls_keep_every_possible_target_live() {
    let address = ebpf::MM_BYTECODE_START + 4 * 8;
    let before = encode(&[
        insn(ebpf::LD_DW_IMM, 3, 0, 0, address as u32 as i64),
        insn(0, 0, 0, 0, (address >> 32) as i64),
        insn(ebpf::CALL_REG, 0, 0, 0, 3),
        insn(ebpf::EXIT, 0, 0, 0, 0),
        insn(ebpf::ADD64_IMM, 0, 0, 0, 7),
        insn(ebpf::EXIT, 0, 0, 0, 0),
    ]);
    let (after, stats) = optimize_text(&before, &metadata()).unwrap();
    assert_eq!(stats.indirect_calls, 1);
    assert_eq!(stats.unreachable_instructions_neutralized, 0);
    equivalent(&before, &after, false);
}

#[test]
fn branch_entry_and_relocation_prevent_unsound_stack_forwarding() {
    let before = encode(&[
        insn(ebpf::ST_DW_REG, 10, 2, -8, 0),
        insn(ebpf::LD_DW_REG, 3, 10, -8, 0),
        insn(ebpf::ST_DW_REG, 1, 3, 0, 0),
        insn(ebpf::EXIT, 0, 0, 0, 0),
    ]);
    let mut m = metadata();
    m.protected.insert(1);
    let (after, stats) = optimize_text(&before, &m).unwrap();
    assert_eq!(stats.stack_loads_forwarded, 0);
    equivalent(&before, &after, false);
}

#[test]
fn alu32_zero_extension_is_never_treated_as_an_identity() {
    let before = encode(&[
        insn(ebpf::MOV32_REG, 2, 2, 0, 0),
        insn(ebpf::ADD32_IMM, 2, 0, 0, 0),
        insn(ebpf::ST_DW_REG, 1, 2, 0, 0),
        insn(ebpf::EXIT, 0, 0, 0, 0),
    ]);
    let (after, stats) = optimize_text(&before, &metadata()).unwrap();
    assert_eq!(stats.identity_runs_skipped, 0);
    equivalent(&before, &after, false);
}

#[test]
fn arithmetic_strength_reduction_preserves_wraparound_and_zero_extension() {
    for opcode in [
        ebpf::MUL64_IMM,
        ebpf::DIV64_IMM,
        ebpf::MOD64_IMM,
        ebpf::DIV32_IMM,
        ebpf::MOD32_IMM,
    ] {
        for value in [1, 2, 256, 1 << 30] {
            let before = encode(&[
                insn(opcode, 2, 0, 0, value),
                insn(ebpf::ST_DW_REG, 1, 2, 0, 0),
                insn(ebpf::EXIT, 0, 0, 0, 0),
            ]);
            let (after, stats) = optimize_text(&before, &metadata()).unwrap();
            assert_eq!(stats.arithmetic_strength_reductions, 1);
            equivalent(&before, &after, false);
        }
    }
}

#[test]
fn constant_register_branches_preserve_both_destinations() {
    for opcode in [
        ebpf::JEQ64_REG,
        ebpf::JGE64_REG,
        ebpf::JLE64_REG,
        ebpf::JSGE64_REG,
        ebpf::JSLE64_REG,
        ebpf::JNE64_REG,
        ebpf::JGT64_REG,
        ebpf::JLT64_REG,
        ebpf::JSGT64_REG,
        ebpf::JSLT64_REG,
        ebpf::JSET64_IMM,
    ] {
        let before = encode(&[
            insn(opcode, 2, 2, 1, 0),
            insn(ebpf::ADD64_IMM, 0, 0, 0, 17),
            insn(ebpf::ST_DW_REG, 1, 0, 0, 0),
            insn(ebpf::EXIT, 0, 0, 0, 0),
        ]);
        let (after, stats) = optimize_text(&before, &metadata()).unwrap();
        assert_eq!(stats.constant_branches_simplified, 1);
        equivalent(&before, &after, false);
    }
}

fn minimal_elf(text: &[u8]) -> Vec<u8> {
    let text_offset = 120usize;
    let names = b"\0.text\0.shstrtab\0";
    let names_offset = text_offset + text.len();
    let shoff = (names_offset + names.len() + 7) & !7;
    let mut b = vec![0; shoff + 3 * 64];
    b[..4].copy_from_slice(b"\x7fELF");
    b[4] = 2;
    b[5] = 1;
    b[6] = 1;
    let w16 = |b: &mut [u8], p, v: u16| b[p..p + 2].copy_from_slice(&v.to_le_bytes());
    let w32 = |b: &mut [u8], p, v: u32| b[p..p + 4].copy_from_slice(&v.to_le_bytes());
    let w64 = |b: &mut [u8], p, v: u64| b[p..p + 8].copy_from_slice(&v.to_le_bytes());
    w16(&mut b, 16, 3);
    w16(&mut b, 18, 247);
    w32(&mut b, 20, 1);
    w64(&mut b, 24, text_offset as u64);
    w64(&mut b, 32, 64);
    w64(&mut b, 40, shoff as u64);
    w16(&mut b, 52, 64);
    w16(&mut b, 54, 56);
    w16(&mut b, 56, 1);
    w16(&mut b, 58, 64);
    w16(&mut b, 60, 3);
    w16(&mut b, 62, 2);
    w32(&mut b, 64, 1);
    w32(&mut b, 68, 5);
    for p in [72, 80, 88] {
        w64(&mut b, p, text_offset as u64);
    }
    w64(&mut b, 96, text.len() as u64);
    w64(&mut b, 104, text.len() as u64);
    w64(&mut b, 112, 8);
    b[text_offset..names_offset].copy_from_slice(text);
    b[names_offset..names_offset + names.len()].copy_from_slice(names);
    let h = shoff + 64;
    w32(&mut b, h, 1);
    w32(&mut b, h + 4, 1);
    w64(&mut b, h + 8, 6);
    w64(&mut b, h + 16, text_offset as u64);
    w64(&mut b, h + 24, text_offset as u64);
    w64(&mut b, h + 32, text.len() as u64);
    w64(&mut b, h + 48, 8);
    let h = shoff + 128;
    w32(&mut b, h, 7);
    w32(&mut b, h + 4, 3);
    w64(&mut b, h + 24, names_offset as u64);
    w64(&mut b, h + 32, names.len() as u64);
    w64(&mut b, h + 48, 1);
    b
}

#[test]
fn rewritten_elf_is_admitted_by_real_agave_loader() {
    let text = encode(&[
        insn(ebpf::MOV64_REG, 0, 0, 0, 0),
        insn(ebpf::ADD64_IMM, 0, 0, 0, 0),
        insn(ebpf::EXIT, 0, 0, 0, 0),
    ]);
    let before = minimal_elf(&text);
    let optimized = optimize(&before).unwrap();
    assert!(optimized.stats.identity_runs_skipped > 0);
    assert!(optimized.bytes.len() < before.len());
    equivalent_elf(&before, &optimized);
}

#[test]
fn sbpf_v0_multiply32_sign_extension_is_preserved() {
    for value in [1, 2, 256] {
        let before = encode(&[
            insn(ebpf::MUL32_IMM, 2, 0, 0, value),
            insn(ebpf::ST_DW_REG, 1, 2, 0, 0),
            insn(ebpf::EXIT, 0, 0, 0, 0),
        ]);
        let (after, stats) = optimize_text(&before, &metadata()).unwrap();
        assert_eq!(stats.arithmetic_strength_reductions, 0);
        equivalent(&before, &after, false);
    }
}

#[test]
fn stack_reload_with_a_taken_branch_entry_is_retained() {
    let before = encode(&[
        insn(ebpf::JEQ64_IMM, 2, 0, 1, 0),
        insn(ebpf::ST_DW_REG, 10, 2, -8, 0),
        insn(ebpf::LD_DW_REG, 3, 10, -8, 0),
        insn(ebpf::ST_DW_REG, 1, 3, 0, 0),
        insn(ebpf::EXIT, 0, 0, 0, 0),
    ]);
    let (after, stats) = optimize_text(&before, &metadata()).unwrap();
    assert_eq!(stats.stack_loads_forwarded, 0);
    equivalent(&before, &after, false);
}

#[test]
fn unreachable_lddw_is_neutralized_as_a_complete_pair() {
    let before = encode(&[
        insn(ebpf::JA, 0, 0, 2, 0),
        insn(ebpf::LD_DW_IMM, 2, 0, 0, 123),
        insn(0, 0, 0, 0, 456),
        insn(ebpf::EXIT, 0, 0, 0, 0),
    ]);
    let (after, stats) = optimize_text(&before, &metadata()).unwrap();
    assert_eq!(stats.unreachable_instructions_neutralized, 1);
    assert_eq!(ebpf::get_insn(&after, 1).opc, ebpf::EXIT);
    assert_eq!(ebpf::get_insn(&after, 2).opc, ebpf::EXIT);
    equivalent(&before, &after, false);
}

#[test]
fn recursive_call_thunk_does_not_become_the_relocation_sentinel() {
    let before = encode(&[
        insn(ebpf::CALL_IMM, 0, 0, 0, 1),
        insn(ebpf::EXIT, 0, 0, 0, 0),
        insn(ebpf::JA, 0, 0, -3, 0),
    ]);
    let (after, stats) = optimize_text(&before, &metadata()).unwrap();
    assert_eq!(stats.call_targets_threaded, 0);
    equivalent(&before, &after, false);
}

fn equivalent_elf(before: &[u8], after: &Optimization) {
    let config = Config {
        enable_register_tracing: true,
        ..Config::default()
    };
    let loader = Arc::new(BuiltinProgram::<State>::new_loader(config));
    for seed in [
        0,
        1,
        u64::MAX,
        1 << 63,
        u32::MAX as u64,
        0xffff_ffff_0000_0000,
    ]
    .into_iter()
    .chain((0..48).map(|n| n * 0x0123_4567_89ab_cdefu64))
    {
        let old = Executable::<State>::load(before, loader.clone()).unwrap();
        old.verify::<RequisiteVerifier>().unwrap();
        let new = Executable::<State>::load(&after.bytes, loader.clone()).unwrap();
        new.verify::<RequisiteVerifier>().unwrap();
        let (mut old, old_count) = execute_loaded(old, seed);
        let (new, new_count) = execute_loaded(new, seed);
        // PC is the VM's instruction index, not a program-visible register. It
        // must follow the emitted old-to-new map; the other registers and every
        // byte of writable VM memory remain exactly equal.
        let old_pc = old.registers[11] as usize;
        let mapped = after
            .stats
            .pc_map
            .iter()
            .find(|entry| entry[0] <= old_pc && old_pc < entry[0] + entry[2])
            .unwrap();
        assert_eq!(
            new.registers[11] as usize,
            mapped[1] + old_pc - mapped[0],
            "old pc {old_pc}, map {:?}, regs {:?}->{:?}",
            after.stats.pc_map,
            old.registers,
            new.registers
        );
        old.registers[11] = new.registers[11];
        assert!(old==new,"ELF machine state diverged seed {seed}: registers {:?}->{:?}, input byte {:?}, stack byte {:?}, heap byte {:?}, result {}->{}",old.registers,new.registers,old.input.iter().zip(&new.input).position(|(a,b)|a!=b),old.stack.iter().zip(&new.stack).position(|(a,b)|a!=b),old.heap.iter().zip(&new.heap).position(|(a,b)|a!=b),old.result,new.result);
        assert!(
            new_count <= old_count,
            "ELF compaction increased executed instructions"
        );
    }
}

#[test]
fn compact_dead_blocks_and_retarget_direct_branches() {
    let text = encode(&[
        insn(ebpf::JA, 0, 0, 2, 0),
        insn(ebpf::MOV64_IMM, 0, 0, 0, 999),
        insn(ebpf::ST_DW_REG, 1, 0, 0, 0),
        insn(ebpf::EXIT, 0, 0, 0, 0),
    ]);
    let before = minimal_elf(&text);
    let after = optimize(&before).unwrap();
    assert_eq!(after.stats.text_slots_removed, 2);
    assert!(after.bytes.len() < before.len());
    equivalent_elf(&before, &after);
}

#[test]
fn identical_direct_call_functions_share_one_body() {
    let text = encode(&[
        insn(ebpf::CALL_IMM, 0, 0, 0, 3),
        insn(ebpf::CALL_IMM, 0, 0, 0, 4),
        insn(ebpf::ST_DW_REG, 1, 0, 0, 0),
        insn(ebpf::EXIT, 0, 0, 0, 0),
        insn(ebpf::MOV64_IMM, 0, 0, 0, 7),
        insn(ebpf::EXIT, 0, 0, 0, 0),
        insn(ebpf::MOV64_IMM, 0, 0, 0, 7),
        insn(ebpf::EXIT, 0, 0, 0, 0),
    ]);
    let before = minimal_elf(&text);
    let after = optimize(&before).unwrap();
    assert_eq!(after.stats.text_slots_removed, 2);
    assert_eq!(after.stats.identical_functions_merged, 1);
    assert!(after.bytes.len() < before.len());
    equivalent_elf(&before, &after);
}

// A real v0 dynamic image containing text/data relative relocations, a symbolic
// rodata relocation, and optionally an unresolved direct-call relocation.
fn relocation_elf(indirect: bool, merge: bool) -> Vec<u8> {
    let mut code = encode(&[
        insn(ebpf::JA, 0, 0, 2, 0),
        insn(ebpf::LD_DW_IMM, 7, 0, 0, 0),
        insn(0, 0, 0, 0, 0), // dead relocated address
        insn(ebpf::LD_DW_IMM, 3, 0, 0, 0),
        insn(0, 0, 0, 0, 0), // function-pointer table
        insn(ebpf::LD_DW_REG, 4, 3, 0, 0),
        if indirect {
            insn(ebpf::CALL_REG, 0, 0, 0, 4)
        } else {
            insn(ebpf::CALL_IMM, 0, 0, 0, -1)
        },
        insn(ebpf::LD_DW_IMM, 5, 0, 0, 0),
        insn(0, 0, 0, 0, 0), // symbol + addend
        insn(ebpf::LD_DW_REG, 6, 5, 0, 0),
        insn(ebpf::ADD64_REG, 0, 6, 0, 0),
        insn(ebpf::ST_DW_REG, 1, 0, 0, 0),
        insn(ebpf::MOV64_IMM, 3, 0, 0, 0),
        insn(ebpf::MOV64_IMM, 4, 0, 0, 0),
        insn(ebpf::MOV64_IMM, 5, 0, 0, 0),
        insn(ebpf::MOV64_IMM, 6, 0, 0, 0),
        insn(ebpf::EXIT, 0, 0, 0, 0),
        insn(ebpf::MOV64_IMM, 0, 0, 0, 42),
        insn(ebpf::EXIT, 0, 0, 0, 0),
    ]);
    if merge {
        code.truncate(17 * 8);
        code.extend(encode(&[
            insn(ebpf::MOV64_IMM, 0, 0, 0, 42),
            insn(ebpf::CALL_IMM, 0, 0, 0, 2),
            insn(ebpf::CALL_IMM, 0, 0, 0, 5),
            insn(ebpf::EXIT, 0, 0, 0, 0),
            insn(ebpf::LD_DW_IMM, 8, 0, 0, 0),
            insn(0, 0, 0, 0, 0),
            insn(ebpf::MOV64_IMM, 8, 0, 0, 0),
            insn(ebpf::EXIT, 0, 0, 0, 0),
            insn(ebpf::LD_DW_IMM, 8, 0, 0, 0),
            insn(0, 0, 0, 0, 0),
            insn(ebpf::MOV64_IMM, 8, 0, 0, 0),
            insn(ebpf::EXIT, 0, 0, 0, 0),
        ]));
    }
    let names =
        b"\0.text\0.rodata\0.data.rel.ro\0.dynamic\0.dynsym\0.dynstr\0.rel.dyn\0.shstrtab\0";
    let strings = b"\0rodata\0callee\0";
    let text = 176usize;
    let rodata = text + code.len();
    let data = rodata + 16;
    let dynamic = data + 8;
    let dynsym = dynamic + 9 * 16;
    let dynstr = dynsym + 3 * 24;
    let rel = (dynstr + strings.len() + 7) & !7;
    let rel_count = (if indirect { 4 } else { 5 }) + if merge { 2 } else { 0 };
    let shstr = rel + rel_count * 16;
    let shoff = (shstr + names.len() + 7) & !7;
    let mut b = vec![0; shoff + 9 * 64];
    b[..4].copy_from_slice(b"\x7fELF");
    b[4] = 2;
    b[5] = 1;
    b[6] = 1;
    compact::put16(&mut b, 16, 3);
    compact::put16(&mut b, 18, 247);
    compact::put32(&mut b, 20, 1);
    compact::put64(&mut b, 24, text as u64);
    compact::put64(&mut b, 32, 64);
    compact::put64(&mut b, 40, shoff as u64);
    compact::put16(&mut b, 52, 64);
    compact::put16(&mut b, 54, 56);
    compact::put16(&mut b, 56, 2);
    compact::put16(&mut b, 58, 64);
    compact::put16(&mut b, 60, 9);
    compact::put16(&mut b, 62, 8);
    for (i, (kind, offset, len, flags)) in [(1, text, shstr - text, 5), (2, dynamic, 9 * 16, 6)]
        .into_iter()
        .enumerate()
    {
        let p = 64 + i * 56;
        compact::put32(&mut b, p, kind);
        compact::put32(&mut b, p + 4, flags);
        for q in [8, 16, 24] {
            compact::put64(&mut b, p + q, offset as u64);
        }
        for q in [32, 40] {
            compact::put64(&mut b, p + q, len as u64);
        }
        compact::put64(&mut b, p + 48, 8);
    }
    compact::put32(&mut code, 1 * 8 + 4, rodata as u32);
    compact::put32(&mut code, 3 * 8 + 4, data as u32);
    compact::put32(&mut code, 7 * 8 + 4, 8); // R_BPF_64_64 addend within the rodata symbol
    if merge {
        for pc in [21, 25] {
            compact::put32(&mut code, pc * 8 + 4, rodata as u32);
        }
    }
    b[text..text + code.len()].copy_from_slice(&code);
    compact::put64(&mut b, rodata + 8, 7);
    compact::put64(&mut b, data, ((text + 17 * 8) as u64) << 32);
    for (i, (tag, value)) in [
        (17, rel),
        (18, rel_count * 16),
        (19, 16),
        (6, dynsym),
        (11, 24),
        (5, dynstr),
        (10, strings.len()),
        (0x6fff_fffa, 3),
        (0, 0),
    ]
    .into_iter()
    .enumerate()
    {
        compact::put64(&mut b, dynamic + i * 16, tag);
        compact::put64(&mut b, dynamic + i * 16 + 8, value as u64);
    }
    for (i, (name, kind, section, value, size)) in
        [(1, 1, 2, rodata, 16), (8, 2, 1, text + 17 * 8, 16)]
            .into_iter()
            .enumerate()
    {
        let at = dynsym + (i + 1) * 24;
        compact::put32(&mut b, at, name);
        b[at + 4] = kind;
        compact::put16(&mut b, at + 6, section);
        compact::put64(&mut b, at + 8, value as u64);
        compact::put64(&mut b, at + 16, size as u64);
    }
    b[dynstr..dynstr + strings.len()].copy_from_slice(strings);
    let mut rs = vec![
        (text + 1 * 8, 8u64),
        (text + 3 * 8, 8u64),
        (data, 8u64),
        (text + 7 * 8, (1u64 << 32) | 1),
    ];
    if !indirect {
        rs.push((text + 6 * 8, (2u64 << 32) | 10));
    }
    if merge {
        for pc in [21, 25] {
            rs.push((text + pc * 8, 8));
        }
    }
    for (i, (site, kind)) in rs.into_iter().enumerate() {
        compact::put64(&mut b, rel + i * 16, site as u64);
        compact::put64(&mut b, rel + i * 16 + 8, kind);
    }
    b[shstr..shstr + names.len()].copy_from_slice(names);
    let specs = [
        (".text", 1, 6, text, code.len(), 0, 0, 8, 0),
        (".rodata", 1, 2, rodata, 16, 0, 0, 8, 0),
        (".data.rel.ro", 1, 3, data, 8, 0, 0, 8, 0),
        (".dynamic", 6, 3, dynamic, 9 * 16, 6, 0, 8, 16),
        (".dynsym", 11, 2, dynsym, 3 * 24, 6, 1, 8, 24),
        (".dynstr", 3, 2, dynstr, strings.len(), 0, 0, 1, 0),
        (".rel.dyn", 9, 2, rel, rel_count * 16, 5, 0, 8, 16),
        (".shstrtab", 3, 0, shstr, names.len(), 0, 0, 1, 0),
    ];
    for (i, (name, kind, flags, offset, size, link, info, alignment, entry)) in
        specs.into_iter().enumerate()
    {
        let at = shoff + (i + 1) * 64;
        let name_offset = names
            .windows(name.len() + 1)
            .position(|s| s == format!("{name}\0").as_bytes())
            .unwrap();
        compact::put32(&mut b, at, name_offset as u32);
        compact::put32(&mut b, at + 4, kind);
        compact::put64(&mut b, at + 8, flags);
        if flags & 2 != 0 {
            compact::put64(&mut b, at + 16, offset as u64);
        }
        compact::put64(&mut b, at + 24, offset as u64);
        compact::put64(&mut b, at + 32, size as u64);
        compact::put32(&mut b, at + 40, link);
        compact::put32(&mut b, at + 44, info);
        compact::put64(&mut b, at + 48, alignment);
        compact::put64(&mut b, at + 56, entry);
    }
    b
}

#[test]
fn relocation_compaction_preserves_symbols_data_function_pointers_and_calls() {
    for indirect in [false, true] {
        let before = relocation_elf(indirect, false);
        let after = optimize(&before).unwrap();
        assert_eq!(after.stats.text_slots_removed, 2);
        assert_eq!(after.stats.relocations_removed, 1);
        assert_eq!(after.stats.relocations_prelinked, 3);
        let image = ElfView::parse(&after.bytes).unwrap();
        for section in image.sections.iter().filter(|s| s.kind == 9) {
            for offset in (section.range.start..section.range.end).step_by(16) {
                assert_eq!(u64_at(&after.bytes, offset + 8).unwrap() as u32, 10);
            }
        }
        assert!(after.bytes.len() < before.len());
        equivalent_elf(&before, &after);
    }
}

#[test]
fn identical_functions_with_data_relocations_are_merged_once() {
    let before = relocation_elf(false, true);
    let after = optimize(&before).unwrap();
    assert_eq!(after.stats.identical_functions_merged, 1);
    assert_eq!(after.stats.text_slots_removed, 6);
    assert_eq!(after.stats.relocations_removed, 2);
    equivalent_elf(&before, &after);
}

#[test]
fn unreachable_function_body_and_symbol_are_removed_together() {
    let code = encode(&[
        insn(ebpf::JA, 0, 0, 2, 0),
        insn(ebpf::MOV64_IMM, 0, 0, 0, 99),
        insn(ebpf::EXIT, 0, 0, 0, 0),
        insn(ebpf::MOV64_IMM, 0, 0, 0, 7),
        insn(ebpf::EXIT, 0, 0, 0, 0),
    ]);
    let mut before = minimal_elf(&code);
    let sym = (120 + code.len() + 7) & !7;
    let strings = sym + 48;
    let names_offset = strings + 6;
    let names = b"\0.text\0.symtab\0.strtab\0.shstrtab\0";
    let shoff = (names_offset + names.len() + 7) & !7;
    before.resize(shoff + 5 * 64, 0);
    before[120 + code.len()..].fill(0);
    before[strings..strings + 6].copy_from_slice(b"\0dead\0");
    before[names_offset..names_offset + names.len()].copy_from_slice(names);
    compact::put64(&mut before, 40, shoff as u64);
    compact::put16(&mut before, 60, 5);
    compact::put16(&mut before, 62, 4);
    compact::put32(&mut before, sym + 24, 1);
    before[sym + 28] = 2;
    compact::put16(&mut before, sym + 30, 1);
    compact::put64(&mut before, sym + 32, 128);
    compact::put64(&mut before, sym + 40, 16);
    let specs = [
        (1, 1, 6, 120, code.len(), 0, 0, 8, 0),
        (7, 2, 0, sym, 48, 3, 2, 8, 24),
        (15, 3, 0, strings, 6, 0, 0, 1, 0),
        (23, 3, 0, names_offset, names.len(), 0, 0, 1, 0),
    ];
    for (index, (name, kind, flags, offset, size, link, info, alignment, entry)) in
        specs.into_iter().enumerate()
    {
        let at = shoff + (index + 1) * 64;
        compact::put32(&mut before, at, name);
        compact::put32(&mut before, at + 4, kind);
        compact::put64(&mut before, at + 8, flags);
        if flags & 2 != 0 {
            compact::put64(&mut before, at + 16, offset as u64);
        }
        compact::put64(&mut before, at + 24, offset as u64);
        compact::put64(&mut before, at + 32, size as u64);
        compact::put32(&mut before, at + 40, link);
        compact::put32(&mut before, at + 44, info);
        compact::put64(&mut before, at + 48, alignment);
        compact::put64(&mut before, at + 56, entry);
    }
    let after = optimize(&before).unwrap();
    assert_eq!(after.stats.text_slots_removed, 2);
    let view = ElfView::parse(&after.bytes).unwrap();
    let syms = view.sections.iter().find(|s| s.name == ".symtab").unwrap();
    assert_eq!(
        &after.bytes[syms.range.start + 24..syms.range.start + 48],
        &[0; 24]
    );
    equivalent_elf(&before, &after);
}

#[test]
fn different_literal_relocations_do_not_merge_observable_results() {
    let mut before = relocation_elf(false, true);
    let view = ElfView::parse(&before).unwrap();
    let rodata = view
        .sections
        .iter()
        .find(|s| s.name == ".rodata")
        .unwrap()
        .addr;
    compact::put32(
        &mut before,
        view.text.start + 21 * 8 + 4,
        (rodata + 8) as u32,
    );
    write(
        &mut before[view.text.clone()],
        23,
        insn(ebpf::LD_DW_REG, 0, 8, 0, 0),
    );
    write(
        &mut before[view.text],
        27,
        insn(ebpf::LD_DW_REG, 0, 8, 0, 0),
    );
    let after = optimize(&before).unwrap();
    assert_eq!(after.stats.identical_functions_merged, 0);
    equivalent_elf(&before, &after);
}

#[test]
fn nonallocated_debug_removal_preserves_vm_execution() {
    let code = encode(&[
        insn(ebpf::MOV64_IMM, 0, 0, 0, 7),
        insn(ebpf::EXIT, 0, 0, 0, 0),
    ]);
    let mut before = minimal_elf(&code);
    let debug = 120 + code.len();
    let names_offset = debug + 32;
    let names = b"\0.text\0.debug_info\0.shstrtab\0";
    let shoff = (names_offset + names.len() + 7) & !7;
    before.resize(shoff + 4 * 64, 0);
    before[debug..].fill(0);
    before[debug..debug + 32].fill(0xab);
    before[names_offset..names_offset + names.len()].copy_from_slice(names);
    compact::put64(&mut before, 40, shoff as u64);
    compact::put16(&mut before, 60, 4);
    compact::put16(&mut before, 62, 3);
    for (i, (name, flags, offset, size)) in [
        (1, 6, 120, code.len()),
        (7, 0, debug, 32),
        (19, 0, names_offset, names.len()),
    ]
    .into_iter()
    .enumerate()
    {
        let at = shoff + (i + 1) * 64;
        compact::put32(&mut before, at, name);
        compact::put32(&mut before, at + 4, if i == 2 { 3 } else { 1 });
        compact::put64(&mut before, at + 8, flags);
        if i == 0 {
            compact::put64(&mut before, at + 16, offset as u64);
        }
        compact::put64(&mut before, at + 24, offset as u64);
        compact::put64(&mut before, at + 32, size as u64);
        compact::put64(&mut before, at + 48, if i == 0 { 8 } else { 1 });
    }
    let after = optimize(&before).unwrap();
    assert_eq!(after.stats.non_allocated_bytes_removed, 32);
    assert!(after.bytes.len() < before.len());
    equivalent_elf(&before, &after);
}

#[test]
fn already_based_relocations_do_not_add_the_vm_base_twice() {
    let mut before = relocation_elf(true, false);
    let view = ElfView::parse(&before).unwrap();
    compact::put32(
        &mut before,
        view.text.start + 3 * 8 + 12,
        (ebpf::MM_BYTECODE_START >> 32) as u32,
    );
    let symbols = view.sections.iter().find(|s| s.name == ".dynsym").unwrap();
    let value = u64_at(&before, symbols.range.start + 24 + 8).unwrap();
    compact::put64(
        &mut before,
        symbols.range.start + 24 + 8,
        value + ebpf::MM_BYTECODE_START,
    );
    let after = optimize(&before).unwrap();
    assert_eq!(after.stats.relocations_prelinked, 3);
    equivalent_elf(&before, &after);
}

#[test]
fn unrelocated_numeric_constants_in_image_ranges_survive_compaction() {
    for value in [184u64, ebpf::MM_BYTECODE_START + 184] {
        let code = encode(&[
            insn(ebpf::JA, 0, 0, 1, 0),
            insn(ebpf::MOV64_IMM, 0, 0, 0, 999),
            insn(ebpf::LD_DW_IMM, 3, 0, 0, value as u32 as i64),
            insn(0, 0, 0, 0, (value >> 32) as i64),
            insn(ebpf::MOV64_REG, 0, 2, 0, 0),
            insn(ebpf::JGE64_REG, 2, 3, 1, 0),
            insn(ebpf::ADD64_IMM, 0, 0, 0, 1),
            insn(ebpf::ST_DW_REG, 1, 0, 0, 0),
            insn(ebpf::EXIT, 0, 0, 0, 0),
        ]);
        let before = minimal_elf(&code);
        let after = optimize(&before).unwrap();
        assert!(after.stats.text_slots_removed > 0);
        let view = ElfView::parse(&after.bytes).unwrap();
        let low = ebpf::get_insn(&after.bytes[view.text.clone()], 1).imm as u32 as u64;
        let high = ebpf::get_insn(&after.bytes[view.text], 2).imm as u32 as u64;
        assert_eq!(low | high << 32, value);
        equivalent_elf(&before, &after);
    }
}

#[test]
fn recovered_vtable_call_removes_disconnected_sdk_dispatch() {
    let mut before = relocation_elf(true, false);
    let view = ElfView::parse(&before).unwrap();
    // The skipped lddw pair becomes a disconnected unknown callx path. An
    // indirect instruction in dead code must not make every function live.
    write(
        &mut before[view.text.clone()],
        1,
        insn(ebpf::CALL_REG, 0, 0, 0, 8),
    );
    write(
        &mut before[view.text.clone()],
        2,
        insn(ebpf::MOV64_IMM, 0, 0, 0, 0),
    );
    let rel = view.sections.iter().find(|s| s.kind == 9).unwrap();
    let shoff = u64_at(&before, 40).unwrap() as usize;
    let rel_index = view.sections.iter().position(|s| s.kind == 9).unwrap();
    before.copy_within(rel.range.start + 16..rel.range.end, rel.range.start);
    compact::put64(
        &mut before,
        shoff + rel_index * 64 + 32,
        (rel.range.len() - 16) as u64,
    );
    let dynamic = view.sections.iter().find(|s| s.kind == 6).unwrap();
    for at in (dynamic.range.start..dynamic.range.end).step_by(16) {
        if u64_at(&before, at).unwrap() == 18 {
            compact::put64(&mut before, at + 8, (rel.range.len() - 16) as u64);
        }
    }
    let after = optimize(&before).unwrap();
    assert_eq!(after.stats.unresolved_indirect_calls, 0);
    assert!(after
        .stats
        .indirect_call_sites
        .iter()
        .any(|site| site.pc == 6 && site.targets == vec![17]));
    assert!(after.stats.text_slots_removed >= 2);
    equivalent_elf(&before, &after);
}

#[test]
fn stack_spilled_vtable_pointer_is_recovered_without_reference_counting_assumptions() {
    let mut before = relocation_elf(true, false);
    let view = ElfView::parse(&before).unwrap();
    // Spill the vtable result, then load it back into the call register.
    for (pc, instruction) in [
        (6, insn(ebpf::ST_DW_REG, 10, 4, -8, 0)),
        (7, insn(ebpf::LD_DW_REG, 4, 10, -8, 0)),
        (8, insn(ebpf::CALL_REG, 0, 0, 0, 4)),
        (9, insn(ebpf::MOV64_IMM, 3, 0, 0, 0)),
        (10, insn(ebpf::MOV64_IMM, 4, 0, 0, 0)),
        (11, insn(ebpf::MOV64_IMM, 5, 0, 0, 0)),
        (12, insn(ebpf::MOV64_IMM, 6, 0, 0, 0)),
        (13, insn(ebpf::ST_DW_IMM, 10, 0, -8, 0)),
        (14, insn(ebpf::EXIT, 0, 0, 0, 0)),
    ] {
        write(&mut before[view.text.clone()], pc, instruction);
    }
    let rel = view.sections.iter().find(|s| s.kind == 9).unwrap();
    let at = (rel.range.start..rel.range.end)
        .step_by(16)
        .find(|at| u64_at(&before, *at).unwrap() == (view.text.start + 7 * 8) as u64)
        .unwrap();
    before.copy_within(at + 16..rel.range.end, at);
    let shoff = u64_at(&before, 40).unwrap() as usize;
    let index = view.sections.iter().position(|s| s.kind == 9).unwrap();
    compact::put64(
        &mut before,
        shoff + index * 64 + 32,
        (rel.range.len() - 16) as u64,
    );
    let dynamic = view.sections.iter().find(|s| s.kind == 6).unwrap();
    for at in (dynamic.range.start..dynamic.range.end).step_by(16) {
        if u64_at(&before, at).unwrap() == 18 {
            compact::put64(&mut before, at + 8, (rel.range.len() - 16) as u64);
        }
    }
    let after = optimize(&before).unwrap();
    assert_eq!(after.stats.unresolved_indirect_calls, 0);
    assert!(after
        .stats
        .indirect_call_sites
        .iter()
        .any(|site| site.pc == 8 && site.targets == vec![17]));
    equivalent_elf(&before, &after);
}

#[test]
fn unknown_indirect_call_keeps_interior_instruction_targets_and_reports_site() {
    let mut before = relocation_elf(true, false);
    let view = ElfView::parse(&before).unwrap();
    // r1 points to arbitrary writable input, so the loaded value is not a
    // vtable proof even when a seeded execution happens not to reach it.
    write(
        &mut before[view.text.clone()],
        5,
        insn(ebpf::LD_DW_REG, 4, 1, 0, 0),
    );
    let after = optimize(&before).unwrap();
    assert_eq!(after.stats.unresolved_indirect_calls, 1);
    assert_eq!(after.stats.text_slots_removed, 0);
    assert!(after
        .stats
        .indirect_call_sites
        .iter()
        .any(|site| site.pc == 6 && !site.resolved));
    equivalent_elf(&before, &after);
}

#[test]
fn interprocedural_vtable_parameter_keeps_exact_target_and_removes_dead_export() {
    let mut before = relocation_elf(true, false);
    let view = ElfView::parse(&before).unwrap();
    // Entry loads a vtable function then calls a helper. The helper performs
    // callx through its argument. Dead exported symbols are not entry roots.
    write(
        &mut before[view.text.clone()],
        6,
        insn(ebpf::CALL_IMM, 0, 0, 0, 5),
    );
    for (pc, instruction) in [
        (7, insn(ebpf::MOV64_IMM, 3, 0, 0, 0)),
        (8, insn(ebpf::MOV64_IMM, 4, 0, 0, 0)),
        (9, insn(ebpf::MOV64_IMM, 5, 0, 0, 0)),
        (10, insn(ebpf::MOV64_IMM, 6, 0, 0, 0)),
        (11, insn(ebpf::EXIT, 0, 0, 0, 0)),
        (12, insn(ebpf::CALL_REG, 0, 0, 0, 4)),
        (13, insn(ebpf::EXIT, 0, 0, 0, 0)),
        (14, insn(ebpf::MOV64_IMM, 0, 0, 0, 99)),
        (15, insn(ebpf::EXIT, 0, 0, 0, 0)),
        (16, insn(ebpf::EXIT, 0, 0, 0, 0)),
    ] {
        write(&mut before[view.text.clone()], pc, instruction);
    }
    let rel = view.sections.iter().find(|s| s.kind == 9).unwrap();
    let at = (rel.range.start..rel.range.end)
        .step_by(16)
        .find(|at| u64_at(&before, *at).unwrap() == (view.text.start + 7 * 8) as u64)
        .unwrap();
    before.copy_within(at + 16..rel.range.end, at);
    let shoff = u64_at(&before, 40).unwrap() as usize;
    let index = view.sections.iter().position(|s| s.kind == 9).unwrap();
    compact::put64(
        &mut before,
        shoff + index * 64 + 32,
        (rel.range.len() - 16) as u64,
    );
    let dynamic = view.sections.iter().find(|s| s.kind == 6).unwrap();
    for at in (dynamic.range.start..dynamic.range.end).step_by(16) {
        if u64_at(&before, at).unwrap() == 18 {
            compact::put64(&mut before, at + 8, (rel.range.len() - 16) as u64);
        }
    }
    let symbols = view.sections.iter().find(|s| s.kind == 11).unwrap();
    compact::put64(
        &mut before,
        symbols.range.start + 2 * 24 + 8,
        (view.text_addr + 14 * 8) as u64,
    );
    let after = optimize(&before).unwrap();
    assert_eq!(after.stats.unresolved_indirect_calls, 0);
    assert!(after
        .stats
        .indirect_call_sites
        .iter()
        .any(|s| s.pc == 12 && s.targets == vec![17]));
    assert!(after.stats.text_slots_removed >= 3);
    equivalent_elf(&before, &after);
}

#[test]
fn partial_stack_overwrite_invalidates_a_function_pointer_proof() {
    let mut before = relocation_elf(true, false);
    let view = ElfView::parse(&before).unwrap();
    for (pc, instruction) in [
        (6, insn(ebpf::ST_DW_REG, 10, 4, -8, 0)),
        (7, insn(ebpf::ST_B_REG, 10, 2, -5, 0)),
        (8, insn(ebpf::LD_DW_REG, 4, 10, -8, 0)),
        (9, insn(ebpf::CALL_REG, 0, 0, 0, 4)),
        (10, insn(ebpf::MOV64_IMM, 3, 0, 0, 0)),
        (11, insn(ebpf::MOV64_IMM, 4, 0, 0, 0)),
        (12, insn(ebpf::MOV64_IMM, 5, 0, 0, 0)),
        (13, insn(ebpf::MOV64_IMM, 6, 0, 0, 0)),
        (14, insn(ebpf::ST_DW_IMM, 10, 0, -8, 0)),
    ] {
        write(&mut before[view.text.clone()], pc, instruction);
    }
    let rel = view.sections.iter().find(|s| s.kind == 9).unwrap();
    let at = (rel.range.start..rel.range.end)
        .step_by(16)
        .find(|at| u64_at(&before, *at).unwrap() == (view.text.start + 7 * 8) as u64)
        .unwrap();
    before.copy_within(at + 16..rel.range.end, at);
    let shoff = u64_at(&before, 40).unwrap() as usize;
    let index = view.sections.iter().position(|s| s.kind == 9).unwrap();
    compact::put64(
        &mut before,
        shoff + index * 64 + 32,
        (rel.range.len() - 16) as u64,
    );
    let dynamic = view.sections.iter().find(|s| s.kind == 6).unwrap();
    for at in (dynamic.range.start..dynamic.range.end).step_by(16) {
        if u64_at(&before, at).unwrap() == 18 {
            compact::put64(&mut before, at + 8, (rel.range.len() - 16) as u64);
        }
    }
    let after = optimize(&before).unwrap();
    assert!(after
        .stats
        .indirect_call_sites
        .iter()
        .any(|site| site.pc == 9 && !site.resolved));
    assert_eq!(after.stats.text_slots_removed, 0);
    equivalent_elf(&before, &after);
}


#[test]
fn arithmetic_derived_code_address_stays_unresolved_during_relocation() {
    let mut before=relocation_elf(true,false);
    let view=ElfView::parse(&before).unwrap();
    for (pc,instruction) in [
        (6,insn(ebpf::ADD64_IMM,4,0,0,0)),
        (7,insn(ebpf::ADD64_IMM,4,0,0,8)),
        (8,insn(ebpf::CALL_REG,0,0,0,4)),
        (9,insn(ebpf::MOV64_IMM,3,0,0,0)),
        (10,insn(ebpf::MOV64_IMM,4,0,0,0)),
        (11,insn(ebpf::MOV64_IMM,5,0,0,0)),
        (12,insn(ebpf::MOV64_IMM,6,0,0,0)),
    ] {write(&mut before[view.text.clone()],pc,instruction);}
    let rel=view.sections.iter().find(|s|s.kind==9).unwrap();
    let at=(rel.range.start..rel.range.end).step_by(16).find(|at|u64_at(&before,*at).unwrap()==(view.text.start+7*8) as u64).unwrap();
    before.copy_within(at+16..rel.range.end,at);
    let shoff=u64_at(&before,40).unwrap() as usize;
    let index=view.sections.iter().position(|s|s.kind==9).unwrap();
    compact::put64(&mut before,shoff+index*64+32,(rel.range.len()-16) as u64);
    let dynamic=view.sections.iter().find(|s|s.kind==6).unwrap();
    for at in (dynamic.range.start..dynamic.range.end).step_by(16) {
        if u64_at(&before,at).unwrap()==18 {compact::put64(&mut before,at+8,(rel.range.len()-16) as u64);}
    }
    let after=optimize(&before).unwrap();
    assert!(after.stats.indirect_call_sites.iter().any(|s|s.pc==8&&!s.resolved));
    assert_eq!(after.stats.text_slots_removed,0);
    equivalent_elf(&before,&after);
}
