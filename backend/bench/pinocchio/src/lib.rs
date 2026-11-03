#![no_std]

use pinocchio::{entrypoint::lazy::InstructionContext, program_error::ProgramError, ProgramResult};

pinocchio::lazy_program_entrypoint!(process_instruction);
pinocchio::nostd_panic_handler!();
pinocchio::no_allocator!();

const ID: [u8; 32] = [
    218, 7, 92, 178, 255, 94, 198, 129, 118, 19, 222, 83, 11, 105, 42, 135, 53, 71, 119, 105, 218,
    71, 67, 12, 189, 129, 84, 51, 92, 74, 131, 39,
];
const INITIALIZE: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];

fn process_instruction(mut context: InstructionContext) -> ProgramResult {
    // Only locate the instruction tail; no AccountInfo array or heap is needed.
    while context.remaining() > 0 {
        context.next_account()?;
    }
    if context.program_id()? != &ID {
        pinocchio::msg!("AnchorError occurred. Error Code: DeclaredProgramIdMismatch. Error Number: 4100. Error Message: The declared program id does not match the actual program id.");
        return Err(ProgramError::Custom(4100));
    }
    let data = context.instruction_data()?;
    if data.starts_with(&INITIALIZE) {
        pinocchio::msg!("Instruction: Initialize");
        return Ok(());
    }
    if data.starts_with(&0x1d9acb512ea545e4u64.to_le_bytes()) {
        pinocchio::msg!("AnchorError occurred. Error Code: EventInstructionStub. Error Number: 1500. Error Message: The program was compiled without `event-cpi` feature.");
        return Err(ProgramError::Custom(1500));
    }
    pinocchio::msg!("AnchorError occurred. Error Code: InstructionFallbackNotFound. Error Number: 101. Error Message: Fallback functions are not supported.");
    Err(ProgramError::Custom(101))
}
