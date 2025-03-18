use {
    borsh::{
        BorshDeserialize,
        BorshSerialize
    },

    solana_program::{
        program_error::ProgramError,
        pubkey::Pubkey,
        instruction::{
            AccountMeta,
            Instruction
        }
    },
    
    crate::{
        types::*,
        program::ID as LOTTERY_PROGRAM_ID
    }
};

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, PartialEq)]
pub enum Instructions {
    /// Create And Initialize Config Account
    /// 
    /// Accounts Expected By This Instrcution :
    ///     0. `[s]` authority of the global config account
    ///     1. `[w,s]` funding account for rent
    ///     2. `[w]` config account pda
    ///     3. `[]` system-program account
    CreateAndInitializeProgramConfigAccount {
        authority: Pubkey,
        lottery_creation_fee: u64,
        lottery_tickets_fee: f64,
        maximum_number_of_winners: u8,
        pyth_price_receiver_programid: Pubkey,
        usdc_mint_account: Pubkey,
        maximum_time_of_price_feed_age: u8,
        minimum_tickets_to_be_sold_in_lottery: u8,
        pyth_price_feed_accounts: [PriceFeedAccount; 3],
        maximum_time_for_lottery_account: u32,
        treasury: Pubkey,
        pyth_price_feed_ids: [String; 3],
        max_lottery_description_bytes: u64
    },

    /// Create And Initialize lottery Account
    /// 
    /// Accounts Expected By This Instruction :
    ///     0. `[w]` lottery account pda
    ///     1. `[s]` authority of the lottery account
    ///     2. `[w,s]` funding account for rents, usdc and arbitrary tokens
    ///     3. `[]` USDC mint account 
    ///     4. `[w]` lottery-account's associated usdc-token account
    ///     5. `[w]` funding-account's usdc-token account
    ///     6. `[]` Arbitrary mint account
    ///     7. `[w]` lottery-account's associated arbitrary-token account
    ///     8. `[w]` funding-account's arbitrary-token account 
    ///     9. `[]` standard token program account
    ///    10. `[]` associated token program account 
    ///    11. `[]` system-program account
    ///    12. `[]` config account
    CreateAndInitializeLotteryAccount {
        fund_amount: u64,
        winners_count: u8,
        starting_time: i64,
        ending_time: i64,
        minimum_tickets_amount_required_to_be_sold: u32,
        ticket_price: u64,
        maximum_number_of_tickets_per_user: Option<u32>,
        lottery_description: String
    },

    /// Create And Initialize User Account
    /// 
    /// Accounts Expected By This Instruction : 
    ///     0. `[w]` user account
    ///     1. `[s]` authority of the user-account
    ///     2. `[w,s]` funding account for rent
    ///     3. `[]` lottery account
    ///     4. `[]` system program account
    ///     5. `[]` config account
    CreateAndInitializeUserAccount,

    /// Buy Ticket/s For Specific lottery
    /// 
    /// Accounts Expected By this Instruction :
    ///     0. `[]` config account
    ///     1. `[w]` user account
    ///     2. `[s]` authority of the user-account
    ///     3. `[w,s]` funding account for rent and usdc tokens
    ///     4. `[w]` lottery account
    ///     5. `[w]` lottery's usdc associated token account
    ///     6. `[w]` funding's usdc token account
    ///     7. `[]` usdc mint account
    ///     8. `[]` system program account
    ///     9. `[]` standard token program account
    BuyTicket {
        tickets_amount: u32,
        expected_token_price_per_ticket: u64
    },

    /// Change lottery's ticket-price
    /// 
    /// Accounts Expected By This Instruction :
    ///     0. `[w]` lottery account
    ///     1. `[s]` lottery-account's authority
    ///     2. `[]` config account
    ChangeLotteryTicketPrice {
        new_ticket_price: u64 // USDC
    },

    /// End successfull lottery and pick winners *<everyone can call this instruction>*
    /// 
    /// Accounts Expected By This Instruction :
    ///     0. `[w]` lottery account
    ///     1. `[]` config account
    ///     2. `[]` SOL pyth price feed account
    ///     3. `[]` BTC pyth price feed account
    ///     4. `[]` ETH pyth price feed account
    EndLotteryAndPickWinners,

    /// lottery's creator (owner) will be able to withdraw the -> total_tickets_usdc - protocol_fee
    /// 
    /// Accounts Expected By This Instruction :
    ///     0. `[w]` lottery account
    ///     1. `[]` config account
    ///     2. `[s]` lottery authority
    ///     3. `[w]` lottery's associated usdc token account
    ///     4. `[w]` fund-receiver usdc token account
    ///     5. `[]` usdc mint account 
    ///     6. `[]` standard token program account 
    WithdrawSucceedLottery,

    /// Winners will be able to get their prize
    /// 
    /// Accounts Expected By This Instruction :
    ///     0. `[w]` lottery account
    ///     1. `[]` user account
    ///     2. `[s]` user account authority
    ///     3. `[w]` lottery's associated arbitrary token account
    ///     4. `[w]` fund-receiver arbitrary token account
    ///     5. `[]` arbitrary mint account 
    ///     6. `[]` standard token program account 
    ///     7. `[]` config account
    WithdrawLotterysWinners,

    /// Users can claim their tickets rent exempt after lottery ended successfuly (non-winner users)
    /// 
    /// Accounts Expected By This Instruction :
    ///     0. `[w]` lottery account
    ///     1. `[w]` user account
    ///     2. `[s]` user account authority
    ///     3. `[w]` fund-receiver tickets rent_exempt lamports account
    ///     4. `[w]` fund-receiver user account rent_exempt
    ///     5. `[]` config account
    WithdrawAndCloseSucceedUser,

    /// Lottery's owner (creator) can withdraw their funds if lottery fails
    /// 
    /// Accounts Expected By This Instruction :
    ///     0. `[]` config account
    ///     1. `[w]` lottery account
    ///     2. `[s]` lottery authority
    ///     3. `[]` usdc mint account
    ///     4. `[]` arbitrary mint account
    ///     5. `[w]` lottery's associated usdc token account
    ///     6. `[w]` lottery's associated arbitrary token account
    ///     7. `[w]` fund-receiver usdc token account
    ///     8. `[w]` fund-receiver arbitrary token account
    ///     9. `[w]` fund-receiver rent_exempt lamports account
    ///    10. `[]` standard token program account
    WithdrawFailedLottery,

    /// Users can withdraw their funds and close their accounts if lottery fails
    /// 
    /// Accounts Expected By This Instruction :
    ///     0. `[]` config account
    ///     1. `[w]` lottery account
    ///     2. `[w]` user account
    ///     3. `[s]` user_account's authority
    ///     4. `[]` usdc mint account
    ///     5. `[w]` lottery's associated usdc token account
    ///     6. `[w]` fund-receiver usdc token account
    ///     7. `[w]` fund-receiver tickets_rent_exempt lamports account
    ///     8. `[w]` fund-receiver rent_exempt lamports account
    ///     9. `[]` standard token program account
    WithdrawAndCloseFailedUser,

    /// Lottery owner(creator) can close the lottery & lottery_associated_usdc_token accounts to reclaim rent_exempts 
    /// 
    /// Accounts Expected By This Instruction : 
    ///     0. `[]` config account
    ///     1. `[w]` lottery account
    ///     2. `[s]` lottery account authority
    ///     3. `[]` usdc mint account
    ///     4. `[w]` lottery's associated usdc token account
    ///     5. `[w]` fund-receiver usdc token account
    ///     6. `[w]` fund-receiver rent_exempt lamports account
    ///     7. `[]` standard token program account  
    CloseLotteryAccountAndUsdcTokenAccount,

    /// Change the lottery_fee_creation amount
    /// 
    /// Accounts Expected By This Instruction :
    ///     0. `[w]` config account
    ///     1. `[s]` config authority account 
    ChangeFeeOfLotteryCreation {
        new_fee: u64
    },

    /// Change the config_account authority
    /// 
    /// Accounts Expected By This Instruction :
    ///     0. `[w]` config account
    ///     1. `[s]` config authority account
    ///     2. `[]` new authority for config account
    ChangeConfigAccountAuthority,

    /// Change the lottery_ticket_fee amount
    /// 
    /// Accounts Expected By This Instruction :
    ///     0. `[w]` config account
    ///     1. `[s]` config authority account 
    ChangeFeeOfTickets {
        new_fee: f64
    },

    /// Claim protocl fees from N lottery accounts
    /// 
    /// Accounts Expected By This Instruction :
    ///     0. `[]` config account
    ///     1. `[s]` config account authority
    ///     2. `[w]` treasury (usdc token account)
    ///     3. `[]` usdc mint account
    ///     4. `[]` standard token program account
    ///     5. 5..5+N `[w]` N lottery account
    ///     6. 5+N.. `[w]` N lotteries's associated usdc token accounts (atleast instruction MUST have one token-account)
    ClaimProtocolFees {
        n: u8
    },

    /// Change the max_number_of_winners
    /// 
    /// Accounts Expected By This Instruction :
    ///     0. `[w]` config account
    ///     1. `[s]` config authority account 
    ChangeMaximumNumberOfWinners {
        new_max: u8
    },

    /// Change the max_age_of_price_feed
    /// 
    /// Accounts Expected By This Instruction :
    ///     0. `[w]` config account
    ///     1. `[s]` config authority account 
    ChangeMaximumAgeOfPriceFeed {
        new_max: u8
    },

    /// Change the is_pause flag
    /// 
    /// Accounts Expected By This Instruction :
    ///     0. `[w]` config account
    ///     1. `[s]` config authority account 
    ChangePauseState {
        pause: bool
    },

    /// Change the treasury, USDC token account
    /// 
    /// Accounts Expected By This Instruction :
    ///     0. `[w]` config account
    ///     1. `[s]` config authority account
    ///     2. `[]` new-treasury account
    ChangeTreasury,
    
    /// Change the previous protocol's mint account
    /// 
    /// Accounts Expected By This Instruction :
    ///     0. `[w]` config account
    ///     1. `[s]` config authority account
    ChangeProtocolMintAccount {
        new_mint_account: Pubkey
    },

    /// Change the Pyth's price receiver program-id
    ///  
    /// Accounts Expected By This Instruction :
    ///     0. `[w]` config account
    ///     1. `[s]` config authority account
    ChangePythPriceReceiverProgramAccount {
        new_pyth_price_receiver_programid: Pubkey
    },

    /// Change the price feed id
    ///  
    /// Accounts Expected By This Instruction :
    ///     0. `[w]` config account
    ///     1. `[s]` config authority account
    ChangePriceFeedId {
        index: u8,
        price_feed_id: String
    },

    /// Change the price feed account
    ///  
    /// Accounts Expected By This Instruction :
    ///     0. `[w]` config account
    ///     1. `[s]` config authority account
    ChangePriceFeedAccount {
        index: u8,
        price_feed_account: Pubkey
    },

    /// Change the max lottery's-description length (Bytes)
    ///  
    /// Accounts Expected By This Instruction :
    ///     0. `[w]` config account
    ///     1. `[s]` config authority account
    ChangeMaxLotteryDescriptionLength {
        new_length: u64
    }
}

impl Instructions {
    pub fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        Instructions::try_from_slice(data)
            .map_err(|_| ProgramError::InvalidInstructionData)
    }
}

pub fn instruction_create_and_initialize_lottery_account(
    lottery_account: Pubkey,
    lottery_account_authority_account: Pubkey,
    funding_account: Pubkey,
    usdc_mint_account: Pubkey,
    lottery_associated_usdc_token_account: Pubkey,
    funding_usdc_token_account: Pubkey,
    arbitrary_mint_account: Pubkey,
    lottery_associated_arbitrary_token_account: Pubkey,
    funding_arbitrary_token_account: Pubkey,
    standard_token_program_account: Pubkey,
    associated_token_program_account: Pubkey,
    system_program: Pubkey,
    config_account: Pubkey,
    fund_amount: u64,
    winners_count: u8,
    starting_time: i64,
    ending_time: i64,
    minimum_tickets_amount_required_to_be_sold: u32,
    ticket_price: u64,
    maximum_number_of_tickets_per_user: Option<u32>,
    lottery_description: String
) -> Instruction {
    let accounts_meta = vec![
        AccountMeta::new(lottery_account, false),
        AccountMeta::new_readonly(lottery_account_authority_account, true),
        AccountMeta::new(funding_account, true),
        AccountMeta::new_readonly(usdc_mint_account, false),
        AccountMeta::new(lottery_associated_usdc_token_account, false),
        AccountMeta::new(funding_usdc_token_account, false),
        AccountMeta::new_readonly(arbitrary_mint_account, false),
        AccountMeta::new(lottery_associated_arbitrary_token_account, false),
        AccountMeta::new(funding_arbitrary_token_account, false),
        AccountMeta::new_readonly(standard_token_program_account, false),
        AccountMeta::new_readonly(associated_token_program_account, false),
        AccountMeta::new_readonly(system_program, false),
        AccountMeta::new_readonly(config_account, false)
    ];

    let instruction_data = Instructions::CreateAndInitializeLotteryAccount {
        fund_amount,
        winners_count,
        starting_time,
        ending_time,
        minimum_tickets_amount_required_to_be_sold,
        ticket_price,
        maximum_number_of_tickets_per_user,
        lottery_description
    };

    Instruction::new_with_borsh(
        LOTTERY_PROGRAM_ID,
        &instruction_data,
        accounts_meta
    )
}

pub fn instruction_create_and_initialize_user_account(
    user_account: Pubkey,
    user_account_authority_account: Pubkey,
    funding_account: Pubkey,
    lottery_account: Pubkey,
    system_program_account: Pubkey,
    config_account: Pubkey
) -> Instruction {
    let accounts_meta = vec![
        AccountMeta::new(user_account, false),
        AccountMeta::new_readonly(user_account_authority_account, true),
        AccountMeta::new(funding_account, true),
        AccountMeta::new_readonly(lottery_account, false),
        AccountMeta::new_readonly(system_program_account, false),
        AccountMeta::new_readonly(config_account, false)
    ];

    let instruction_data = Instructions::CreateAndInitializeUserAccount;

    Instruction::new_with_borsh(
        LOTTERY_PROGRAM_ID,
        &instruction_data,
        accounts_meta
    )
}

pub fn instruction_buy_ticket(
    config_account: Pubkey,
    user_account: Pubkey,
    user_account_authority_account: Pubkey,
    funding_account: Pubkey,
    lottery_account: Pubkey,
    lottery_usdc_associated_token_account: Pubkey,
    funding_usdc_token_account: Pubkey,
    usdc_mint_account: Pubkey,
    system_program_account: Pubkey,
    standard_token_program_account: Pubkey,
    tickets_amount: u32,
    expected_token_price_per_ticket: u64
) -> Instruction {
    let accounts_meta = vec![
        AccountMeta::new_readonly(config_account, false),
        AccountMeta::new(user_account, false),
        AccountMeta::new_readonly(user_account_authority_account, true),
        AccountMeta::new(funding_account, true),
        AccountMeta::new(lottery_account, false),
        AccountMeta::new(lottery_usdc_associated_token_account, false),
        AccountMeta::new(funding_usdc_token_account, false),
        AccountMeta::new_readonly(usdc_mint_account, false),
        AccountMeta::new_readonly(system_program_account, false),
        AccountMeta::new_readonly(standard_token_program_account, false)
    ];

    let instruction_data = Instructions::BuyTicket {
        tickets_amount,
        expected_token_price_per_ticket
    };

    Instruction::new_with_borsh(
        LOTTERY_PROGRAM_ID,
        &instruction_data,
        accounts_meta
    )
}

pub fn instruction_change_lottery_ticket_price(
    lottery_account: Pubkey,
    lottery_account_authority_account: Pubkey,
    config_account: Pubkey,
    new_ticket_price: u64
) -> Instruction {
    let accounts_meta = vec![
        AccountMeta::new(lottery_account, false),
        AccountMeta::new_readonly(lottery_account_authority_account, true),
        AccountMeta::new_readonly(config_account, false)
    ];

    let instruction_data = Instructions::ChangeLotteryTicketPrice { new_ticket_price };

    Instruction::new_with_borsh(
        LOTTERY_PROGRAM_ID,
        &instruction_data,
        accounts_meta
    )
}

pub fn instruction_end_lottery_and_pick_winners(
    lottery_account: Pubkey,
    config_account: Pubkey,
    sol_price_feed_account: Pubkey,
    btc_price_feed_account: Pubkey,
    eth_price_feed_account: Pubkey
) -> Instruction {
    let accounts_meta = vec![
        AccountMeta::new(lottery_account, false),
        AccountMeta::new_readonly(config_account, false),
        AccountMeta::new_readonly(sol_price_feed_account, false),
        AccountMeta::new_readonly(btc_price_feed_account, false),
        AccountMeta::new_readonly(eth_price_feed_account, false)
    ];

    let instruction_data = Instructions::EndLotteryAndPickWinners;

    Instruction::new_with_borsh(
        LOTTERY_PROGRAM_ID,
        &instruction_data,
        accounts_meta
    )
}

pub fn instruction_withdraw_succeed_lottery(
    lottery_account: Pubkey,
    config_account: Pubkey,
    lottery_authority_account: Pubkey,
    lottery_associated_usdc_token_account: Pubkey,
    fund_receiver_usdc_token_account: Pubkey,
    usdc_mint_account: Pubkey,
    standard_token_program_account: Pubkey
) -> Instruction {
    let accounts_meta = vec![
        AccountMeta::new(lottery_account, false),
        AccountMeta::new_readonly(config_account, false),
        AccountMeta::new_readonly(lottery_authority_account, true),
        AccountMeta::new(lottery_associated_usdc_token_account, false),
        AccountMeta::new(fund_receiver_usdc_token_account, false),
        AccountMeta::new_readonly(usdc_mint_account, false),
        AccountMeta::new_readonly(standard_token_program_account, false)
    ];

    let instruction_data = Instructions::WithdrawSucceedLottery;

    Instruction::new_with_borsh(
        LOTTERY_PROGRAM_ID,
        &instruction_data,
        accounts_meta
    )
}

pub fn instruction_withdraw_lottery_winners(
    lottery_account: Pubkey,
    user_account: Pubkey,
    user_account_authority_account: Pubkey,
    lottery_associated_arbitrary_token_account: Pubkey,
    fund_receiver_arbitrary_token_account: Pubkey,
    arbitrary_mint_account: Pubkey,
    standard_token_program_account: Pubkey,
    config_account: Pubkey
) -> Instruction {
    let accounts_meta = vec![
        AccountMeta::new(lottery_account, false),
        AccountMeta::new_readonly(user_account, false),
        AccountMeta::new_readonly(user_account_authority_account, true),
        AccountMeta::new(lottery_associated_arbitrary_token_account, false),
        AccountMeta::new(fund_receiver_arbitrary_token_account, false),
        AccountMeta::new_readonly(arbitrary_mint_account, false),
        AccountMeta::new_readonly(standard_token_program_account, false),
        AccountMeta::new_readonly(config_account, false)
    ];

    let instruction_data = Instructions::WithdrawLotterysWinners;

    Instruction::new_with_borsh(
        LOTTERY_PROGRAM_ID,
        &instruction_data,
        accounts_meta
    )
}

pub fn instruction_withdraw_and_close_succeed_user(
    lottery_account: Pubkey,
    user_account: Pubkey,
    user_account_authority_account: Pubkey,
    fund_receiver_tickets_rent_exempt_account: Pubkey,
    fund_receiver_rent_exempt_account: Pubkey,
    config_account: Pubkey
) -> Instruction {
    let accounts_meta = vec![
        AccountMeta::new(lottery_account, false),
        AccountMeta::new(user_account, false),
        AccountMeta::new_readonly(user_account_authority_account, true),
        AccountMeta::new(fund_receiver_tickets_rent_exempt_account, false),
        AccountMeta::new(fund_receiver_rent_exempt_account, false),
        AccountMeta::new_readonly(config_account, false)
    ];

    let instruction_data = Instructions::WithdrawAndCloseSucceedUser;

    Instruction::new_with_borsh(
        LOTTERY_PROGRAM_ID,
        &instruction_data,
        accounts_meta
    )
}

pub fn instruction_withdraw_failed_lottery(
    config_account: Pubkey,
    lottery_account: Pubkey,
    lottery_authority_account: Pubkey,
    usdc_mint_account: Pubkey,
    arbitrary_mint_account: Pubkey,
    lottery_associated_usdc_token_account: Pubkey,
    lottery_associated_arbitrary_token_account: Pubkey,
    fund_receiver_usdc_token_account: Pubkey,
    fund_receiver_arbitrary_token_account: Pubkey,
    fund_receiver_refunded_rent_exempt: Pubkey,
    standard_token_program_account: Pubkey
) -> Instruction {
    let accounts_meta = vec![
        AccountMeta::new_readonly(config_account, false),
        AccountMeta::new(lottery_account, false),
        AccountMeta::new_readonly(lottery_authority_account, true),
        AccountMeta::new_readonly(usdc_mint_account, false),
        AccountMeta::new_readonly(arbitrary_mint_account, false),
        AccountMeta::new(lottery_associated_usdc_token_account, false),
        AccountMeta::new(lottery_associated_arbitrary_token_account, false),
        AccountMeta::new(fund_receiver_usdc_token_account, false),
        AccountMeta::new(fund_receiver_arbitrary_token_account, false),
        AccountMeta::new(fund_receiver_refunded_rent_exempt, false),
        AccountMeta::new_readonly(standard_token_program_account, false)
    ];

    let instruction_data = Instructions::WithdrawFailedLottery;

    Instruction::new_with_borsh(
        LOTTERY_PROGRAM_ID,
        &instruction_data,
        accounts_meta
    )
}

pub fn instruction_withdraw_and_close_failed_user(
    config_account: Pubkey,
    lottery_account: Pubkey,
    user_account: Pubkey,
    user_account_authority_account: Pubkey,
    usdc_mint_account: Pubkey,
    lottery_associated_usdc_token_account: Pubkey,
    fund_receiver_usdc_token_account: Pubkey,
    fund_receiver_tickets_rent_exempt_account: Pubkey,
    fund_receiver_rent_exempt_account: Pubkey,
    standard_token_program_account: Pubkey
) -> Instruction {
    let accounts_meta = vec![
        AccountMeta::new_readonly(config_account, false),
        AccountMeta::new(lottery_account, false),
        AccountMeta::new(user_account, false),
        AccountMeta::new_readonly(user_account_authority_account, true),
        AccountMeta::new_readonly(usdc_mint_account, false),
        AccountMeta::new(lottery_associated_usdc_token_account, false),
        AccountMeta::new(fund_receiver_usdc_token_account, false),
        AccountMeta::new(fund_receiver_tickets_rent_exempt_account, false),
        AccountMeta::new(fund_receiver_rent_exempt_account, false),
        AccountMeta::new_readonly(standard_token_program_account, false)
    ];

    let instruction_data = Instructions::WithdrawAndCloseFailedUser;

    Instruction::new_with_borsh(
        LOTTERY_PROGRAM_ID,
        &instruction_data,
        accounts_meta
    )
}

pub fn instruction_close_lottery_account_and_usdc_token_account(
    config_account: Pubkey,
    lottery_account: Pubkey,
    lottery_account_authority_account: Pubkey,
    usdc_mint_account: Pubkey,
    lottery_associated_usdc_token_account: Pubkey,
    fund_receiver_usdc_token_account: Pubkey,
    fund_receiver_rent_exempt_account: Pubkey,
    standard_token_program_account: Pubkey
) -> Instruction {
    let accounts_meta = vec![
        AccountMeta::new_readonly(config_account, false),
        AccountMeta::new(lottery_account, false),
        AccountMeta::new_readonly(lottery_account_authority_account, true),
        AccountMeta::new_readonly(usdc_mint_account, false),
        AccountMeta::new(lottery_associated_usdc_token_account, false),
        AccountMeta::new(fund_receiver_usdc_token_account, false),
        AccountMeta::new(fund_receiver_rent_exempt_account, false),
        AccountMeta::new_readonly(standard_token_program_account, false)
    ];

    let instruction_data = Instructions::CloseLotteryAccountAndUsdcTokenAccount;

    Instruction::new_with_borsh(
        LOTTERY_PROGRAM_ID,
        &instruction_data,
        accounts_meta
    )
}