use {
    thiserror::Error,

    solana_program::{
        program_error::ProgramError,
        decode_error::DecodeError
    },
    
    num_derive::FromPrimitive,
};

#[derive(Debug, Error, PartialEq, Clone, Copy, FromPrimitive)]
pub enum LotteryError {
    #[error("invalid config authority.")]
    InvalidConfigAuthority = 50,
    #[error("invalid config account.")]
    InvalidConfigAccount,
    #[error("Maximum time has been exceed")]
    MaximumTimeExceed,
    #[error("invalid winners amount.")]
    InvalidWinnersAmount,
    #[error("winners_amount % fund_amount == 0")]
    WinnersAndFundAmountMismatch,
    #[error("invalid starting or ending time.")]
    InvalidTime,
    #[error("invalid funding amount.")]
    InvalidFundAmount,
    #[error("only 'TOKEN_STANDARD_PROGRAM' supported.")]
    OnlyTokenStandardProgram,
    #[error("invalid usdc-mint account.")]
    InvalidUsdcMintAccount,
    #[error("invalid discriminator.")]
    InvalidDiscriminator,
    #[error("lottery is not in correct state.")]
    InvalidLotteryState,
    #[error("lottery authority cannot attend in its own lottery!")]
    InvalidUser,
    #[error("invalid lottery's usdc associated token account.")]
    InvalidLotteryAssociatedUsdcTokenAccount,
    #[error("cannot exceed the max amount of the tickets_per_user.")]
    MaxTicketsAmountViolated,
    #[error("invalid ticket amount.")]
    InvalidTicketAmount,
    #[error("lottery account mismatch.")]
    InvalidLotteryAccount,
    #[error("invalid authority for the lottery_acount.")]
    InvalidLotteryAccountAuthority,
    #[error("failed to add space to the lottery-account, thus cannot add tickets (realloc failed).")]
    ReallocationFailed,
    #[error("min tickets required amount MUST be >= winners amount.")]
    InvalidMinTicketsReqAndWinnersAmount,
    #[error("invalid minimum tickets amount is less than the minimum.")]
    InvalidMinimumAmountTickets,
    #[error("failed to find program address.")]
    FailedToFindProgramAddress,
    #[error("overflow.")]
    Overflow,
    #[error("invalid SOL price feed account.")]
    InvalidSolPriceFeedAccount,
    #[error("invalid BTC price feed account.")]
    InvalidBtcPriceFeedAccount,
    #[error("invalid ETH price feed account.")]
    InvalidEthPriceFeedAccount,
    #[error("lottery was not successfull.")]
    LotteryWasNotSuccessfull,
    #[error("lottery already ended.")]
    LotteryAlreadyEnded,
    #[error("invalid price-feed-account's owner.")]
    InvalidPriceFeedAccountsOwner,
    #[error("insufficient random numbers.")]
    InsufficientRandomNumbers,
    #[error("funds already withdrawed.")]
    FundsAlreadyWithdrawed,
    #[error("winners are not selected yet!")]
    WinnersNotSelected,
    #[error("winner account not-found!")]
    WinnerNotFound,
    #[error("invalid arbitrary associated token account.")]
    InvalidLotteryArbitraryAssociatedTokenAccount,
    #[error("invalid arbitrary mint account.")]
    InvalidArbitraryMintAccount,
    #[error("cannot close the account, users funds are still there!")]
    CannotCloseAccounts,
    #[error("invalid lottery_tickets_fee.")]
    InvalidLotteryTicketsFee,
    #[error("invalid new authority.")]
    InvalidNewConfigAccountAuthority,
    #[error("protocol is paused due to the vulnerability.")]
    ProtocolIsPaused,
    #[error("invalid max_number of winners.")]
    InvalidMaxNumberOfWinners,
    #[error("invalid max price feed age.")]
    InvalidMaxPriceFeedAge,
    #[error("invalid n parameter.")]
    InvalidNParameter,
    #[error("invalid amount of lotteries's associated token accounts.")]
    InvalidAmountOfAssociatedTokenAccount,
    #[error("invalid amount of lotteries's account.")]
    InvalidAmountOfLotteries,
    #[error("invalid treasury account.")]
    InvalidTreasuryAccount,
    #[error("protocol fee is already claimed for this lottery!")]
    ProtocolFeeAlreadyClaimed,
    #[error("invalid usdc token account.")]
    InvalidUsdcTokenAccount,
    #[error("failed to get the ticket.")]
    FailedToGetTicket,
    #[error("max tickets per instruction is 300-tickets.")]
    MaxTicketsPerInstructionExceeded,
    #[error("max lottery description length exceeded.")]
    MaxLotteryDescriptionBytesExceeded,
    #[error("first close lottery arbtirary ata.")]
    FirstCloseLotteryArbitrartAssociatedTokenAccount,
    #[error("user is one of the winners.")]
    UserIsOneOfTheWinners,
    #[error("current_ticket_price == new_ticket_price.")]
    InvalidTicketPrice,
    #[error("account must be a raw-account.")]
    AccountMustBeRaw,
    #[error("current_lottery_ticket_price != user_expected_ticket_price")]
    ExpectedTicketPriceViolated,
    #[error("this instruction must be the transaction-level instruction.")]
    MustBeTransactionLevelIx,
    #[error("this instruction must be the last instruction in the transaction")]
    MustBeTheLastIx,
    #[error("invalid sysvar-instruction")]
    InvalidSysvarInstructionAccount
}

impl From<LotteryError> for ProgramError {
    fn from(e: LotteryError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for LotteryError {
    fn type_of() -> &'static str {
        "LotteryError"
    }
}
