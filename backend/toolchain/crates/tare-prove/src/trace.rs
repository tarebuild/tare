//! Optional diagnostic register tracing; ordinary proofs do not enable it.
use litesvm::{InvocationInspectCallback, LiteSVM};
use serde::Serialize;
use solana_program_runtime::invoke_context::{Executable, InvokeContext, RegisterTrace};
use solana_transaction::sanitized::SanitizedTransaction;
use solana_transaction_context::{instruction::InstructionContext, IndexOfAccount};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
#[derive(Clone, Default)]
pub(crate) struct Recorder(Arc<Mutex<State>>);
#[derive(Default)]
struct State {
    context: (String, String, String),
    records: Vec<InvocationTrace>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvocationTrace {
    pub sequence: String,
    pub label: String,
    pub variant: String,
    pub program: String,
    pub data_hex: String,
    pub elf_sha256: String,
    pub executed_instructions: usize,
    #[serde(serialize_with = "crate::json_u64::serialize_json")]
    pub opcode_counts: BTreeMap<String, u64>,
    #[serde(serialize_with = "crate::json_u64::serialize_json")]
    pub pc_counts: BTreeMap<u64, u64>,
    pub calls: Vec<CallTrace>,
}
#[derive(Clone, Serialize)]
pub struct CallTrace {
    #[serde(with = "crate::json_u64")]
    pub pc: u64,
    pub opcode: u8,
    pub target: i64,
    #[serde(with = "crate::json_u64::array")]
    pub arguments: [u64; 5],
}
impl Recorder {
    pub(crate) fn context(&self, sequence: &str, label: &str, variant: &str) {
        self.0.lock().unwrap().context = (sequence.into(), label.into(), variant.into());
    }
    pub(crate) fn records(&self) -> Vec<InvocationTrace> {
        self.0.lock().unwrap().records.clone()
    }
}
impl InvocationInspectCallback for Recorder {
    fn before_invocation(
        &self,
        _: &LiteSVM,
        _: &SanitizedTransaction,
        _: &[IndexOfAccount],
        _: &mut InvokeContext,
        _: bool,
    ) {
    }
    fn after_invocation(
        &self,
        svm: &LiteSVM,
        _: &SanitizedTransaction,
        _: &[IndexOfAccount],
        context: &InvokeContext,
        enabled: bool,
    ) {
        if !enabled {
            return;
        }
        context.iterate_vm_traces(&|instruction: InstructionContext,
                                    executable: &Executable,
                                    registers: RegisterTrace| {
            if registers.is_empty() {
                return;
            }
            let mut state = self.0.lock().unwrap();
            let (sequence, label, variant) = state.context.clone();
            let (_, text) = executable.get_text_bytes();
            let mut opcodes = BTreeMap::new();
            let mut pcs = BTreeMap::new();
            let mut calls = Vec::new();
            for regs in registers {
                let pc = regs[11];
                let insn = solana_program_runtime::solana_sbpf::ebpf::get_insn_unchecked(
                    text,
                    pc as usize,
                );
                *opcodes.entry(format!("0x{:02x}", insn.opc)).or_insert(0) += 1;
                *pcs.entry(pc).or_insert(0) += 1;
                if insn.opc & 0x07 == 0x05 && insn.opc & 0xf0 == 0x80 {
                    calls.push(CallTrace {
                        pc,
                        opcode: insn.opc,
                        target: insn.imm,
                        arguments: [regs[1], regs[2], regs[3], regs[4], regs[5]],
                    });
                }
            }
            let program_key = instruction.get_program_key().unwrap();
            let elf_sha256 = svm
                .accounts_db()
                .try_program_elf_bytes(program_key)
                .map(|bytes| {
                    use sha2::Digest;
                    hex::encode(sha2::Sha256::digest(bytes))
                })
                .unwrap_or_default();
            state.records.push(InvocationTrace {
                sequence,
                label,
                variant,
                program: program_key.to_string(),
                elf_sha256,
                data_hex: hex::encode(instruction.get_instruction_data()),
                executed_instructions: registers.len(),
                opcode_counts: opcodes,
                pc_counts: pcs,
                calls,
            });
        });
    }
}
