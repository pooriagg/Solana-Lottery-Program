use {
    crate::{
        error::LotteryError,
        instruction::Instructions,
        state::{
            Config,
            Lottery,
            LotteryState,
            User,
            CONFIG_ACCOUNT_SEED,
            LOTTERY_ACCOUNT_SEED,
            USER_ACCOUNT_SEED
        },
        types::*
    },

    borsh::{
        BorshDeserialize,
        BorshSerialize
    },

    pyth_solana_receiver_sdk::price_update::{
        get_feed_id_from_hex,
        FeedId,
        Price,
        PriceUpdateV2,
        VerificationLevel
    },

    solana_program::{
        account_info::{
            next_account_info,
            next_account_infos,
            AccountInfo
        },
        entrypoint::ProgramResult,
        hash::{
            hash as sha256,
            HASH_BYTES
        },
        log::sol_log,
        program::{
            invoke,
            invoke_signed
        },
        program_error::ProgramError,
        program_memory::{
            sol_memcmp,
            sol_memcpy
        },
        program_pack::Pack,
        pubkey::Pubkey,
        pubkey,
        system_instruction::{
            transfer as transfer_lamports,
            allocate as allocate_memory,
            assign as assign_new_owner
        },
        sysvar::{
            Sysvar,
            clock::Clock,
            rent::Rent,
        },
        system_program::{
            check_id,
            ID as SYSTEM_PROGRAM_ID
        }
    },

    spl_associated_token_account::{
        get_associated_token_address,
        instruction::create_associated_token_account_idempotent,
        ID as ASSOCIATED_TOKEN_PROGRAM_ID
    },

    spl_token::{
        instruction::{
            transfer_checked as transfer_spl_checked,
            close_account as close_token_account
        },
        state::{
            Account as TokenAccount,
            Mint as MintAccount
        }
    },
    
    std::mem::size_of
};

// Initial authority
// - mainnet and devnet
#[cfg(feature = "onchain_authority")]
const CONFIG_ACCOUNT_INITIAL_AUTHORITY: Pubkey = pubkey!("H2GkqgpqjQhkyeK9PKmXBFf3qX8cKc3TYCAFhTT5Tuvv");
// - local testing
#[cfg(not(feature = "onchain_authority"))]
const CONFIG_ACCOUNT_INITIAL_AUTHORITY: Pubkey = pubkey!("3BA57XksXaU5oqej6YW7a4nRqzgX9fjLTKvxcjKtfQ5n");

pub struct Processor {}
impl Processor {
    pub fn process_create_and_initialize_program_config_account(
        accounts_info: &[AccountInfo],
        program_id: &Pubkey,
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
    ) -> ProgramResult {
        let accounts_info = &mut accounts_info.iter();

        let config_account_authority_account_info = next_account_info(accounts_info)?;
        let fund_account_info = next_account_info(accounts_info)?;
        let config_global_account_info = next_account_info(accounts_info)?;
        let system_program_account_info = next_account_info(accounts_info)?;

        check_system_program_id(system_program_account_info.key)?;

        check_account_is_signer(config_account_authority_account_info)?;

        check_max_numbers_of_winner(&maximum_number_of_winners)?;

        check_max_price_feed_age(&maximum_time_of_price_feed_age)?;

        check_accounts_key_to_be_identical(
            config_account_authority_account_info.key,
            &CONFIG_ACCOUNT_INITIAL_AUTHORITY,
            LotteryError::InvalidConfigAuthority.into()
        )?;

        // validate fee_per_ticket
        Config::validate_fee_per_ticket(&lottery_tickets_fee)?;

        let (
            config_pda_addr,
            config_pda_canonical_bump
        ) = Pubkey::try_find_program_address(
            &[
                CONFIG_ACCOUNT_SEED.as_bytes()
            ],
            program_id
        ).ok_or::<ProgramError>(LotteryError::FailedToFindProgramAddress.into())?;

        check_accounts_key_to_be_identical(
            config_global_account_info.key,
            &config_pda_addr,
            ProgramError::InvalidSeeds
        )?;

        // handle config account
        let config_account = Config::new(
            config_pda_canonical_bump,
            authority,
            lottery_creation_fee,
            lottery_tickets_fee,
            maximum_number_of_winners,
            pyth_price_receiver_programid,
            usdc_mint_account,
            maximum_time_of_price_feed_age,
            minimum_tickets_to_be_sold_in_lottery,
            pyth_price_feed_accounts,
            maximum_time_for_lottery_account,
            treasury,
            pyth_price_feed_ids,
            max_lottery_description_bytes
        )?;

        let data_size = config_account
            .try_to_vec()
            .unwrap()
            .len();

        // create the config-account
        let seeds: &[&[u8]] = &[
            CONFIG_ACCOUNT_SEED.as_bytes(),
            &[ config_pda_canonical_bump ]
        ];
        create_pda_account(
            config_global_account_info,
            fund_account_info,
            data_size,
            program_id,
            seeds
        )?;
        sol_log("Config account created.");

        config_account.serialize(
            &mut &mut config_global_account_info.data.try_borrow_mut().unwrap()[..]
        )?;
        sol_log("Config account initialized.");
        
        Ok(())
    }

    pub fn process_create_and_initialize_lottery_account(
        program_id: &Pubkey,
        accounts_info: &[AccountInfo],
        fund_amount: u64,
        winners_count: u8,
        starting_time: Time,
        ending_time: Time,
        minimum_tickets_amount_required_to_be_sold: u32,
        ticket_price: u64,
        maximum_number_of_tickets_per_user: Option<u32>,
        lottery_description: String
    ) -> ProgramResult {
        let accounts_info = &mut accounts_info.iter();

        let lottery_account_info = next_account_info(accounts_info)?;
        let lottery_account_authority_account_info = next_account_info(accounts_info)?;
        let funding_account_info = next_account_info(accounts_info)?;
        let usdc_mint_account_info = next_account_info(accounts_info)?;
        let lottery_associated_usdc_token_account_info = next_account_info(accounts_info)?;
        let funding_usdc_token_account_info = next_account_info(accounts_info)?;
        let arbitrary_mint_account_info = next_account_info(accounts_info)?;
        let lottery_associated_arbitrary_token_account_info = next_account_info(accounts_info)?;
        let funding_arbitrary_token_account_info = next_account_info(accounts_info)?;
        let standard_token_program_account_info = next_account_info(accounts_info)?;
        let associated_token_program_account_info = next_account_info(accounts_info)?;
        let system_program_account_info = next_account_info(accounts_info)?;
        let config_global_account_info = next_account_info(accounts_info)?;

        // We don't need this check BUT to be developer-friendly we performed this check.
        if associated_token_program_account_info.key != &ASSOCIATED_TOKEN_PROGRAM_ID {
            return Err(
                ProgramError::IncorrectProgramId
            );
        };

        check_system_program_id(system_program_account_info.key)?;

        check_account_is_signer(lottery_account_authority_account_info)?;

        check_account_is_raw(lottery_account_info)?;

        // validate config account
        Config::validate_config_account(config_global_account_info, program_id)?;

        let config_account = Config::deserialize(
            &mut &config_global_account_info.data.try_borrow().unwrap()[..]
        )?;

        // check lottery's description-length
        if lottery_description.len() > config_account.max_lottery_description_bytes as usize {
            return Err(
                LotteryError::MaxLotteryDescriptionBytesExceeded.into()
            );
        };

        // check is_pause flag
        config_account.check_is_pause()?;

        // handle lottery account
        if ending_time
            .checked_sub(starting_time)
            .ok_or::<ProgramError>(LotteryError::Overflow.into())? > (config_account.maximum_time_for_lottery_account as i64)
        {   
            return Err(
                LotteryError::MaximumTimeExceed.into()
            );
        };

        let (
            lottery_pda_addr,
            lottery_pda_canonical_bump
        ) = Pubkey::try_find_program_address(
            &[
                LOTTERY_ACCOUNT_SEED.as_bytes(),
                lottery_account_authority_account_info.key.to_bytes().as_slice(),
                get_lottery_literal_seed(&lottery_description).as_slice()
            ],
            program_id
        ).ok_or::<ProgramError>(LotteryError::FailedToFindProgramAddress.into())?;
        
        check_accounts_key_to_be_identical(
            &lottery_pda_addr,
            lottery_account_info.key,
            ProgramError::InvalidSeeds
        )?;

        if !(
            winners_count > 0 &&
            winners_count <= config_account.maximum_number_of_winners
        ) {
            return Err(
                LotteryError::InvalidWinnersAmount.into()
            );
        };

        if minimum_tickets_amount_required_to_be_sold < (config_account.minimum_tickets_to_be_sold_in_lottery as u32) {
            return Err(
                LotteryError::InvalidMinimumAmountTickets.into()
            );
        };

        if (winners_count as u32) > minimum_tickets_amount_required_to_be_sold {
            return Err(
                LotteryError::InvalidMinTicketsReqAndWinnersAmount.into()
            );
        };

        let current_time = (Clock::get()?).unix_timestamp;

        if !(
            starting_time > current_time &&
            ending_time > starting_time
        ) {
            return Err(
                LotteryError::InvalidTime.into()
            );
        };

        if fund_amount == 0 {
            return Err(
                LotteryError::InvalidFundAmount.into()
            );
        };

        if fund_amount % (winners_count as u64) != 0 {
            return Err(
                LotteryError::WinnersAndFundAmountMismatch.into()
            );
        };

        let mut lottery_account = Lottery::new(
            lottery_pda_canonical_bump,
            fund_amount,
            config_account.lottery_creation_fee,
            winners_count,
            starting_time,
            ending_time,
            current_time,
            minimum_tickets_amount_required_to_be_sold,
            ticket_price,
            *arbitrary_mint_account_info.key,
            *lottery_account_authority_account_info.key,
            maximum_number_of_tickets_per_user,
            lottery_description.clone()
        );

        let data_size = lottery_account
            .try_to_vec()
            .unwrap()
            .len();

        let space_needed_per_winner = size_of::<Pubkey>() + size_of::<bool>();
        let total_space_needed_for_winners = winners_count as usize * space_needed_per_winner;

        let total_data_size = data_size + total_space_needed_for_winners;

        // create the lottery-account
        let seeds: &[&[u8]] = &[
            LOTTERY_ACCOUNT_SEED.as_bytes(),
            &lottery_account_authority_account_info.key.to_bytes(),
            &get_lottery_literal_seed(&lottery_description),
            &[ lottery_pda_canonical_bump ]
        ];
        create_pda_account(
            lottery_account_info,
            funding_account_info,
            total_data_size,
            program_id,
            seeds
        )?;
        sol_log("Lottery account created.");

        lottery_account.initial_bytes = total_data_size
            .try_into()
            .unwrap();

        lottery_account.serialize(
            &mut &mut lottery_account_info.data.try_borrow_mut().unwrap()[..]
        )?;
        sol_log("Lottery account initialized.");

        // handle usdc ata
        //  create lottery's associated usdc token account
        check_accounts_key_to_be_identical(
            usdc_mint_account_info.key,
            &config_account.usdc_mint_account,
            LotteryError::InvalidUsdcMintAccount.into()
        )?;

        //  validate lottery's arbitrary asscoiated token account
        check_accounts_key_to_be_identical(
            &get_associated_token_address(
                lottery_account_info.key,
                usdc_mint_account_info.key
            ),
            lottery_associated_usdc_token_account_info.key,
            LotteryError::InvalidLotteryAssociatedUsdcTokenAccount.into()
        )?;

        invoke(
            &create_associated_token_account_idempotent(
                funding_account_info.key,
                lottery_account_info.key,
                usdc_mint_account_info.key,
                standard_token_program_account_info.key
            ),
            &[
                funding_account_info.clone(),
                lottery_associated_usdc_token_account_info.clone(),
                lottery_account_info.clone(),
                usdc_mint_account_info.clone(),
                system_program_account_info.clone(),
                standard_token_program_account_info.clone()
            ]
        )?;
        sol_log("Lottery's usdc token account activated.");

        //  transfer 'lottery-creation-fee' to the newly created usdc-associated-token-account
        let MintAccount { decimals, .. } = MintAccount::unpack(
            &usdc_mint_account_info.data.try_borrow().unwrap()
        )?;

        invoke(
            &transfer_spl_checked(
                standard_token_program_account_info.key,
                funding_usdc_token_account_info.key,
                usdc_mint_account_info.key,
                lottery_associated_usdc_token_account_info.key,
                funding_account_info.key,
                &[],
                config_account.lottery_creation_fee,
                decimals
            )?,
            &[
                funding_usdc_token_account_info.clone(),
                usdc_mint_account_info.clone(),
                lottery_associated_usdc_token_account_info.clone(),
                funding_account_info.clone()
            ]
        )?;
        sol_log("Lottery's creation-fee transfered.");

        // handle arbitrary ata
        //  validate lottery's arbitrary asscoiated token account
        check_accounts_key_to_be_identical(
            &get_associated_token_address(
                lottery_account_info.key,
                arbitrary_mint_account_info.key
            ),
            lottery_associated_arbitrary_token_account_info.key,
            LotteryError::InvalidLotteryArbitraryAssociatedTokenAccount.into()
        )?;

        //  handling lottery's arbitrary associated token account
        invoke(
            &create_associated_token_account_idempotent(
                funding_account_info.key,
                lottery_account_info.key,
                arbitrary_mint_account_info.key,
                standard_token_program_account_info.key
            ),
            &[
                funding_account_info.clone(),
                lottery_associated_arbitrary_token_account_info.clone(),
                lottery_account_info.clone(),
                arbitrary_mint_account_info.clone(),
                system_program_account_info.clone(),
                standard_token_program_account_info.clone()
            ]
        )?;
        sol_log("Lottery's arbitrary token account activated.");

        //  transfer 'lottery-creation-fee' to the newly created usdc-associated-token-account
        let MintAccount { decimals, .. } = MintAccount::unpack(
            &arbitrary_mint_account_info.data.try_borrow().unwrap()
        )?;

        invoke(
            &transfer_spl_checked(
                standard_token_program_account_info.key,
                funding_arbitrary_token_account_info.key,
                arbitrary_mint_account_info.key,
                lottery_associated_arbitrary_token_account_info.key,
                funding_account_info.key,
                &[],
                fund_amount,
                decimals
            )?,
            &[
                funding_arbitrary_token_account_info.clone(),
                arbitrary_mint_account_info.clone(),
                lottery_associated_arbitrary_token_account_info.clone(),
                funding_account_info.clone()
            ]
        )?;
        sol_log("Funds transfered.");

        Ok(())
    }

    pub fn process_create_and_initialize_user_account(
        program_id: &Pubkey,
        accounts_info: &[AccountInfo]
    ) -> ProgramResult {
        let accounts_info = &mut accounts_info.iter();

        let user_account_info = next_account_info(accounts_info)?;
        let user_account_authority_account_info = next_account_info(accounts_info)?;
        let funding_account_info = next_account_info(accounts_info)?;
        let lottery_account_info = next_account_info(accounts_info)?;
        let system_program_account_info = next_account_info(accounts_info)?;
        let config_global_account_info = next_account_info(accounts_info)?;

        check_system_program_id(system_program_account_info.key)?;

        check_account_is_signer(user_account_authority_account_info)?;

        check_account_is_raw(user_account_info)?;

        // validate config account
        Config::validate_config_account(
            config_global_account_info,
            program_id
        )?;

        // check is_pause flag
        Config::check_is_pause_raw(config_global_account_info)?;

        // validate lottery account
        Lottery::validate_lottery_account(lottery_account_info, program_id)?;

        let lottery_account = Lottery::deserialize(
            &mut &lottery_account_info.data.try_borrow().unwrap()[..]
        )?;

        let current_time = (Clock::get()?).unix_timestamp;
        
        if lottery_account.is_started_and_not_ended(current_time) == false {
            return Err(
                LotteryError::InvalidLotteryState.into()
            );
        };

        // handle user account
        let (
            user_account_pda_addr,
            user_account_pda_canonical_bump
        ) = Pubkey::try_find_program_address(
            &[
                USER_ACCOUNT_SEED.as_bytes(),
                user_account_authority_account_info.key.to_bytes().as_slice(),
                lottery_account_info.key.to_bytes().as_slice()
            ],
            program_id
        ).ok_or::<ProgramError>(LotteryError::FailedToFindProgramAddress.into())?;

        check_accounts_key_to_be_identical(
            &user_account_pda_addr,
            user_account_info.key,
            ProgramError::InvalidSeeds
        )?;

        // create the user-account
        let space = User::LEN;
        let seeds: &[&[u8]] = &[
            USER_ACCOUNT_SEED.as_bytes(),
            &user_account_authority_account_info.key.to_bytes(),
            &lottery_account_info.key.to_bytes(),
            &[ user_account_pda_canonical_bump ]
        ];
        create_pda_account(
            user_account_info,
            funding_account_info,
            space,
            program_id,
            seeds
        )?;
        sol_log("User account created.");

        let mut user_account = User::deserialize(
            &mut &user_account_info.data.try_borrow().unwrap()[..]
        )?;

        user_account.discriminator = User::get_discriminator();
        user_account.canonical_bump = user_account_pda_canonical_bump;
        user_account.lottery = *lottery_account_info.key;
        user_account.authority = *user_account_authority_account_info.key;
        user_account.created_at = current_time;

        user_account.serialize(
            &mut &mut user_account_info.data.try_borrow_mut().unwrap()[..]
        )?;

        sol_log("User account initialized.");

        Ok(())
    }

    pub fn process_buy_ticket(
        program_id: &Pubkey,
        accounts_info: &[AccountInfo],
        tickets_amount: u32,
        expected_token_price_per_ticket: u64
    ) -> ProgramResult {
        let accounts_info = &mut accounts_info.iter();

        let config_global_account_info = next_account_info(accounts_info)?;
        let user_account_info = next_account_info(accounts_info)?;
        let user_account_authority_account_info = next_account_info(accounts_info)?;
        let funding_account_info = next_account_info(accounts_info)?;
        let lottery_account_info = next_account_info(accounts_info)?;
        let lottery_usdc_associated_token_account_info = next_account_info(accounts_info)?;
        let funding_usdc_token_account_info = next_account_info(accounts_info)?;
        let usdc_mint_account_info = next_account_info(accounts_info)?;
        let system_program_account_info = next_account_info(accounts_info)?;
        let standard_token_program_account_info = next_account_info(accounts_info)?;

        check_system_program_id(system_program_account_info.key)?;

        check_account_is_signer(user_account_authority_account_info)?;

        Lottery::check_max_tickets_per_instruction(tickets_amount)?;

        let user_account = User::deserialize(
            &mut &user_account_info.data.try_borrow().unwrap()[..]
        )?;

        if tickets_amount == 0 {
            return Err(
                LotteryError::InvalidTicketAmount.into()
            );
        };

        let current_time = (Clock::get()?).unix_timestamp;

        // handle config account
        Config::validate_config_account(config_global_account_info, program_id)?;

        let config_account = Config::deserialize(
            &mut &config_global_account_info.data.try_borrow().unwrap()[..]
        )?;

        // check is_pause flag
        config_account.check_is_pause()?;

        check_accounts_key_to_be_identical(
            usdc_mint_account_info.key,
            &config_account.usdc_mint_account,
            LotteryError::InvalidUsdcMintAccount.into()
        )?;
        
        // handle lottery account
        Lottery::validate_lottery_account(lottery_account_info, program_id)?;

        let lottery_account = Lottery::deserialize(
            &mut &lottery_account_info.data.try_borrow().unwrap()[..]
        )?;

        // protecting the user against front-running
        if expected_token_price_per_ticket != lottery_account.ticket_price {
            return Err(
                LotteryError::ExpectedTicketPriceViolated.into()
            );
        };

        if lottery_account.is_started_and_not_ended(current_time) == false {
            return Err(
                LotteryError::InvalidLotteryState.into()
            );
        };

        if user_account.authority == lottery_account.authority {
            return Err(
                LotteryError::InvalidUser.into()
            );
        };

        check_accounts_key_to_be_identical(
            &get_associated_token_address(
                lottery_account_info.key,
                usdc_mint_account_info.key
            ),
            lottery_usdc_associated_token_account_info.key,
            LotteryError::InvalidLotteryAssociatedUsdcTokenAccount.into()
        )?;

        // validate user account pda
        User::validate_user_account(
            user_account_info,
            program_id,
            lottery_account_info.key,
            user_account_authority_account_info.key
        )?;

        // validate user's holding tickets amount
        user_account.validate_user_holding_tickets_amount(
            &lottery_account.maximum_number_of_tickets_per_user,
            tickets_amount
        )?; 

        let old_total_tickets_acquired = user_account.total_tickets_acquired;
        let new_total_tickets_acquired = old_total_tickets_acquired
            .checked_add(tickets_amount)
            .ok_or::<ProgramError>(LotteryError::Overflow.into())?;
        // update user_accounts's total_tickets_acquired field
        sol_memcpy(
            user_account_info
                .data
                .try_borrow_mut()
                .unwrap()
                .get_mut(89..93)
                .unwrap(),
            new_total_tickets_acquired.to_le_bytes().as_slice(),
            size_of::<u32>()
        );

        //  handle transfering fee & updating lottery accounts
        let total_tickets_price = calculate_fee_and_update_lottery_account(
            &config_account,
            &lottery_account,
            lottery_account_info,
            tickets_amount
        )?;

        let MintAccount { decimals, .. } = MintAccount::unpack(
            &usdc_mint_account_info.data.try_borrow().unwrap()
        )?;

        let old_total_tickets_value = user_account.total_tickets_value;
        let new_total_tickets_value = old_total_tickets_value.checked_add(
            total_tickets_price
        ).ok_or::<ProgramError>(LotteryError::Overflow.into())?;
        // update user_accounts's total_tickets_value field
        sol_memcpy(
            user_account_info
                .data
                .try_borrow_mut()
                .unwrap()
                .get_mut(73..81)
                .unwrap(),
                new_total_tickets_value.to_le_bytes().as_slice(),
            size_of::<u64>()
        );

        invoke(
            &transfer_spl_checked(
                standard_token_program_account_info.key,
                funding_usdc_token_account_info.key,
                usdc_mint_account_info.key,
                lottery_usdc_associated_token_account_info.key,
                funding_account_info.key,
                &[],
                total_tickets_price,
                decimals
            )?,
            &[
                funding_usdc_token_account_info.clone(),
                usdc_mint_account_info.clone(),
                lottery_usdc_associated_token_account_info.clone(),
                funding_account_info.clone()
            ]
        )?;
        sol_log("Tickets total price in USDC transfered to the lottery.");

        let rent_sysvar_account = Rent::get()?;
        let space_needed = (tickets_amount as usize).checked_mul(pubkey::PUBKEY_BYTES).unwrap();
        let rent_exempt = (
            rent_sysvar_account.lamports_per_byte_year
                .checked_mul(space_needed as u64)
                .ok_or::<ProgramError>(LotteryError::Overflow.into())?
        ).checked_mul(rent_sysvar_account.exemption_threshold as u64).ok_or::<ProgramError>(LotteryError::Overflow.into())?;

        let old_total_rent_paid = user_account.total_rent_exempt_paied;
        let new_total_rent_paied = old_total_rent_paid
            .checked_add(rent_exempt)
            .ok_or::<ProgramError>(LotteryError::Overflow.into())?;
        // update user_accounts's total_tickets_value field
        sol_memcpy(
            user_account_info
                .data
                .try_borrow_mut()
                .unwrap()
                .get_mut(81..89)
                .unwrap(),
                new_total_rent_paied.to_le_bytes().as_slice(),
            size_of::<u64>()
        );

        invoke(
            &transfer_lamports(
                funding_account_info.key,
                lottery_account_info.key,
                rent_exempt
            ),
            &[
                funding_account_info.clone(),
                lottery_account_info.clone()
            ]
        )?;
        sol_log("Rent-exempt lamports transfered to the lottery account.");

        let lottery_old_data_size = lottery_account_info.data_len();
        let lottery_new_data_size = lottery_old_data_size.checked_add(space_needed).unwrap();

        lottery_account_info
            .realloc(lottery_new_data_size, false)
            .map_err::<ProgramError, _>(|_| LotteryError::ReallocationFailed.into())?;

        // add tickets to the lottery account
        Lottery::add_ticket(
            lottery_account_info,
            lottery_account.initial_bytes,
            lottery_account.tickets_total_amount,
            tickets_amount,
            *user_account_info.key
        );

        solana_program::msg!(
            "Total-Tikcets => {} - Total-Tokens-Transfered => {} USDC",
            tickets_amount,
            spl_token::amount_to_ui_amount(total_tickets_price, decimals)
        );

        Ok(())
    }

    pub fn process_change_lottery_ticket_price(
        accounts_info: &[AccountInfo],
        program_id: &Pubkey,
        new_ticket_price: u64
    ) -> ProgramResult {
        let accounts_info = &mut accounts_info.iter();

        let lottery_account_info = next_account_info(accounts_info)?;
        let lottery_account_authority_account_info = next_account_info(accounts_info)?;
        let config_global_account_info = next_account_info(accounts_info)?;

        check_account_is_signer(lottery_account_authority_account_info)?;

        // validate config account
        Config::validate_config_account(
            config_global_account_info,
            program_id
        )?;

        // check is_pause flag
        Config::check_is_pause_raw(config_global_account_info)?;

        // validate lottery-account
        Lottery::validate_lottery_account(
            lottery_account_info,
            program_id
        )?;

        let lottery_account = Lottery::deserialize(
            &mut &lottery_account_info.data.try_borrow().unwrap()[..]
        )?;

        check_accounts_key_to_be_identical(
            &lottery_account.authority,
            lottery_account_authority_account_info.key,
            LotteryError::InvalidLotteryAccountAuthority.into()
        )?;

        let current_time = (Clock::get()?).unix_timestamp;
        
        if lottery_account.is_ended(current_time) == true {
            return Err(
                LotteryError::InvalidLotteryState.into()
            );
        };

        if lottery_account.ticket_price == new_ticket_price {
            return Err(
                LotteryError::InvalidTicketPrice.into()
            );
        };

        // update the lottery account
        sol_memcpy(
            lottery_account_info
                .data
                .try_borrow_mut()
                .unwrap()
                .get_mut(89..97)
                .unwrap(),
            &new_ticket_price.to_le_bytes(),
            std::mem::size_of::<u64>()
        );
        
        sol_log("Lottery's Ticket price has been updated.");

        Ok(())
    }

    pub fn process_end_lottery_and_pick_winners(
        program_id: &Pubkey,
        accounts_info: &[AccountInfo]
    ) -> ProgramResult {
        let accounts_info = &mut accounts_info.iter();

        let lottery_account_info = next_account_info(accounts_info)?;
        let config_global_account_info = next_account_info(accounts_info)?;
        let sol_price_feed_account_info = next_account_info(accounts_info)?;
        let btc_price_feed_account_info = next_account_info(accounts_info)?;
        let eth_price_feed_account_info = next_account_info(accounts_info)?;

        let current_time = (Clock::get()?).unix_timestamp;

        // validate config account
        Config::validate_config_account(config_global_account_info, program_id)?;

        let config_account = Config::deserialize(
            &mut &config_global_account_info.data.try_borrow().unwrap()[..]
        )?;

        // check is_pause flag
        config_account.check_is_pause()?;

        // validate price feed accounts
        config_account.validate_price_feed_accounts(
            sol_price_feed_account_info,
            btc_price_feed_account_info,
            eth_price_feed_account_info
        )?;

        // validate lottery account
        Lottery::validate_lottery_account(
            lottery_account_info,
            program_id
        )?;

        let mut lottery_account = Lottery::deserialize(
            &mut &lottery_account_info.data.try_borrow().unwrap()[..]
        )?;
        if lottery_account.is_ended_successfuly == true {
            return Err(
                LotteryError::LotteryAlreadyEnded.into()
            );
        };

        if lottery_account.get_lottery_state(current_time) != LotteryState::Successful {
            return Err(
                LotteryError::LotteryWasNotSuccessfull.into()
            );
        };

        // choose price feed account
        let selected_price_feed_index = (current_time % 3) as usize;
        let verification_level = VerificationLevel::Full;
        let price_feed_max_age = config_account.maximum_time_of_price_feed_age;

        let selected_price_feed_data = if selected_price_feed_index == 0 {
            get_price(
                sol_price_feed_account_info,
                verification_level,
                price_feed_max_age,
                &config_account.get_sol_price_feed_id(),
                &Clock::get()?
            )?
        } else if selected_price_feed_index == 1 {
            get_price(
                btc_price_feed_account_info,
                verification_level,
                price_feed_max_age,
                &config_account.get_btc_price_feed_id(),
                &Clock::get()?
            )?
        } else {
            get_price(
                eth_price_feed_account_info,
                verification_level,
                price_feed_max_age,
                &config_account.get_eth_price_feed_id(),
                &Clock::get()?
            )?
        };

        // generate SHA256-hash of the selected price
        let sha256_hash: [u8; 32] = sha256(
            selected_price_feed_data
                .price
                .to_le_bytes()
                .as_ref()
        ).to_bytes();

        // randomly select winners
        lottery_account.pick_winners(
            &sha256_hash,
            lottery_account_info
        )?;

        lottery_account.is_ended_successfuly = true;

        lottery_account.random_numbers_info = (
            if selected_price_feed_index == 0 {
                *sol_price_feed_account_info.key
            } else if selected_price_feed_index == 1 {
                *btc_price_feed_account_info.key
            } else {
                *eth_price_feed_account_info.key
            },
            selected_price_feed_data.publish_time,
            selected_price_feed_data.price
        );

        lottery_account.serialize(
            &mut &mut lottery_account_info.data.try_borrow_mut().unwrap()[..]
        )?;

        sol_log("Lottery account updated.");

        Ok(())
    }

    pub fn process_withdraw_succeed_lottery(
        program_id: &Pubkey,
        accounts_info: &[AccountInfo]
    ) -> ProgramResult {
        let accounts_info = &mut accounts_info.iter();

        let lottery_account_info = next_account_info(accounts_info)?;
        let config_global_account_info = next_account_info(accounts_info)?;
        let lottery_authority_account_info = next_account_info(accounts_info)?;
        let lottery_associated_usdc_token_account_info = next_account_info(accounts_info)?;
        let fund_receiver_usdc_token_account_info = next_account_info(accounts_info)?;
        let usdc_mint_account_info = next_account_info(accounts_info)?;
        let standard_token_program_account_info = next_account_info(accounts_info)?;

        check_account_is_signer(lottery_authority_account_info)?;

        let current_time = (Clock::get()?).unix_timestamp;

        let lottery_account = Lottery::deserialize(
            &mut &lottery_account_info.data.try_borrow().unwrap()[..]
        )?;

        // validate config account
        Config::validate_config_account(config_global_account_info, program_id)?;

        // check is_pause flag
        Config::check_is_pause_raw(config_global_account_info)?;

        // validate usdc mint account
        compare_usdc_mint_account_with_config_global_account_info(
            config_global_account_info,
            usdc_mint_account_info.key
        )?;

        // validate lottery account
        Lottery::validate_lottery_account(lottery_account_info, program_id)?;
        // validate lottery state
        if lottery_account.get_lottery_state(current_time) != LotteryState::Successful {
            return Err(
                LotteryError::LotteryWasNotSuccessfull.into()
            );
        };

        if lottery_account.is_creator_withdrawed_when_lottery_was_successful == true {
            return Err(
                LotteryError::FundsAlreadyWithdrawed.into()
            );
        };
        
        if lottery_account.is_ended_successfuly == false {
            return Err(
                LotteryError::WinnersNotSelected.into()
            );
        };
        // validate lottery authority
        check_accounts_key_to_be_identical(
            &lottery_account.authority,
            lottery_authority_account_info.key,
            LotteryError::InvalidLotteryAccountAuthority.into()
        )?;

        // validate lottery associated usdc token account 
        check_accounts_key_to_be_identical(
            &get_associated_token_address(
                lottery_account_info.key,
                usdc_mint_account_info.key
            ),
            lottery_associated_usdc_token_account_info.key,
            LotteryError::InvalidLotteryAssociatedUsdcTokenAccount.into()
        )?;

        let MintAccount { decimals, .. } = MintAccount::unpack(
            &usdc_mint_account_info.data.try_borrow().unwrap()
        )?;

        let TokenAccount { amount, .. } = TokenAccount::unpack(
            &lottery_associated_usdc_token_account_info.data.try_borrow().unwrap()
        )?; 

        let total_protocol_fee_per_ticket = lottery_account.protocol_fee;
        let lottery_creation_fee = lottery_account.lottery_creation_fee;
        let total_protocol_fee = total_protocol_fee_per_ticket.checked_add(
            lottery_creation_fee
        ).ok_or::<ProgramError>(LotteryError::Overflow.into())?;

        let usdc_to_withdraw = amount
            .checked_sub(total_protocol_fee)
            .ok_or::<ProgramError>(LotteryError::Overflow.into())?;

        invoke_signed(
            &transfer_spl_checked(
                standard_token_program_account_info.key,
                lottery_associated_usdc_token_account_info.key,
                usdc_mint_account_info.key,
                fund_receiver_usdc_token_account_info.key,
                lottery_account_info.key,
                &[],
                usdc_to_withdraw,
                decimals
            )?,
            &[
                lottery_associated_usdc_token_account_info.clone(),
                usdc_mint_account_info.clone(),
                fund_receiver_usdc_token_account_info.clone(),
                lottery_account_info.clone()
            ],
            &[
                &[
                    LOTTERY_ACCOUNT_SEED.as_bytes(),
                    &lottery_authority_account_info.key.to_bytes(),
                    get_lottery_literal_seed(&lottery_account.lottery_description).as_slice(),
                    &[ lottery_account.canonical_bump ]
                ]
            ]
        )?;
        sol_log("Funds withdrawed successfully.");

        // update the lottery account
        let mut lottery_account_data = lottery_account_info
            .data
            .try_borrow_mut()
            .unwrap();

        let is_creator_withdrawed_when_lottery_was_successful = lottery_account_data.get_mut(142).unwrap();
        *is_creator_withdrawed_when_lottery_was_successful = true as u8;
        // update the lottery account

        Ok(())
    }

    pub fn process_withdraw_lottery_winners(
        program_id: &Pubkey,
        accounts_info: &[AccountInfo]
    ) -> ProgramResult {
        let accounts_info = &mut accounts_info.iter();

        let lottery_account_info = next_account_info(accounts_info)?;
        let user_account_info = next_account_info(accounts_info)?;
        let user_account_authority_account_info = next_account_info(accounts_info)?;
        let lottery_associated_arbitrary_token_account_info = next_account_info(accounts_info)?;
        let fund_receiver_arbitrary_token_account_info = next_account_info(accounts_info)?;
        let arbitrary_mint_account_info = next_account_info(accounts_info)?;
        let standard_token_program_account_info = next_account_info(accounts_info)?;
        let config_global_account_info = next_account_info(accounts_info)?;

        check_account_is_signer(user_account_authority_account_info)?;

        // validate config account
        Config::validate_config_account(
            config_global_account_info,
            program_id
        )?;

        // check is_pause flag
        Config::check_is_pause_raw(config_global_account_info)?;

        let current_time = (Clock::get()?).unix_timestamp;

        let mut lottery_account = Lottery::deserialize(
            &mut &lottery_account_info.data.try_borrow().unwrap()[..]
        )?;

        // validate user account
        User::validate_user_account(
            user_account_info,
            program_id,
            lottery_account_info.key,
            user_account_authority_account_info.key
        )?;

        // validate lottery account
        Lottery::validate_lottery_account(lottery_account_info, program_id)?;
        // validate lottery state
        if lottery_account.get_lottery_state(current_time) != LotteryState::Successful {
            return Err(
                LotteryError::LotteryWasNotSuccessfull.into()
            );
        };
        if lottery_account.is_ended_successfuly == false {
            return Err(
                LotteryError::WinnersNotSelected.into()
            );
        };

        // validate lottery arbitrary mint account
        check_accounts_key_to_be_identical(
            arbitrary_mint_account_info.key,
            &lottery_account.arbitrary_mint_account_address,
            LotteryError::InvalidArbitraryMintAccount.into()
        )?;

        // validate lottery associated arbitrary token account
        check_accounts_key_to_be_identical(
            &get_associated_token_address(
                lottery_account_info.key,
                arbitrary_mint_account_info.key
            ),
            lottery_associated_arbitrary_token_account_info.key,
            LotteryError::InvalidLotteryArbitraryAssociatedTokenAccount.into()
        )?;

        // validate user_account as winner 
        let w_count = lottery_account.get_winner_info(user_account_info.key)?;
        let arbitrary_token_per_winner = lottery_account.fund_amount / (lottery_account.winners_count as u64);
        let tokens_amount_to_transfer = (w_count as u64) * arbitrary_token_per_winner;

        let MintAccount { decimals, .. } = MintAccount::unpack(
            &arbitrary_mint_account_info.data.try_borrow().unwrap()
        )?;

        // transfer funds
        invoke_signed(
            &transfer_spl_checked(
                standard_token_program_account_info.key,
                lottery_associated_arbitrary_token_account_info.key,
                arbitrary_mint_account_info.key,
                fund_receiver_arbitrary_token_account_info.key,
                lottery_account_info.key,
                &[], 
                tokens_amount_to_transfer,
                decimals
            )?,
            &[
                lottery_associated_arbitrary_token_account_info.clone(),
                arbitrary_mint_account_info.clone(),
                fund_receiver_arbitrary_token_account_info.clone(),
                lottery_account_info.clone()
            ],
            &[
                &[
                    LOTTERY_ACCOUNT_SEED.as_bytes(),
                    &lottery_account.authority.to_bytes(),
                    get_lottery_literal_seed(&lottery_account.lottery_description).as_slice(),
                    &[ lottery_account.canonical_bump ]
                ]
            ]
        )?;
        sol_log("Congratulation! funds withdrawed successfully.");

        lottery_account.serialize(
            &mut &mut lottery_account_info.data.try_borrow_mut().unwrap()[..]
        )?;

        Ok(())
    }

    pub fn process_withdraw_and_close_succeed_user(
        program_id: &Pubkey,
        accounts_info: &[AccountInfo]
    ) -> ProgramResult {
        let accounts_info = &mut accounts_info.iter();

        let lottery_account_info = next_account_info(accounts_info)?;
        let user_account_info = next_account_info(accounts_info)?;
        let user_account_authority_account_info = next_account_info(accounts_info)?;
        let fund_receiver_tickets_rent_exempt_lamports_account_info = next_account_info(accounts_info)?;
        let fund_receiver_user_account_rent_exempt_lamports_account_info = next_account_info(accounts_info)?;
        let config_global_account_info = next_account_info(accounts_info)?;

        check_account_is_signer(user_account_authority_account_info)?;

        // validate config account
        Config::validate_config_account(
            config_global_account_info, 
            program_id
        )?;

        // check protocol state
        Config::check_is_pause_raw(config_global_account_info)?;

        // validate lottery account
        Lottery::validate_lottery_account(
            lottery_account_info, 
            program_id
        )?;

        // validate user account
        User::validate_user_account(
            &user_account_info, 
            program_id, 
            lottery_account_info.key, 
            user_account_authority_account_info.key
        )?;

        // validate lottery account state
        let lottery_account = Lottery::deserialize(
            &mut &lottery_account_info.data.try_borrow().unwrap()[..]
        )?;

        let current_time = (Clock::get()?).unix_timestamp;
        if lottery_account.get_lottery_state(current_time) != LotteryState::Successful {
            return Err(
                LotteryError::InvalidLotteryState.into()
            );
        };

        if lottery_account.is_ended_successfuly == false {
            return Err(
                LotteryError::WinnersNotSelected.into()
            );
        };

        // validate that user in not one of the winners
        for (winner, _) in lottery_account.winners.iter() {
            if user_account_info.key == winner {
                return Err(
                    LotteryError::UserIsOneOfTheWinners.into()
                );
            };
        };

        let user_account = User::deserialize(
            &mut &user_account_info.data.try_borrow().unwrap()[..]
        )?;

        if user_account.total_tickets_acquired > 0 {
            // reduce lottery account size
            let reduce_size = user_account.total_tickets_acquired
                .checked_mul(pubkey::PUBKEY_BYTES as u32)
                .ok_or::<ProgramError>(LotteryError::Overflow.into())? as usize;

            let lottery_account_old_data_size = lottery_account_info.data_len();
            let lottery_account_new_data_size = lottery_account_old_data_size
                .checked_sub(reduce_size)
                .ok_or::<ProgramError>(LotteryError::Overflow.into())?;

            lottery_account_info.realloc(
                lottery_account_new_data_size,
                false
            )?;
            sol_log("Lottery account data size reduced.");

            // transfer tickets_rent_exempt lamports to the fund_receiver account
            let lottery_account_old_balance = lottery_account_info.lamports();
            let fund_receiver_tickets_rent_exempt_account_old_balance = fund_receiver_tickets_rent_exempt_lamports_account_info.lamports();

            **lottery_account_info.try_borrow_mut_lamports()? = (lottery_account_old_balance)
                .checked_sub(user_account.total_rent_exempt_paied)
                .ok_or::<ProgramError>(LotteryError::Overflow.into())?;

            **fund_receiver_tickets_rent_exempt_lamports_account_info.try_borrow_mut_lamports()? = (fund_receiver_tickets_rent_exempt_account_old_balance)
                .checked_add(user_account.total_rent_exempt_paied)
                .ok_or::<ProgramError>(LotteryError::Overflow.into())?;
            
            sol_log("Tickets rent-exempt lamprots transfered.");
        };

        // close user account
        User::close_user_account(
            &user_account_info,
            fund_receiver_user_account_rent_exempt_lamports_account_info
        )?;
        sol_log("Succeed user account closed.");

        Ok(())
    }

    pub fn process_withdraw_failed_lottery(
        program_id: &Pubkey,
        accounts_info: &[AccountInfo]
    ) -> ProgramResult {
        let accounts_info = &mut accounts_info.iter();

        let config_global_account_info = next_account_info(accounts_info)?;
        let lottery_account_info = next_account_info(accounts_info)?;
        let lottery_authority_account_info = next_account_info(accounts_info)?;
        let usdc_mint_account_info = next_account_info(accounts_info)?;
        let arbitrary_mint_account_info = next_account_info(accounts_info)?;
        let lottery_associated_usdc_token_account_info = next_account_info(accounts_info)?;
        let lottery_associated_arbitrary_token_account_info = next_account_info(accounts_info)?;
        let fund_receiver_usdc_token_account_info = next_account_info(accounts_info)?;
        let fund_receiver_arbitrary_token_account_info = next_account_info(accounts_info)?;
        let fund_receiver_refunded_rent_exempt = next_account_info(accounts_info)?;
        let standard_token_program_account_info = next_account_info(accounts_info)?;

        check_account_is_signer(lottery_authority_account_info)?;

        // validate config account
        Config::validate_config_account(
            config_global_account_info,
            program_id
        )?;

        // check is_pause flag
        Config::check_is_pause_raw(config_global_account_info)?;
     
        // validate usdc mint account
        compare_usdc_mint_account_with_config_global_account_info(
            config_global_account_info,
            usdc_mint_account_info.key
        )?;

        // validate lottery account
        Lottery::validate_lottery_account(
            lottery_account_info,
            program_id
        )?;

        let lottery_account = Lottery::deserialize(
            &mut &lottery_account_info.data.try_borrow().unwrap()[..]
        )?;

        let current_time = (Clock::get()?).unix_timestamp;
        if lottery_account.get_lottery_state(current_time) != LotteryState::Failed {
            return Err(
                LotteryError::InvalidLotteryState.into()
            );
        };

        if lottery_account.is_creator_withdrawed_when_lottery_was_failed == true {
            return Err(
                LotteryError::FundsAlreadyWithdrawed.into()
            );
        };

        check_accounts_key_to_be_identical(
            &lottery_account.authority,
            lottery_authority_account_info.key,
            LotteryError::InvalidLotteryAccountAuthority.into()
        )?;

        check_accounts_key_to_be_identical(
            &lottery_account.arbitrary_mint_account_address,
            arbitrary_mint_account_info.key,
            LotteryError::InvalidArbitraryMintAccount.into()
        )?;

        check_accounts_key_to_be_identical(
            &get_associated_token_address(
                lottery_account_info.key,
                usdc_mint_account_info.key
            ),
            lottery_associated_usdc_token_account_info.key,
            LotteryError::InvalidLotteryAssociatedUsdcTokenAccount.into()
        )?;

        check_accounts_key_to_be_identical(
            &get_associated_token_address(
                lottery_account_info.key,
                arbitrary_mint_account_info.key
            ),
            lottery_associated_arbitrary_token_account_info.key,
            LotteryError::InvalidLotteryArbitraryAssociatedTokenAccount.into()
        )?;

        // transfer lottery's creation fee to the fund_receiver account
        let usdc_token_amount = lottery_account.lottery_creation_fee;
        let MintAccount { decimals: usdc_token_decimals, .. } = MintAccount::unpack(
            &usdc_mint_account_info.data.try_borrow().unwrap()
        )?;

        invoke_signed(
            &transfer_spl_checked(
                standard_token_program_account_info.key,
                lottery_associated_usdc_token_account_info.key,
                usdc_mint_account_info.key,
                fund_receiver_usdc_token_account_info.key,
                lottery_account_info.key,
                &[],
                usdc_token_amount,
                usdc_token_decimals
            )?,
            &[
                lottery_associated_usdc_token_account_info.clone(),
                usdc_mint_account_info.clone(),
                fund_receiver_usdc_token_account_info.clone(),
                lottery_account_info.clone()
            ],
            &[
                &[
                    LOTTERY_ACCOUNT_SEED.as_bytes(),
                    &lottery_authority_account_info.key.to_bytes(),
                    get_lottery_literal_seed(&lottery_account.lottery_description).as_slice(),
                    &[ lottery_account.canonical_bump ]
                ]
            ]
        )?;
        sol_log("Creation_fee refunded.");

        // transfer all tokens in associated_arbitrary_token_account to the fund_receiver
        let TokenAccount { amount: arbitrary_token_amount, .. } = TokenAccount::unpack(
            &lottery_associated_arbitrary_token_account_info.data.try_borrow().unwrap()
        )?;
        let MintAccount { decimals: arbitrary_token_decimals, .. } = MintAccount::unpack(
            &arbitrary_mint_account_info.data.try_borrow().unwrap()
        )?;

        invoke_signed(
            &transfer_spl_checked(
                standard_token_program_account_info.key,
                lottery_associated_arbitrary_token_account_info.key,
                arbitrary_mint_account_info.key,
                fund_receiver_arbitrary_token_account_info.key,
                lottery_account_info.key,
                &[],
                arbitrary_token_amount,
                arbitrary_token_decimals
            )?,
            &[
                lottery_associated_arbitrary_token_account_info.clone(),
                arbitrary_mint_account_info.clone(),
                fund_receiver_arbitrary_token_account_info.clone(),
                lottery_account_info.clone()
            ],
            &[
                &[
                    LOTTERY_ACCOUNT_SEED.as_bytes(),
                    &lottery_authority_account_info.key.to_bytes(),
                    get_lottery_literal_seed(&lottery_account.lottery_description).as_slice(),
                    &[ lottery_account.canonical_bump ]
                ]
            ]
        )?;
        sol_log("arbitrary_tokens refunded.");

        // close associated_arbitrary_token_account and send refund_lamports to the 
        invoke_signed(
            &close_token_account(
                standard_token_program_account_info.key,
                lottery_associated_arbitrary_token_account_info.key,
                fund_receiver_refunded_rent_exempt.key,
                lottery_account_info.key,
                &[]
            )?,
            &[
                lottery_associated_arbitrary_token_account_info.clone(),
                fund_receiver_refunded_rent_exempt.clone(),
                lottery_account_info.clone()
            ],
            &[
                &[
                    LOTTERY_ACCOUNT_SEED.as_bytes(),
                    &lottery_authority_account_info.key.to_bytes(),
                    get_lottery_literal_seed(&lottery_account.lottery_description).as_slice(),
                    &[ lottery_account.canonical_bump ]
                ]
            ]
        )?;
        sol_log("Arbitrary_token_account closed & rent_exempt_lamports refunded.");

        // update the lottery account
        let mut lottery_account_data = lottery_account_info
            .data
            .try_borrow_mut()
            .unwrap();

        let is_creator_withdrawed_when_lottery_was_failed = lottery_account_data.get_mut(143).unwrap();
        *is_creator_withdrawed_when_lottery_was_failed = true as u8;
        // update the lottery account

        Ok(())
    }

    pub fn process_withdraw_and_close_failed_user(
        program_id: &Pubkey,
        accounts_info: &[AccountInfo]
    ) -> ProgramResult {
        let accounts_info = &mut accounts_info.iter();

        let config_global_account_info = next_account_info(accounts_info)?;
        let lottery_account_info = next_account_info(accounts_info)?;
        let user_account_info = next_account_info(accounts_info)?;
        let user_account_authority_account_info = next_account_info(accounts_info)?;
        let usdc_mint_account_info = next_account_info(accounts_info)?;
        let lottery_associated_usdc_token_account_info = next_account_info(accounts_info)?;
        let fund_receiver_usdc_token_account_info = next_account_info(accounts_info)?;
        let fund_receiver_tickets_rent_exempt_account_info = next_account_info(accounts_info)?;
        let fund_receiver_rent_exempt_account_info = next_account_info(accounts_info)?;
        let standard_token_program_account_info = next_account_info(accounts_info)?;

        check_account_is_signer(user_account_authority_account_info)?;

        // validate config account
        Config::validate_config_account(
            config_global_account_info,
            program_id
        )?;

        // check is_pause flag
        Config::check_is_pause_raw(config_global_account_info)?;

        // validate usdc mint account
        compare_usdc_mint_account_with_config_global_account_info(
            config_global_account_info,
            usdc_mint_account_info.key
        )?;

        // validate user account
        User::validate_user_account(
            user_account_info,
            program_id,
            lottery_account_info.key,
            user_account_authority_account_info.key
        )?;

        let user_account = User::deserialize(
            &mut &user_account_info.data.try_borrow().unwrap()[..]
        )?;

        // validate lottery account
        Lottery::validate_lottery_account(
            lottery_account_info,
            program_id
        )?;

        let lottery_account = Lottery::deserialize(
            &mut &lottery_account_info.data.try_borrow().unwrap()[..]
        )?;

        let current_time = (Clock::get()?).unix_timestamp;
        if lottery_account.get_lottery_state(current_time) != LotteryState::Failed {
            return Err(
                LotteryError::InvalidLotteryState.into()
            );
        };

        check_accounts_key_to_be_identical(
            &get_associated_token_address(
                lottery_account_info.key,
                usdc_mint_account_info.key
            ),
            lottery_associated_usdc_token_account_info.key,
            LotteryError::InvalidLotteryAssociatedUsdcTokenAccount.into()
        )?;

        if user_account.total_tickets_acquired > 0 {
            // reduce lottery account size
            let reduce_size = user_account.total_tickets_acquired
                .checked_mul(pubkey::PUBKEY_BYTES as u32)
                .ok_or::<ProgramError>(LotteryError::Overflow.into())? as usize;

            let lottery_account_old_data_size = lottery_account_info.data_len();
            let lottery_account_new_data_size = lottery_account_old_data_size
                .checked_sub(reduce_size)
                .ok_or::<ProgramError>(LotteryError::Overflow.into())?;

            lottery_account_info.realloc(
                lottery_account_new_data_size,
                false
            )?;
            sol_log("Lottery account data size reduced.");

            // transfer user_account's usdc_tokens to the fund_receiver account
            let MintAccount { decimals, .. } = MintAccount::unpack(
                &usdc_mint_account_info.data.try_borrow().unwrap()
            )?;

            invoke_signed(
                &transfer_spl_checked(
                    standard_token_program_account_info.key,
                    lottery_associated_usdc_token_account_info.key,
                    usdc_mint_account_info.key,
                    fund_receiver_usdc_token_account_info.key,
                    lottery_account_info.key,
                    &[],
                    user_account.total_tickets_value,
                    decimals
                )?,
                &[
                    lottery_associated_usdc_token_account_info.clone(),
                    usdc_mint_account_info.clone(),
                    fund_receiver_usdc_token_account_info.clone(),
                    lottery_account_info.clone()
                ],
                &[
                    &[
                        LOTTERY_ACCOUNT_SEED.as_bytes(),
                        &lottery_account.authority.to_bytes(),
                        get_lottery_literal_seed(&lottery_account.lottery_description).as_slice(),
                        &[ lottery_account.canonical_bump ]
                    ]
                ]
            )?;
            sol_log("USDC tokens transfered.");

            // transfer tickets_rent_exempt lamports to the fund_receiver account
            let lottery_account_old_balance = lottery_account_info.lamports();
            let fund_receiver_tickets_rent_exempt_account_old_balance = fund_receiver_tickets_rent_exempt_account_info.lamports();

            **lottery_account_info.try_borrow_mut_lamports()? = (lottery_account_old_balance)
                .checked_sub(user_account.total_rent_exempt_paied)
                .ok_or::<ProgramError>(LotteryError::Overflow.into())?;

            **fund_receiver_tickets_rent_exempt_account_info.try_borrow_mut_lamports()? = (fund_receiver_tickets_rent_exempt_account_old_balance)
                .checked_add(user_account.total_rent_exempt_paied)
                .ok_or::<ProgramError>(LotteryError::Overflow.into())?;
            
            sol_log("Tickets rent-exempt lamprots transfered.");
        };

        // close user account
        User::close_user_account(
            user_account_info,
            fund_receiver_rent_exempt_account_info
        )?;
        sol_log("Failed user account closed.");

        Ok(())
    }

    pub fn process_close_lottery_account_and_usdc_token_account(
        program_id: &Pubkey,
        accounts_info: &[AccountInfo]
    ) -> ProgramResult {
        let accounts_info = &mut accounts_info.iter();

        let config_global_account_info = next_account_info(accounts_info)?;
        let lottery_account_info = next_account_info(accounts_info)?;
        let lottery_account_authority_account_info = next_account_info(accounts_info)?;
        let usdc_mint_account_info = next_account_info(accounts_info)?;
        let lottery_associated_usdc_token_account_info = next_account_info(accounts_info)?;
        let fund_receiver_usdc_token_account_info = next_account_info(accounts_info)?;
        let fund_receiver_rent_exempt_account_info = next_account_info(accounts_info)?;
        let standard_token_program_account_info = next_account_info(accounts_info)?;

        check_account_is_signer(lottery_account_authority_account_info)?;

        // validate config account
        Config::validate_config_account(
            config_global_account_info,
            program_id
        )?;

        // check is_pause flag
        Config::check_is_pause_raw(config_global_account_info)?;

        // validate usdc mint account
        compare_usdc_mint_account_with_config_global_account_info(
            config_global_account_info,
            usdc_mint_account_info.key
        )?;

        // validate lottery account
        Lottery::validate_lottery_account(
            lottery_account_info,
            program_id
        )?;

        let lottery_account = Lottery::deserialize(
            &mut &lottery_account_info.data.try_borrow().unwrap()[..]
        )?;

        // validate lottery authority
        check_accounts_key_to_be_identical(
            &lottery_account.authority,
            lottery_account_authority_account_info.key,
            LotteryError::InvalidLotteryAccountAuthority.into()
        )?;

        // validate lottery state
        let current_time = (Clock::get()?).unix_timestamp;
        if lottery_account.get_lottery_state(current_time) != LotteryState::Failed {
            return Err(
                LotteryError::InvalidLotteryState.into()
            );
        };

        if lottery_account.is_creator_withdrawed_when_lottery_was_failed == false {
            return Err(
                LotteryError::FirstCloseLotteryArbitrartAssociatedTokenAccount.into()
            );
        };

        // validate lottery associated usdc token account
        check_accounts_key_to_be_identical(
            &get_associated_token_address(
                lottery_account_info.key,
                usdc_mint_account_info.key
            ),
            lottery_associated_usdc_token_account_info.key,
            LotteryError::InvalidLotteryAssociatedUsdcTokenAccount.into()
        )?;

        // validate lottery account "initial_bytes" field
        let current_data_size: u64 = lottery_account_info
            .data_len()
            .try_into()
            .unwrap();

        if current_data_size != lottery_account.initial_bytes {
            return Err(
                LotteryError::CannotCloseAccounts.into()
            );
        };

        // transfer all usdc tokens if exists
        let TokenAccount { amount: usdc_token_balance, .. } = TokenAccount::unpack(
            &lottery_associated_usdc_token_account_info.data.try_borrow().unwrap()
        )?;

        if usdc_token_balance > 0 {
            let MintAccount { decimals, .. } = MintAccount::unpack(
                &usdc_mint_account_info.data.try_borrow().unwrap()
            )?;

            invoke_signed(
                &transfer_spl_checked(
                    standard_token_program_account_info.key,
                    lottery_associated_usdc_token_account_info.key,
                    usdc_mint_account_info.key,
                    fund_receiver_usdc_token_account_info.key,
                    lottery_account_info.key,
                    &[],
                    usdc_token_balance,
                    decimals
                )?,
                &[
                    lottery_associated_usdc_token_account_info.clone(),
                    usdc_mint_account_info.clone(),
                    fund_receiver_usdc_token_account_info.clone(),
                    lottery_account_info.clone()
                ],
                &[
                    &[
                        LOTTERY_ACCOUNT_SEED.as_bytes(),
                        &lottery_account.authority.to_bytes(),
                        get_lottery_literal_seed(&lottery_account.lottery_description).as_slice(),
                        &[ lottery_account.canonical_bump ]
                    ]
                ]
            )?;
            sol_log("USDC tokens transfered.");
        };

        // close usdc_associated_token_account and reclaim the rent_exempt_lamports
        invoke_signed(
            &close_token_account(
                standard_token_program_account_info.key,
                lottery_associated_usdc_token_account_info.key,
                fund_receiver_rent_exempt_account_info.key,
                lottery_account_info.key,
                &[]
            )?,
            &[
                lottery_associated_usdc_token_account_info.clone(),
                fund_receiver_rent_exempt_account_info.clone(),
                lottery_account_info.clone()
            ],
            &[
                &[
                    LOTTERY_ACCOUNT_SEED.as_bytes(),
                    &lottery_account_authority_account_info.key.to_bytes(),
                    get_lottery_literal_seed(&lottery_account.lottery_description).as_slice(),
                    &[ lottery_account.canonical_bump ]
                ]
            ]
        )?;
        sol_log("Usdc token account closed & rent exempt lamports refunded.");

        // close lottery_account and reclaim the rent_exempt_lamports
        Lottery::close_lottery_account(
            lottery_account_info,
            fund_receiver_rent_exempt_account_info
        )?;
        sol_log("Lottery account closed & rent exempt lamports refunded.");

        Ok(())
    }

    pub fn process_change_fee_of_lottery_creation(
        program_id: &Pubkey,
        accounts_info: &[AccountInfo],
        new_fee: u64
    ) -> ProgramResult {
        let accounts_info = &mut accounts_info.iter();

        let config_global_account_info = next_account_info(accounts_info)?;
        let config_account_authority_account_info = next_account_info(accounts_info)?;

        check_account_is_signer(config_account_authority_account_info)?;

        // validate config account
        Config::validate_config_account(
            config_global_account_info,
            program_id
        )?;

        // validate authority account
        check_accounts_key_to_be_identical(
            &get_config_account_authority(config_global_account_info),
            config_account_authority_account_info.key,
            LotteryError::InvalidConfigAuthority.into()
        )?;

        // update the lottery account
        sol_memcpy(
            config_global_account_info
                .data
                .try_borrow_mut()
                .unwrap()
                .get_mut(50..58)
                .unwrap(),
            &new_fee.to_le_bytes(),
            std::mem::size_of::<u64>()
        );
        
        sol_log("Config account updated.");

        Ok(())
    }

    pub fn process_change_config_account_authority(
        program_id: &Pubkey,
        accounts_info: &[AccountInfo]
    ) -> ProgramResult {
        let accounts_info = &mut accounts_info.iter();

        let config_global_account_info = next_account_info(accounts_info)?;
        let config_account_authority_account_info = next_account_info(accounts_info)?;
        let new_authority_account_info = next_account_info(accounts_info)?;

        check_account_is_signer(config_account_authority_account_info)?;

        // validate config account
        Config::validate_config_account(
            config_global_account_info,
            program_id
        )?;
        
        // validate authority account
        check_accounts_key_to_be_identical(
            &get_config_account_authority(config_global_account_info),
            config_account_authority_account_info.key,
            LotteryError::InvalidConfigAuthority.into()
        )?;

        if new_authority_account_info.key == &get_config_account_authority(config_global_account_info) {
            return Err(
                LotteryError::InvalidNewConfigAccountAuthority.into()
            );
        };

        // update the lottery account
        sol_memcpy(
            config_global_account_info
                .data
                .try_borrow_mut()
                .unwrap()
                .get_mut(10..42)
                .unwrap(),
            &new_authority_account_info.key.to_bytes(),
            std::mem::size_of::<Pubkey>()
        );

        sol_log("Config account updated.");

        Ok(())
    }

    pub fn process_change_fee_of_tickets(
        program_id: &Pubkey,
        accounts_info: &[AccountInfo],
        new_fee: f64
    ) -> ProgramResult {
        let accounts_info = &mut accounts_info.iter();

        let config_global_account_info = next_account_info(accounts_info)?;
        let config_account_authority_account_info = next_account_info(accounts_info)?;

        check_account_is_signer(config_account_authority_account_info)?;

        // validate config account
        Config::validate_config_account(
            config_global_account_info,
            program_id
        )?;

        // validate new_fee
        Config::validate_fee_per_ticket(&new_fee)?;

        // validate authority account
        check_accounts_key_to_be_identical(
            &get_config_account_authority(config_global_account_info),
            config_account_authority_account_info.key,
            LotteryError::InvalidConfigAuthority.into()
        )?;

        // update the config account
        sol_memcpy(
            config_global_account_info
                .data
                .try_borrow_mut()
                .unwrap()
                .get_mut(58..66)
                .unwrap(),
            &new_fee.to_le_bytes(),
            std::mem::size_of::<f64>()
        );

        sol_log("Config account updated.");

        Ok(())
    }

    pub fn process_claim_protocol_fees(
        program_id: &Pubkey,
        accounts_info: &[AccountInfo],
        n: u8
    ) -> ProgramResult {
        let accounts_info = &mut accounts_info.iter();

        let config_global_account_info = next_account_info(accounts_info)?;
        let config_account_authority_info = next_account_info(accounts_info)?;
        let treasury_account_info = next_account_info(accounts_info)?;
        let usdc_mint_account_info = next_account_info(accounts_info)?;
        let standard_token_program_account_info = next_account_info(accounts_info)?;

        check_account_is_signer(config_account_authority_info)?;

        let lotteries_accounts_infos = next_account_infos(accounts_info, n as usize).map_err::<ProgramError, _>(|_|
            LotteryError::InvalidAmountOfLotteries.into()
        )?;
        let lotteries_associated_usdc_token_accounts_infos = next_account_infos(accounts_info, n as usize).map_err::<ProgramError, _>(|_|
            LotteryError::InvalidAmountOfAssociatedTokenAccount.into()
        )?;

        // validate n parameter
        if n == 0 {
            return Err(
                LotteryError::InvalidNParameter.into()
            );
        };

        // validate config account
        Config::validate_config_account(config_global_account_info, program_id)?;

        let config_account = Config::deserialize(
            &mut &config_global_account_info.data.try_borrow().unwrap()[..]
        )?;

        // validate config account authority
        check_accounts_key_to_be_identical(
            config_account_authority_info.key,
            &config_account.authority,
            LotteryError::InvalidConfigAuthority.into()
        )?;

        // validate treasury account
        check_accounts_key_to_be_identical(
            treasury_account_info.key,
            &config_account.treasury,
            LotteryError::InvalidTreasuryAccount.into()
        )?;

        // validate usdc mint account
        check_accounts_key_to_be_identical(
            usdc_mint_account_info.key,
            &config_account.usdc_mint_account,
            LotteryError::InvalidUsdcMintAccount.into()
        )?;

        let current_time = (Clock::get()?).unix_timestamp;

        for (index, lottery_account_info) in lotteries_accounts_infos.iter().enumerate() {
            let lottery_associated_usdc_token_account_info = lotteries_associated_usdc_token_accounts_infos.get(index).unwrap();

            // validate lottery account
            Lottery::validate_lottery_account(lottery_account_info, program_id)?;

            let lottery_account = Lottery::deserialize(
                &mut &lottery_account_info.data.try_borrow().unwrap()[..]
            )?;

            // validate lottery state
            if lottery_account.get_lottery_state(current_time) != LotteryState::Successful {
                return Err(
                    LotteryError::InvalidLotteryState.into()
                );
            };

            // validate that the lottery is not claimed before
            if lottery_account.is_protocol_fee_claimed == true {
                lottery_account_info.key.log();

                return Err(
                    LotteryError::ProtocolFeeAlreadyClaimed.into()
                );
            };

            // validate associated usdc token account
            check_accounts_key_to_be_identical(
                &get_associated_token_address(
                    lottery_account_info.key,
                    usdc_mint_account_info.key
                ),
                lottery_associated_usdc_token_account_info.key,
                LotteryError::InvalidLotteryAssociatedUsdcTokenAccount.into()
            )?;

            let fee = lottery_account.lottery_creation_fee.checked_add(
                lottery_account.protocol_fee
            ).ok_or::<ProgramError>(LotteryError::Overflow.into())?;

            let MintAccount { decimals, .. } = MintAccount::unpack(
                &usdc_mint_account_info.data.try_borrow().unwrap()
            )?;

            invoke_signed(
                &transfer_spl_checked(
                    standard_token_program_account_info.key,
                    lottery_associated_usdc_token_account_info.key,
                    usdc_mint_account_info.key,
                    treasury_account_info.key,
                    lottery_account_info.key,
                    &[],
                    fee,
                    decimals
                )?,
                &[
                    lottery_associated_usdc_token_account_info.clone(),
                    usdc_mint_account_info.clone(),
                    treasury_account_info.clone(),
                    lottery_account_info.clone()
                ],
                &[
                    &[
                        LOTTERY_ACCOUNT_SEED.as_bytes(),
                        &lottery_account.authority.to_bytes(),
                        get_lottery_literal_seed(&lottery_account.lottery_description).as_slice(),
                        &[ lottery_account.canonical_bump ]
                    ]
                ]
            )?;

            // update the lottery account
            let mut lottery_account_data = lottery_account_info
                .data
                .try_borrow_mut()
                .unwrap();

            let is_protocol_fee_claimed = lottery_account_data.get_mut(145).unwrap();
            *is_protocol_fee_claimed = true as u8;
            // update the lottery account

            solana_program::msg!("Fee Transfered -> {} USDC", spl_token::amount_to_ui_amount(fee, decimals));
        };

        sol_log("All Fees Transferd Successfuly.");

        Ok(())
    }

    pub fn process_change_maximum_number_of_winners(
        program_id: &Pubkey,
        accounts_info: &[AccountInfo],
        new_max: u8
    ) -> ProgramResult {
        let accounts_info = &mut accounts_info.iter();

        let config_global_account_info = next_account_info(accounts_info)?;
        let config_account_authority_account_info = next_account_info(accounts_info)?;

        check_account_is_signer(config_account_authority_account_info)?;

        check_max_numbers_of_winner(&new_max)?;

        // validate config account
        Config::validate_config_account(
            config_global_account_info,
            program_id
        )?;

        // validate authority account
        check_accounts_key_to_be_identical(
            &get_config_account_authority(config_global_account_info),
            config_account_authority_account_info.key,
            LotteryError::InvalidConfigAuthority.into()
        )?;

        // update the config account
        let mut config_account_data = config_global_account_info
            .data
            .try_borrow_mut()
            .unwrap();

        let max_number_of_winners = config_account_data.get_mut(66).unwrap();
        *max_number_of_winners = new_max;
        // update the config account

        sol_log("Config account updated.");

        Ok(())
    }

    pub fn process_change_maximum_age_of_price_feed(
        program_id: &Pubkey,
        accounts_info: &[AccountInfo],
        new_max: u8
    ) -> ProgramResult {
        let accounts_info = &mut accounts_info.iter();

        let config_global_account_info = next_account_info(accounts_info)?;
        let config_account_authority_account_info = next_account_info(accounts_info)?;

        check_account_is_signer(config_account_authority_account_info)?;

        check_max_price_feed_age(&new_max)?;

        // validate config account
        Config::validate_config_account(
            config_global_account_info,
            program_id
        )?;

        // validate authority account
        check_accounts_key_to_be_identical(
            &get_config_account_authority(config_global_account_info),
            config_account_authority_account_info.key,
            LotteryError::InvalidConfigAuthority.into()
        )?;

        // update the config account
        let mut config_account_data = config_global_account_info
            .data
            .try_borrow_mut()
            .unwrap();

        let max_age_of_price_feed = config_account_data.get_mut(131).unwrap();
        *max_age_of_price_feed = new_max;
        // update the config account

        sol_log("Config account updated.");

        Ok(())
    }

    pub fn process_change_pause_state(
        program_id: &Pubkey,
        accounts_info: &[AccountInfo],
        pause: bool
    ) -> ProgramResult {
        let accounts_info = &mut accounts_info.iter();

        let config_global_account_info = next_account_info(accounts_info)?;
        let config_account_authority_account_info = next_account_info(accounts_info)?;

        check_account_is_signer(config_account_authority_account_info)?;

        // validate config account
        Config::validate_config_account(
            config_global_account_info,
            program_id
        )?;

        // validate authority account
        check_accounts_key_to_be_identical(
            &get_config_account_authority(config_global_account_info),
            config_account_authority_account_info.key,
            LotteryError::InvalidConfigAuthority.into()
        )?;

        // update the config account
        let mut config_account_data = config_global_account_info
            .data
            .try_borrow_mut()
            .unwrap();
        
        let is_pause = config_account_data.get_mut(9).unwrap();
        *is_pause = pause as u8;
        // update the config account

        sol_log("Config account updated.");

        Ok(())
    }

    pub fn process_change_treasury_account(
        program_id: &Pubkey,
        accounts_info: &[AccountInfo]
    ) -> ProgramResult {
        let accounts_info = &mut accounts_info.iter();

        let config_global_account_info = next_account_info(accounts_info)?;
        let config_account_authority_info = next_account_info(accounts_info)?;
        let new_treasury_account_info = next_account_info(accounts_info)?;

        check_account_is_signer(config_account_authority_info)?;

        // validate config account
        Config::validate_config_account(config_global_account_info, program_id)?;

        // validate config authority
        check_accounts_key_to_be_identical(
            config_account_authority_info.key,
            &get_config_account_authority(config_global_account_info),
            LotteryError::InvalidConfigAuthority.into()
        )?;

        // update the config account
        sol_memcpy(
            config_global_account_info
                .data
                .try_borrow_mut()
                .unwrap()
                .get_mut(233..265)
                .unwrap(),
            &new_treasury_account_info.key.to_bytes(),
            std::mem::size_of::<Pubkey>()
        );

        sol_log("Config account updated.");

        Ok(())
    }

    pub fn process_change_protocol_mint_account(
        accounts_info: &[AccountInfo],
        program_id: &Pubkey,
        new_mint_account: Pubkey
    ) -> ProgramResult {
        let accounts_info = &mut accounts_info.iter();

        let config_global_account_info = next_account_info(accounts_info)?;
        let config_account_authority_account_info = next_account_info(accounts_info)?;

        check_account_is_signer(config_account_authority_account_info)?;

        // validate config account
        Config::validate_config_account(
            config_global_account_info,
            program_id
        )?;

        // validate authority account
        check_accounts_key_to_be_identical(
            &get_config_account_authority(config_global_account_info),
            config_account_authority_account_info.key,
            LotteryError::InvalidConfigAuthority.into()
        )?;

        // update the config account
        sol_memcpy(
            config_global_account_info
                .data
                .try_borrow_mut()
                .unwrap()
                .get_mut(99..131)
                .unwrap(),
            &new_mint_account.to_bytes(),
            std::mem::size_of::<Pubkey>()
        );

        sol_log("Config account updated.");

        Ok(())
    }

    pub fn process_change_pyth_price_receiver_program_account(
        accounts_info: &[AccountInfo],
        program_id: &Pubkey,
        new_pyth_price_receiver_programid: Pubkey
    ) -> ProgramResult {
        let accounts_info = &mut accounts_info.iter();

        let config_global_account_info = next_account_info(accounts_info)?;
        let config_account_authority_account_info = next_account_info(accounts_info)?;

        check_account_is_signer(config_account_authority_account_info)?;

        // validate config account
        Config::validate_config_account(
            config_global_account_info,
            program_id
        )?;

        // validate authority account
        check_accounts_key_to_be_identical(
            &get_config_account_authority(config_global_account_info),
            config_account_authority_account_info.key,
            LotteryError::InvalidConfigAuthority.into()
        )?;

        // update the config account
        sol_memcpy(
            config_global_account_info
                .data
                .try_borrow_mut()
                .unwrap()
                .get_mut(67..99)
                .unwrap(),
            &new_pyth_price_receiver_programid.to_bytes(),
            std::mem::size_of::<Pubkey>()
        );

        sol_log("Config account updated.");

        Ok(())
    }

    pub fn process_change_price_feed_id(
        program_id: &Pubkey,
        accounts_info: &[AccountInfo],
        price_feed_index: u8,
        new_price_feed_id: String
    ) -> ProgramResult {
        let accounts_info = &mut accounts_info.iter();

        let config_global_account_info = next_account_info(accounts_info)?;
        let config_account_authority_account_info = next_account_info(accounts_info)?;

        check_account_is_signer(config_account_authority_account_info)?;

        // validate config account
        Config::validate_config_account(
            config_global_account_info,
            program_id
        )?;

        // validate authority account
        check_accounts_key_to_be_identical(
            &get_config_account_authority(config_global_account_info),
            config_account_authority_account_info.key,
            LotteryError::InvalidConfigAuthority.into()
        )?;

        let mut config_account = Config::deserialize(
            &mut &config_global_account_info.data.try_borrow().unwrap()[..]
        )?;

        config_account.pyth_price_feed_ids[price_feed_index as usize] = new_price_feed_id;

        config_account.serialize(
            &mut &mut config_global_account_info.data.try_borrow_mut().unwrap()[..]
        )?;

        sol_log("Config account updated.");

        Ok(())
    }

    pub fn process_change_price_feed_account(
        program_id: &Pubkey,
        accounts_info: &[AccountInfo],
        price_feed_account_index: u8,
        new_price_feed_account: Pubkey
    ) -> ProgramResult {
        let accounts_info = &mut accounts_info.iter();

        let config_global_account_info = next_account_info(accounts_info)?;
        let config_account_authority_account_info = next_account_info(accounts_info)?;

        check_account_is_signer(config_account_authority_account_info)?;

        // validate config account
        Config::validate_config_account(
            config_global_account_info,
            program_id
        )?;

        // validate authority account
        check_accounts_key_to_be_identical(
            &get_config_account_authority(config_global_account_info),
            config_account_authority_account_info.key,
            LotteryError::InvalidConfigAuthority.into()
        )?;

        let mut config_account = Config::deserialize(
            &mut &config_global_account_info.data.try_borrow().unwrap()[..]
        )?;

        config_account.pyth_price_feed_accounts[price_feed_account_index as usize] = new_price_feed_account;

        config_account.serialize(
            &mut &mut config_global_account_info.data.try_borrow_mut().unwrap()[..]
        )?;

        sol_log("Config account updated.");

        Ok(())
    }

    pub fn process_change_max_lottery_description_length(
        program_id: &Pubkey,
        accounts_info: &[AccountInfo],
        new_max_lottery_description_length: u64
    ) -> ProgramResult {
        let accounts_info = &mut accounts_info.iter();

        let config_global_account_info = next_account_info(accounts_info)?;
        let config_account_authority_account_info = next_account_info(accounts_info)?;

        check_account_is_signer(config_account_authority_account_info)?;

        // validate config account
        Config::validate_config_account(
            config_global_account_info,
            program_id
        )?;

        // validate authority account
        check_accounts_key_to_be_identical(
            &get_config_account_authority(config_global_account_info),
            config_account_authority_account_info.key,
            LotteryError::InvalidConfigAuthority.into()
        )?;

        // update the config account
        sol_memcpy(
            config_global_account_info
                .data
                .try_borrow_mut()
                .unwrap()
                .get_mut(265..273)
                .unwrap(),
            new_max_lottery_description_length.to_le_bytes().as_slice(),
            size_of::<u64>()
        );

        sol_log("Config account updated.");

        Ok(())
    }

    pub fn process(
        program_id: &Pubkey,
        accounts_info: &[AccountInfo],
        instruction_data: &[u8]
    ) -> ProgramResult {
        let ix = Instructions::unpack(instruction_data)?;

        use Instructions::*;
        match ix {
            CreateAndInitializeProgramConfigAccount {
                authority,
                lottery_creation_fee,
                lottery_tickets_fee,
                maximum_number_of_winners,
                pyth_price_receiver_programid,
                usdc_mint_account,
                maximum_time_of_price_feed_age,
                minimum_tickets_to_be_sold_in_lottery,
                pyth_price_feed_accounts,
                maximum_time_for_lottery_account,
                treasury,
                pyth_price_feed_ids,
                max_lottery_description_bytes
            } => {
                sol_log("Instruction: CreateAndInitializeProgramConfigAccount");

                check_accounts_amount(accounts_info.len(), 4)?;

                Self::process_create_and_initialize_program_config_account(
                    accounts_info,
                    program_id,
                    authority,
                    lottery_creation_fee,
                    lottery_tickets_fee,
                    maximum_number_of_winners,
                    pyth_price_receiver_programid,
                    usdc_mint_account,
                    maximum_time_of_price_feed_age,
                    minimum_tickets_to_be_sold_in_lottery,
                    pyth_price_feed_accounts,
                    maximum_time_for_lottery_account,
                    treasury,
                    pyth_price_feed_ids,
                    max_lottery_description_bytes
                )
            },
            CreateAndInitializeLotteryAccount {
                fund_amount,
                winners_count,
                starting_time,
                ending_time,
                minimum_tickets_amount_required_to_be_sold,
                ticket_price,
                maximum_number_of_tickets_per_user,
                lottery_description
            } => {
                sol_log("Instruction: CreateAndInitializeLotteryAccount");

                check_accounts_amount(accounts_info.len(), 13)?;

                Self::process_create_and_initialize_lottery_account(
                    program_id,
                    accounts_info,
                    fund_amount,
                    winners_count,
                    starting_time,
                    ending_time,
                    minimum_tickets_amount_required_to_be_sold,
                    ticket_price,
                    maximum_number_of_tickets_per_user,
                    lottery_description
                )
            },
            CreateAndInitializeUserAccount => {
                sol_log("Instruction: CreateAndInitializeUserAccount");

                check_accounts_amount(accounts_info.len(), 6)?;

                Self::process_create_and_initialize_user_account(
                    program_id,
                    accounts_info
                )
            },
            BuyTicket {
                tickets_amount,
                expected_token_price_per_ticket
            } => {
                sol_log("Instruction: BuyTicket");

                check_accounts_amount(accounts_info.len(), 10)?;

                Self::process_buy_ticket(
                    program_id,
                    accounts_info,
                    tickets_amount,
                    expected_token_price_per_ticket
                )
            },
            ChangeLotteryTicketPrice { new_ticket_price } => {
                sol_log("Instruction: ChangeLotteryTicketPrice");

                check_accounts_amount(accounts_info.len(), 3)?;

                Self::process_change_lottery_ticket_price(
                    accounts_info,
                    program_id,
                    new_ticket_price
                )
            },
            EndLotteryAndPickWinners => {
                sol_log("Instruction: EndLotteryAndPickWinners");

                check_accounts_amount(accounts_info.len(), 5)?;

                Self::process_end_lottery_and_pick_winners(
                    program_id,
                    accounts_info
                )
            },
            WithdrawSucceedLottery => {
                sol_log("Instruction: WithdrawSucceedLottery");

                check_accounts_amount(accounts_info.len(), 7)?;

                Self::process_withdraw_succeed_lottery(
                    program_id,
                    accounts_info
                )
            },
            WithdrawLotterysWinners => {
                sol_log("Instruction: WithdrawLotterysWinners");

                check_accounts_amount(accounts_info.len(), 8)?;

                Self::process_withdraw_lottery_winners(
                    program_id,
                    accounts_info
                )
            },
            WithdrawAndCloseSucceedUser => {
                sol_log("Instruction: WithdrawAndCloseSucceedUser");

                check_accounts_amount(accounts_info.len(), 6)?;

                Self::process_withdraw_and_close_succeed_user(
                    program_id, 
                    accounts_info
                )
            },
            WithdrawFailedLottery {} => {
                sol_log("Instruction: WithdrawFailedLottery");

                check_accounts_amount(accounts_info.len(), 11)?;

                Self::process_withdraw_failed_lottery(
                    program_id,
                    accounts_info
                )
            },
            WithdrawAndCloseFailedUser {} => {
                sol_log("Instruction: WithdrawAndCloseFailedUser");

                check_accounts_amount(accounts_info.len(), 10)?;

                Self::process_withdraw_and_close_failed_user(
                    program_id,
                    accounts_info
                )
            },
            CloseLotteryAccountAndUsdcTokenAccount => {
                sol_log("Instruction: CloseLotteryAccountAndUsdcTokenAccount");

                check_accounts_amount(accounts_info.len(), 8)?;

                Self::process_close_lottery_account_and_usdc_token_account(
                    program_id,
                    accounts_info
                )
            },
            ChangeFeeOfLotteryCreation { new_fee } => {
                sol_log("Instruction: ChangeFeeOfLotteryCreation");

                check_accounts_amount(accounts_info.len(), 2)?;

                Self::process_change_fee_of_lottery_creation(
                    program_id,
                    accounts_info,
                    new_fee
                )
            },
            ChangeConfigAccountAuthority => {
                sol_log("Instruction: ChangeConfigAccountAuthority");

                check_accounts_amount(accounts_info.len(), 3)?;

                Self::process_change_config_account_authority(
                    program_id,
                    accounts_info
                )
            },
            ChangeFeeOfTickets { new_fee } => {
                sol_log("Instruction: ChangeFeeOfTickets");

                check_accounts_amount(accounts_info.len(), 2)?;

                Self::process_change_fee_of_tickets(
                    program_id,
                    accounts_info,
                    new_fee
                )
            },
            ClaimProtocolFees { n } => {
                sol_log("Instruction: ClaimProtocolFees");

                check_accounts_amount(accounts_info.len(), 5 + (n as usize * 2))?;
                
                Self::process_claim_protocol_fees(
                    program_id,
                    accounts_info,
                    n
                )
            },
            ChangeMaximumNumberOfWinners { new_max } => {
                sol_log("Instruction: ChangeMaximumNumberOfWinners");

                check_accounts_amount(accounts_info.len(), 2)?;

                Self::process_change_maximum_number_of_winners(
                    program_id,
                    accounts_info,
                    new_max
                )
            },
            ChangeMaximumAgeOfPriceFeed { new_max } => {
                sol_log("Instriction: ChangeMaximumAgeOfPriceFeed");

                check_accounts_amount(accounts_info.len(), 2)?;

                Self::process_change_maximum_age_of_price_feed(
                    program_id,
                    accounts_info,
                    new_max
                )
            },
            ChangePauseState { pause } => {
                sol_log("Instruction: ChangePauseState");

                check_accounts_amount(accounts_info.len(), 2)?;

                Self::process_change_pause_state(
                    program_id,
                    accounts_info,
                    pause
                )
            },
            ChangeTreasury => {
                sol_log("Instruction: ChangeTreasury");

                check_accounts_amount(accounts_info.len(), 3)?;

                Self::process_change_treasury_account(
                    program_id,
                    accounts_info
                )
            },
            ChangeProtocolMintAccount { new_mint_account } => {
                sol_log("Instruction: ChangeMintAccount");

                check_accounts_amount(accounts_info.len(), 2)?;

                Self::process_change_protocol_mint_account(
                    accounts_info,
                    program_id,
                    new_mint_account
                )
            },
            ChangePythPriceReceiverProgramAccount { new_pyth_price_receiver_programid } => {
                sol_log("Instruction: ChangePythPriceReceiverProgramAccount");

                check_accounts_amount(accounts_info.len(), 2)?;

                Self::process_change_pyth_price_receiver_program_account(
                    accounts_info,
                    program_id,
                    new_pyth_price_receiver_programid
                )
            },
            ChangePriceFeedId {
                index,
                price_feed_id
            } => {
                sol_log("Instruction: ChangePriceFeedId");

                check_accounts_amount(accounts_info.len(), 2)?;

                Self::process_change_price_feed_id(
                    program_id, 
                    accounts_info, 
                    index, 
                    price_feed_id
                )
            },
            ChangePriceFeedAccount { 
                index, 
                price_feed_account
            } => {
                sol_log("Instruction: ChangePriceFeedAccount");

                check_accounts_amount(accounts_info.len(), 2)?;

                Self::process_change_price_feed_account(
                    program_id, 
                    accounts_info, 
                    index, 
                    price_feed_account
                )
            },
            ChangeMaxLotteryDescriptionLength { new_length } => {
                sol_log("Instruction: ChangeMaxLotteryDescriptionLength");

                check_accounts_amount(accounts_info.len(), 2)?;

                Self::process_change_max_lottery_description_length(
                    program_id, 
                    accounts_info, 
                    new_length
                )
            }
        }
    }
}

fn create_pda_account<'a, 'b>(
    new_pda_account_info: &AccountInfo<'a>,
    fee_payer_account_info: &AccountInfo<'b>,
    space: usize,
    program_id: &Pubkey,
    seeds: &[&[u8]]
) -> ProgramResult where 'b:'a, 'a:'b {
    let rent = Rent::get()?.minimum_balance(space);
    let new_pda_account_balance = new_pda_account_info.lamports();
    if new_pda_account_balance < rent {
        let lamports_needed = rent
            .checked_sub(new_pda_account_balance)
            .unwrap();
        
        invoke(
            &transfer_lamports(
                fee_payer_account_info.key,
                new_pda_account_info.key,
                lamports_needed
            ),
            &[
                fee_payer_account_info.clone(),
                new_pda_account_info.clone()
            ]
        )?;
    };

    invoke_signed(
        &allocate_memory(
            new_pda_account_info.key,
            space as u64
        ),
        &[ new_pda_account_info.clone() ],
        &[ seeds ]
    )?;

    invoke_signed(
        &assign_new_owner(
            new_pda_account_info.key,
            program_id
        ),
        &[ new_pda_account_info.clone() ],
        &[ seeds ]
    )?;

    Ok(())
}

pub fn get_config_account_authority(config_global_account_info: &AccountInfo) -> Pubkey {
    let config_account_data = config_global_account_info
        .data
        .try_borrow()
        .unwrap();

    Pubkey::try_from_slice(
        config_account_data.get(10..42).unwrap()
    ).unwrap()
}

pub fn check_accounts_amount(
    accounts_len: usize,
    expected_len: usize
) -> ProgramResult {
    if accounts_len != expected_len {
        return Err(
            ProgramError::NotEnoughAccountKeys
        );
    };

    Ok(())
}

// We don't need this checker function BUT to be developer-friendly we used it.
pub fn check_system_program_id(program_id: &Pubkey) -> ProgramResult {
    if check_id(program_id) == false {
        return Err(
            ProgramError::IncorrectProgramId
        );
    };
    
    Ok(())
}

pub fn get_lottery_literal_seed(lottery_description: &String) -> [u8; HASH_BYTES] {
    sha256(
        lottery_description.as_bytes()
    ).to_bytes()
}

pub fn get_price(
    price_feed_account_info: &AccountInfo,
    verification_level: VerificationLevel,
    price_feed_max_age: u8,
    price_feed_id: &str,
    clock: &Clock
) -> Result<Price, ProgramError> {
    let feed_id: FeedId = get_feed_id_from_hex(
        price_feed_id
    ).unwrap();

    let price_info = PriceUpdateV2::deserialize(
        &mut &price_feed_account_info.data.try_borrow().unwrap()[8..]
    )?;

    let price: Price = price_info
        .get_price_no_older_than_with_custom_verification_level(
            clock,
            price_feed_max_age as u64,
            &feed_id,
            verification_level
        )
        .map_err(|err| ProgramError::Custom(err as u32))?;

    Ok(price)
}

pub fn calculate_fee_and_update_lottery_account(
    config_account: &Config,
    lottery_account: &Lottery,
    lottery_account_info: &AccountInfo,
    tickets_amount: u32
) -> Result<u64, ProgramError> {
    let usdc_per_ticket = &lottery_account.ticket_price;
    let total_tickets_price = (tickets_amount as u64)
        .checked_mul(*usdc_per_ticket)
        .ok_or::<ProgramError>(LotteryError::Overflow.into())?;

    let protocol_fee_per_tickets = config_account.lottery_tickets_fee;
    let protocol_fee = ((total_tickets_price as f64) * protocol_fee_per_tickets) / ((10u64.pow(2)) as f64);

    // check for overflow, underflow & NAN 
    if 
        protocol_fee == f64::INFINITY || 
        protocol_fee == f64::NEG_INFINITY || 
        protocol_fee.is_nan() == true 
    {
        return Err(
            LotteryError::Overflow.into()
        );
    };

    let old_protocol_fee = lottery_account.protocol_fee;
    let new_protocol_fee = old_protocol_fee
        .checked_add(protocol_fee as u64)
        .ok_or::<ProgramError>(LotteryError::Overflow.into())?;

    // update lottery's protocol_fee
    sol_memcpy(
        lottery_account_info
            .data
            .try_borrow_mut()
            .unwrap()
            .get_mut(134..142)
            .unwrap(),
        new_protocol_fee.to_le_bytes().as_slice(),
        size_of::<u64>()
    );

    Ok(total_tickets_price)
}

pub fn compare_usdc_mint_account_with_config_global_account_info(
    config_global_account_info: &AccountInfo,
    expected_usdc_mint_account: &Pubkey
) -> Result<(), ProgramError> {
    let config_account_data = config_global_account_info
        .data
        .try_borrow()
        .unwrap();

    if sol_memcmp(
        &config_account_data.get(99..131).unwrap(),
        expected_usdc_mint_account.to_bytes().as_slice(),
        std::mem::size_of::<Pubkey>()
    ) != 0 {
        return Err(
            LotteryError::InvalidUsdcMintAccount.into()
        );
    };

    Ok(())
}

pub fn check_max_numbers_of_winner(max_number: &u8) -> ProgramResult {
    if !(
        max_number > &0 &&
        max_number <= &30
    ) {
        return Err(
            LotteryError::InvalidMaxNumberOfWinners.into()
        );
    };

    Ok(())
}

pub fn check_max_price_feed_age(max_age: &u8) -> ProgramResult {
    if max_age == &0 {
        return Err(
            LotteryError::InvalidMaxPriceFeedAge.into()
        );
    };

    Ok(())
}

pub fn check_account_is_signer(account_info: &AccountInfo) -> ProgramResult {
    if account_info.is_signer == false {
        return Err(
            ProgramError::MissingRequiredSignature
        );
    };

    Ok(())
}

pub fn check_accounts_key_to_be_identical(
    account_a: &Pubkey,
    account_b: &Pubkey,
    error: ProgramError
) -> ProgramResult {
    if account_a != account_b {
        return Err(error);
    };

    Ok(())
}

// This check is only for making the code clean, system-program itself will check that the account does not exist before creating it.
pub fn check_account_is_raw(account_info: &AccountInfo) -> ProgramResult {
    if account_info.owner != &SYSTEM_PROGRAM_ID &&
       account_info.data_len() != 0
    {
        return Err(
            LotteryError::AccountMustBeRaw.into()
        );
    };

    Ok(())
}

#[cfg(test)]
mod test_processor {
    use {
        super::{
            get_price,
            BorshSerialize,
            VerificationLevel,
            Pubkey,
            calculate_fee_and_update_lottery_account,
            compare_usdc_mint_account_with_config_global_account_info,
            check_max_numbers_of_winner,
            check_max_price_feed_age,
            check_account_is_signer,
            check_accounts_key_to_be_identical,
            check_account_is_raw,
            AccountInfo,
            Config,
            Lottery
        },
        std::{
            rc::Rc,
            cell::RefCell,
            str::FromStr
        },
        solana_program::{
            clock::Epoch,
            sysvar::clock::Clock,
            program_error::ProgramError,
            native_token::sol_to_lamports
        },
        pyth_solana_receiver_sdk::ID_CONST,
        borsh::BorshDeserialize,
        crate::program::id
    };

    const SOL_PRICE_FEED_ID: &str = "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d";

    #[test]
    fn test_get_price() {
        let mut sol_price_update_v2_data: Vec<u8> = vec![
            34,241,35,99,157,126,244,205,96,49,71,4,52,13,237,223,55,31,212,36,114,20,143,36,142,
            157,26,109,26,94,178,172,58,205,139,127,213,214,178,67,1,239,13,139,111,218,44,235,164,
            29,161,93,64,149,209,218,57,42,13,47,142,208,198,199,188,15,76,250,200,194,128,181,109,
            227,51,34,198,2,0,0,0,143,124,117,0,0,0,0,0,248,255,255,255,68,75,208,103,0,0,0,0,67,75,
            208,103,0,0,0,0,168,236,132,217,2,0,0,0,88,208,130,0,0,0,0,0,22,103,111,19,0,0,0,0,0
        ]; // 134 Bytes (8-Bytes anchor discriminator)
        let sol_price = 11_914_064_867i64;

        let price_feed_max_age = 10u8;
        let verification_level = VerificationLevel::Full;
        let price_feed_id = SOL_PRICE_FEED_ID;

        let result_price = get_price(
            &AccountInfo {
                lamports: Rc::new(RefCell::new(&mut u64::default())),
                owner: &ID_CONST,
                key: &Pubkey::from_str("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE").unwrap(),
                data: Rc::new(RefCell::new(&mut sol_price_update_v2_data)),
                rent_epoch: Epoch::default(),
                is_signer: false,
                is_writable: false,
                executable: false
            },
            verification_level,
            price_feed_max_age,
            price_feed_id,
            &Clock::default()
        ).unwrap();

        assert_eq!(
            sol_price,
            result_price.price
        );
    }

    #[test]
    fn test_calculate_fee_and_update_lottery_account() {
        let mut config_account = Config::default();
        config_account.lottery_tickets_fee = 3.5;

        let mut lottery_account = Lottery::default();
        lottery_account.ticket_price = 1_000000; // 1 USDC

        let tickets_amount = 100u32;

        let mut lottery_account_data = lottery_account.try_to_vec().unwrap();
        let mut lottery_balance = solana_program::native_token::sol_to_lamports(1.0);
        let lottery_owner = id();
        let lottery_pubkey = Pubkey::from_str("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE").unwrap();
        let lottery_account_info = &AccountInfo::new(
            &lottery_pubkey,
            false,
            false,
            &mut lottery_balance,
            &mut lottery_account_data,
            &lottery_owner,
            false,
            Epoch::default()
        );

        let result_total_tickets_price = calculate_fee_and_update_lottery_account(
            &config_account,
            &lottery_account,
            lottery_account_info,
            tickets_amount
        ).unwrap();

        let updated_lottery_account = Lottery::deserialize(
            &mut &lottery_account_info.data.try_borrow().unwrap()[..]
        ).unwrap();

        assert_eq!(
            updated_lottery_account.protocol_fee,
            3_500000
        );

        assert_eq!(
            result_total_tickets_price,
            100_000000
        );
    }

    #[test]
    fn test_compare_usdc_mint_account_with_config_global_account_info() {
        let mut config_account = Config::default();
        config_account.usdc_mint_account = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();

        compare_usdc_mint_account_with_config_global_account_info(
            &AccountInfo {
                lamports: Rc::new(RefCell::new(&mut u64::default())),
                owner: &id(),
                key: &Pubkey::from_str("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE").unwrap(),
                data: Rc::new(RefCell::new(&mut config_account.try_to_vec().unwrap())),
                rent_epoch: Epoch::default(),
                is_signer: false,
                is_writable: false,
                executable: false
            },
            &Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap()
        ).unwrap();
    }

    #[test]
    fn test_success_check_max_numbers_of_winner() {
        check_max_numbers_of_winner(&10u8).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_fail_check_max_numbers_of_winner() {
        check_max_numbers_of_winner(&45u8).unwrap();
    }

    #[test]
    fn test_success_check_max_price_feed_age() {
        check_max_price_feed_age(&10u8).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_fail_check_max_price_feed_age() {
        check_max_price_feed_age(&0u8).unwrap();
    }

    #[test]
    fn test_success_check_account_is_signer() {
        check_account_is_signer(
            &AccountInfo {
                lamports: Rc::new(RefCell::new(&mut u64::default())),
                owner: &Pubkey::default(),
                key: &Pubkey::default(),
                data: Rc::new(RefCell::new(&mut [])),
                rent_epoch: Epoch::default(),
                is_signer: true,
                is_writable: false,
                executable: false
            }
        ).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_fail_check_account_is_signer() {
        check_account_is_signer(
            &AccountInfo {
                lamports: Rc::new(RefCell::new(&mut u64::default())),
                owner: &Pubkey::default(),
                key: &Pubkey::default(),
                data: Rc::new(RefCell::new(&mut [])),
                rent_epoch: Epoch::default(),
                is_signer: false,
                is_writable: false,
                executable: false
            }
        ).unwrap();
    }

    #[test]
    fn test_success_check_accounts_key_to_be_identical() {
        check_accounts_key_to_be_identical(
            &Pubkey::new_from_array([1; 32]),
            &Pubkey::new_from_array([1; 32]),
            ProgramError::Custom(0)
        ).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_fail_check_accounts_key_to_be_identical() {
        check_accounts_key_to_be_identical(
            &Pubkey::new_from_array([1; 32]),
            &Pubkey::new_from_array([2; 32]),
            ProgramError::Custom(0)
        ).unwrap();
    }

    #[test]
    fn test_success_check_account_is_raw() {
        check_account_is_raw(
            &AccountInfo::new(
                &Pubkey::new_from_array([3; 32]),
                false,
                false,
                &mut sol_to_lamports(0.0),
                &mut vec![],
                &Pubkey::default(),
                false,
                Epoch::default()
            )
        ).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_fail_check_account_is_raw() {
        check_account_is_raw(
            &AccountInfo::new(
                &Pubkey::new_from_array([3; 32]),
                false,
                false,
                &mut sol_to_lamports(0.01),
                &mut vec![ 10; 100 ],
                &Pubkey::new_from_array([5; 32]),
                false,
                Epoch::default()
            )
        ).unwrap();
    }
}
