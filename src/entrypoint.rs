use {
    solana_program::{
        pubkey::Pubkey,
        entrypoint::ProgramResult,
        entrypoint,
        account_info::AccountInfo
    },
    crate::processor::Processor
};

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts_info: &[AccountInfo],
    instruction_data: &[u8]
) -> ProgramResult {
    if let Result::Err(error) = Processor::process(program_id, accounts_info, instruction_data) {
        return Result::Err(error);
    };

    Result::Ok(())
}