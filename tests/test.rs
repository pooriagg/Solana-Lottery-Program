use {
    borsh::{
        BorshDeserialize,
        BorshSerialize
    }, 

    pyth_solana_receiver_sdk::{
        price_update::{
            get_feed_id_from_hex, 
            PriceFeedMessage, 
            PriceUpdateV2, 
            VerificationLevel
            
        },
        ID as PYTH_PRICE_RECEIVER_PROGRAM_ID
    }, 

    sol_lottery::{
        error::LotteryError, 
        instruction::{
            Instructions,
            instruction_buy_ticket, 
            instruction_change_lottery_ticket_price, 
            instruction_create_and_initialize_lottery_account, 
            instruction_create_and_initialize_user_account, 
            instruction_end_lottery_and_pick_winners, 
            instruction_withdraw_and_close_failed_user, 
            instruction_withdraw_failed_lottery, 
            instruction_withdraw_lottery_winners, 
            instruction_withdraw_succeed_lottery, 
            instruction_close_lottery_account_and_usdc_token_account,
            instruction_withdraw_and_close_succeed_user
        }, 
        processor::{
            get_lottery_literal_seed,
            Processor
        }, 
        program::ID as LOTTERY_PROGRAM_ID, 
        state::{
            Config,
            Lottery,
            User
        }
    }, 

    solana_program_test::{
        processor,
        tokio,
        ProgramTest,
        ProgramTestContext
    }, 

    solana_sdk::{
        account::{
            Account as SolanaAccount,
            AccountSharedData as SolanaSharedDataAccount
        }, 
        clock::Clock, 
        instruction::{
            AccountMeta,
            Instruction, 
            InstructionError
        }, 
        native_token::sol_to_lamports, 
        program_option::COption, 
        program_pack::Pack, 
        pubkey::Pubkey, 
        rent::{
            ACCOUNT_STORAGE_OVERHEAD, 
            DEFAULT_EXEMPTION_THRESHOLD, 
            DEFAULT_LAMPORTS_PER_BYTE_YEAR
        }, 
        signature::Signer, 
        signer::keypair::Keypair, 
        system_program::ID as SYSTEM_PROGRAM_ID, 
        transaction::{
            Transaction,
            TransactionError
        }
    }, 

    spl_associated_token_account::get_associated_token_address, 

    spl_token::{
        state::{
            Account as TokenAccount,
            AccountState as TokenAccountState,
            Mint as MintAccount
        },
        ID as TOKEN_STANDARD_PROGRAM_ID
    }, 
    
    std::{
        cell::RefCell, 
        rc::Rc, 
        str::FromStr
    }
};

////////////////////////////////////// Helper-Functions ///////////////////////////////
fn setup_program_test(program_id: Pubkey) -> ProgramTest {
    ProgramTest::new(
        "sol_lottery",
        program_id,
        processor!(Processor::process)
    )
}

fn change_clock_sysvar(
    program_test_context: &ProgramTestContext,
    unix_timestamp: i64
) {
    program_test_context.set_sysvar::<Clock>(
        &Clock {
            unix_timestamp,
            ..Clock::default()
        }
    );
}
////////////////////////////////////// Helper-Functions ///////////////////////////////

////////////////////////////////////// Config Instructions
#[tokio::test]
async fn test_create_and_initialize_program_config_account() {
    let config_authority = Keypair::from_bytes(
        &[
            122,112,132,66,72,241,212,228,223,152,188,20,54,92,84,126,221,244,
            138,21,8,4,222,137,47,106,115,230,203,52,23,198,32,81,112,62,73,49,
            48,111,211,251,81,128,189,104,201,142,76,126,117,246,46,33,25,166,
            226,237,190,75,151,133,135,177
        ]
    ).unwrap();
    let lottery_program_id = LOTTERY_PROGRAM_ID;
    let pt = setup_program_test(lottery_program_id);

    let mut ptc = pt.start_with_context().await;

    // failure - config account authority is not the signer
    {
        let lottery_creation_fee = 5_000000_u64;
        let lottery_tickets_fee = 3.0_f64;
        let maximum_number_of_winners = 10_u8;
        let pyth_price_receiver_programid = Pubkey::from_str("rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ").unwrap();
        let usdc_mint_account = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
        let maximum_time_of_price_feed_age = 10_u8;
        let minimum_tickets_to_be_sold_in_lottery = 20_u8;
        let pyth_price_feed_accounts: [Pubkey; 3] = [
            Pubkey::from_str("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE").unwrap(),
            Pubkey::from_str("4cSM2e6rvbGQUFiJbqytoVMi5GgghSMr8LwVrT9VPSPo").unwrap(),
            Pubkey::from_str("42amVS4KgzR9rA28tkVYqVXjq9Qa8dcZQMbH5EYFX6XC").unwrap()
        ];
        let maximum_time_for_lottery_account = 1000_u32;
        let treasury = Pubkey::new_from_array([6; 32]);
        let pyth_price_feed_ids: [String; 3] = [
            "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d".to_string(),
            "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43".to_string(),
            "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace".to_string()
        ];
        let max_lottery_description_bytes = 200u64;
    
        let instruction_data = Instructions::CreateAndInitializeProgramConfigAccount {
            authority: config_authority.pubkey(),
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
        }.try_to_vec().unwrap();
    
        let config_account_pda_account = Pubkey::find_program_address(
            &[
                b"solottery_program_config_account"
            ],
            &lottery_program_id
        ).0;
    
        let instruction_accounts = vec![
            AccountMeta::new_readonly(config_authority.pubkey(), false),
            AccountMeta::new(ptc.payer.pubkey(), true),
            AccountMeta::new(config_account_pda_account, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false)
        ];
    
        let instruction = Instruction {
            program_id: lottery_program_id,
            data: instruction_data,
            accounts: instruction_accounts
        };
    
        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[ &ptc.payer ],
            ptc.last_blockhash
        );
        
        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();
    
        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::MissingRequiredSignature
            )
        );
    }
    // failure - config account authority is not the signer

    // failure - config account passed authority in not valid 
    {
        let config_account_auth = Keypair::new();
        let lottery_creation_fee = 5_000000_u64;
        let lottery_tickets_fee = 3.0_f64;
        let maximum_number_of_winners = 10_u8;
        let pyth_price_receiver_programid = Pubkey::from_str("rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ").unwrap();
        let usdc_mint_account = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
        let maximum_time_of_price_feed_age = 10_u8;
        let minimum_tickets_to_be_sold_in_lottery = 20_u8;
        let pyth_price_feed_accounts: [Pubkey; 3] = [
            Pubkey::from_str("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE").unwrap(),
            Pubkey::from_str("4cSM2e6rvbGQUFiJbqytoVMi5GgghSMr8LwVrT9VPSPo").unwrap(),
            Pubkey::from_str("42amVS4KgzR9rA28tkVYqVXjq9Qa8dcZQMbH5EYFX6XC").unwrap()
        ];
        let maximum_time_for_lottery_account = 1000_u32;
        let treasury = Pubkey::new_from_array([6; 32]);
        let pyth_price_feed_ids: [String; 3] = [
            "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d".to_string(),
            "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43".to_string(),
            "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace".to_string()
        ];
        let max_lottery_description_bytes = 200u64;
    
        let instruction_data = Instructions::CreateAndInitializeProgramConfigAccount {
            authority: config_authority.pubkey(),
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
        }.try_to_vec().unwrap();
    
        let config_account_pda_account = Pubkey::find_program_address(
            &[
                b"solottery_program_config_account"
            ],
            &lottery_program_id
        ).0;
    
        let instruction_accounts = vec![
            AccountMeta::new_readonly(config_account_auth.pubkey(), true),
            AccountMeta::new(ptc.payer.pubkey(), true),
            AccountMeta::new(config_account_pda_account, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false)
        ];
    
        let instruction = Instruction {
            program_id: lottery_program_id,
            data: instruction_data,
            accounts: instruction_accounts
        };
    
        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_account_auth
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidConfigAuthority as u32
                )
            )
        );
    }
    // failure - config account passed authority in not valid 

    // failure - invalid config_account with seeds has been passed
    {
        let lottery_creation_fee = 5_000000_u64;
        let lottery_tickets_fee = 3.0_f64;
        let maximum_number_of_winners = 10_u8;
        let pyth_price_receiver_programid = Pubkey::from_str("rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ").unwrap();
        let usdc_mint_account = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
        let maximum_time_of_price_feed_age = 10_u8;
        let minimum_tickets_to_be_sold_in_lottery = 20_u8;
        let pyth_price_feed_accounts: [Pubkey; 3] = [
            Pubkey::from_str("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE").unwrap(),
            Pubkey::from_str("4cSM2e6rvbGQUFiJbqytoVMi5GgghSMr8LwVrT9VPSPo").unwrap(),
            Pubkey::from_str("42amVS4KgzR9rA28tkVYqVXjq9Qa8dcZQMbH5EYFX6XC").unwrap()
        ];
        let maximum_time_for_lottery_account = 1000_u32;
        let treasury = Pubkey::new_from_array([6; 32]);
        let pyth_price_feed_ids: [String; 3] = [
            "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d".to_string(),
            "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43".to_string(),
            "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace".to_string()
        ];
        let max_lottery_description_bytes = 200u64;
    
        let instruction_data = Instructions::CreateAndInitializeProgramConfigAccount {
            authority: config_authority.pubkey(),
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
        }.try_to_vec().unwrap();
    
        let config_account_pda_account = Pubkey::find_program_address(
            &[
                b"solottery_program_config_accounT"
            ],
            &lottery_program_id
        ).0;
    
        let instruction_accounts = vec![
            AccountMeta::new_readonly(config_authority.pubkey(), true),
            AccountMeta::new(ptc.payer.pubkey(), true),
            AccountMeta::new(config_account_pda_account, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false)
        ];
    
        let instruction = Instruction {
            program_id: lottery_program_id,
            data: instruction_data,
            accounts: instruction_accounts
        };
    
        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_authority
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::InvalidSeeds
            )
        );
    }
    // failure - invalid config_account with seeds has been passed

    // failure - cannot create a config_account twice
    {
        let lottery_creation_fee = 5_000000_u64;
        let lottery_tickets_fee = 3.0_f64;
        let maximum_number_of_winners = 10_u8;
        let pyth_price_receiver_programid = Pubkey::from_str("rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ").unwrap();
        let usdc_mint_account = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
        let maximum_time_of_price_feed_age = 10_u8;
        let minimum_tickets_to_be_sold_in_lottery = 20_u8;
        let pyth_price_feed_accounts: [Pubkey; 3] = [
            Pubkey::from_str("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE").unwrap(),
            Pubkey::from_str("4cSM2e6rvbGQUFiJbqytoVMi5GgghSMr8LwVrT9VPSPo").unwrap(),
            Pubkey::from_str("42amVS4KgzR9rA28tkVYqVXjq9Qa8dcZQMbH5EYFX6XC").unwrap()
        ];
        let maximum_time_for_lottery_account = 1000_u32;
        let treasury = Pubkey::new_from_array([6; 32]);
        let pyth_price_feed_ids: [String; 3] = [
            "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d".to_string(),
            "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43".to_string(),
            "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace".to_string()
        ];
        let max_lottery_description_bytes = 200u64;
    
        let instruction_data = Instructions::CreateAndInitializeProgramConfigAccount {
            authority: config_authority.pubkey(),
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
            pyth_price_feed_ids: pyth_price_feed_ids.clone(),
            max_lottery_description_bytes
        }.try_to_vec().unwrap();
    
        let config_account_pda_account = Pubkey::find_program_address(
            &[
                b"solottery_program_config_account"
            ],
            &lottery_program_id
        ).0;
    
        let instruction_accounts = vec![
            AccountMeta::new_readonly(config_authority.pubkey(), true),
            AccountMeta::new(ptc.payer.pubkey(), true),
            AccountMeta::new(config_account_pda_account, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false)
        ];
    
        let instruction = Instruction {
            program_id: lottery_program_id,
            data: instruction_data,
            accounts: instruction_accounts
        };
    
        let tx = Transaction::new_signed_with_payer(
            &[
                instruction.clone(),
                instruction
            ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_authority
            ],
            ptc.last_blockhash
        );
        
        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                1,
                InstructionError::Custom(
                    solana_sdk::system_instruction::SystemError::AccountAlreadyInUse as u32
                )
            )
        );
    }
    // failure - cannot create a config_account twice

    // success
    {
        let lottery_creation_fee = 5_000000_u64;
        let lottery_tickets_fee = 3.0_f64;
        let maximum_number_of_winners = 10_u8;
        let pyth_price_receiver_programid = Pubkey::from_str("rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ").unwrap();
        let usdc_mint_account = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
        let maximum_time_of_price_feed_age = 10_u8;
        let minimum_tickets_to_be_sold_in_lottery = 20_u8;
        let pyth_price_feed_accounts: [Pubkey; 3] = [
            Pubkey::from_str("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE").unwrap(),
            Pubkey::from_str("4cSM2e6rvbGQUFiJbqytoVMi5GgghSMr8LwVrT9VPSPo").unwrap(),
            Pubkey::from_str("42amVS4KgzR9rA28tkVYqVXjq9Qa8dcZQMbH5EYFX6XC").unwrap()
        ];
        let maximum_time_for_lottery_account = 1000_u32;
        let treasury = Pubkey::new_from_array([6; 32]);
        let pyth_price_feed_ids: [String; 3] = [
            "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d".to_string(),
            "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43".to_string(),
            "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace".to_string()
        ];
        let max_lottery_description_bytes = 200u64;
    
        let instruction_data = Instructions::CreateAndInitializeProgramConfigAccount {
            authority: config_authority.pubkey(),
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
            pyth_price_feed_ids: pyth_price_feed_ids.clone(),
            max_lottery_description_bytes
        }.try_to_vec().unwrap();
    
        let config_account_pda_account = Pubkey::find_program_address(
            &[
                b"solottery_program_config_account"
            ],
            &lottery_program_id
        ).0;
    
        let instruction_accounts = vec![
            AccountMeta::new_readonly(config_authority.pubkey(), true),
            AccountMeta::new(ptc.payer.pubkey(), true),
            AccountMeta::new(config_account_pda_account, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false)
        ];
    
        let instruction = Instruction {
            program_id: lottery_program_id,
            data: instruction_data,
            accounts: instruction_accounts
        };
    
        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_authority
            ],
            ptc.last_blockhash
        );
        
        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();
    
        let SolanaAccount { data: config_account_data, owner: config_account_owner, .. } = ptc
            .banks_client
            .get_account(config_account_pda_account)
            .await
            .unwrap()
            .unwrap();
    
        assert_eq!(
            config_account_owner,
            lottery_program_id,
            "invalid config-account's owner."
        );

        assert_eq!(
            config_account_data.len(),
            Config::LEN,
            "invalid config_account's data length!"
        );
    
        let config_account = Config::deserialize(
            &mut &config_account_data[..]
        ).unwrap();

        assert_eq!(
            config_account.authority,
            config_authority.pubkey(),
            "invalid config authority."
        );

        assert_eq!(
            config_account.discriminator,
            Config::get_discriminator(),
            "invalid config account discriminator."
        );

        assert_eq!(
            config_account.is_pause,
            bool::default(),
            "invalid is_pause flag."
        );

        assert_eq!(
            config_account.latest_update_time,
            i64::default(),
            "invalid latest update time."
        );

        assert_eq!(
            config_account.lottery_creation_fee,
            lottery_creation_fee,
            "invalid lottery creation fee."
        );

        assert_eq!(
            config_account.lottery_tickets_fee,
            lottery_tickets_fee,
            "invalid lottery tickets fee."
        );

        assert_eq!(
            config_account.maximum_number_of_winners,
            maximum_number_of_winners,
            "invalid max number of winner in config_account."
        );

        assert_eq!(
            config_account.maximum_time_for_lottery_account,
            maximum_time_for_lottery_account,
            "invalid max time for lottery account in config_account."
        );

        assert_eq!(
            config_account.maximum_time_of_price_feed_age,
            maximum_time_of_price_feed_age,
            "invalid max time of price feed age in config_account."
        );

        assert_eq!(
            config_account.minimum_tickets_to_be_sold_in_lottery,
            minimum_tickets_to_be_sold_in_lottery,
            "invalid min tickets to be sold in lottery in config_account."
        );

        assert_eq!(
            config_account.usdc_mint_account,
            usdc_mint_account,
            "invalid protocol mint account."
        );

        assert_eq!(
            config_account.pyth_price_feed_accounts,
            pyth_price_feed_accounts,
            "invalid pyth price feed accounts."
        );

        assert_eq!(
            config_account.pyth_price_feed_ids,
            pyth_price_feed_ids,
            "invalid pyth price feed ids."
        );

        assert_eq!(
            config_account.pyth_price_receiver_programid,
            pyth_price_receiver_programid,
            "invalid pyth price receiver program-id."
        );
    }
    // success
}

#[tokio::test]
async fn test_change_fee_of_lottery_creation() {
    let lottery_program_id = LOTTERY_PROGRAM_ID;
    let mut pt = setup_program_test(lottery_program_id);

    //////////////////////? add config account
    let config_account_pda = Pubkey::find_program_address(
        &[
            b"solottery_program_config_account"
        ],
        &lottery_program_id
    );

    let config_authority = Keypair::new();
    let config_account = Config {
        discriminator: Config::get_discriminator(),
        canonical_bump: config_account_pda.1,
        lottery_creation_fee: 5_000000, // 5 USDC
        maximum_number_of_winners: 10,
        usdc_mint_account: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
        maximum_time_for_lottery_account: 1000,
        minimum_tickets_to_be_sold_in_lottery: 20,
        authority: config_authority.pubkey(),
        ..Config::default()
    };
    let config_account_data = config_account.try_to_vec().unwrap();

    let config_solana_account = SolanaAccount {
        owner: lottery_program_id,
        data: config_account_data,
        lamports: sol_to_lamports(0.0009),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account_pda.0,
        config_solana_account
    );
    //////////////////////? add config account  
    
    let mut ptc = pt.start_with_context().await;

    // failure - config account authority is not the signer
    {
        let instruction_data = Instructions::ChangeFeeOfLotteryCreation { new_fee: 10_000000 };

        let instruction_accounts = vec![
            AccountMeta::new(config_account_pda.0, false),
            AccountMeta::new_readonly(config_authority.pubkey(), false)
        ];

        let instruction = Instruction::new_with_borsh::<Instructions>(
            lottery_program_id,
            &instruction_data,
            instruction_accounts
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[ &ptc.payer ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::MissingRequiredSignature
            )
        );
    }
    // failure - config account authority is not the signer

    // failure - invalid config account authority
    {
        let unknown_account = Keypair::new();
        let instruction_data = Instructions::ChangeFeeOfLotteryCreation { new_fee: 10_000000 };

        let instruction_accounts = vec![
            AccountMeta::new(config_account_pda.0, false),
            AccountMeta::new_readonly(unknown_account.pubkey(), true)
        ];

        let instruction = Instruction::new_with_borsh::<Instructions>(
            lottery_program_id,
            &instruction_data,
            instruction_accounts
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &unknown_account
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidConfigAuthority as u32
                )
            )
        );
    }
    // failure - invalid config account authority
    
    // success
    {
        let instruction_data = Instructions::ChangeFeeOfLotteryCreation { new_fee: 10_000000 };

        let instruction_accounts = vec![
            AccountMeta::new(config_account_pda.0, false),
            AccountMeta::new_readonly(config_authority.pubkey(), true)
        ];

        let instruction = Instruction::new_with_borsh::<Instructions>(
            lottery_program_id,
            &instruction_data,
            instruction_accounts
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_authority
            ],
            ptc.last_blockhash
        );

        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        let SolanaAccount { data: config_account_data, .. } = ptc
            .banks_client
            .get_account(config_account_pda.0)
            .await
            .unwrap()
            .unwrap();

        let Config { lottery_creation_fee, .. } = Config::deserialize(
            &mut &config_account_data[..]
        ).unwrap();

        assert_eq!(
            lottery_creation_fee,
            10_000000,
            "invalid new fee!"
        );
    }
    // success
}

#[tokio::test]
async fn test_claim_protocol_fees() {
    let lottery_program_id = LOTTERY_PROGRAM_ID;
    let mut pt = setup_program_test(lottery_program_id);

    //////////////////////? add config account
    let config_account_pda = Pubkey::find_program_address(
        &[
            b"solottery_program_config_account"
        ],
        &lottery_program_id
    );
    
    let config_account_authority = Keypair::new();
    let config_account = Config {
        discriminator: Config::get_discriminator(),
        canonical_bump: config_account_pda.1,
        lottery_creation_fee: 5_000000, // 5 USDC
        maximum_number_of_winners: 10,
        usdc_mint_account: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
        maximum_time_for_lottery_account: 1000,
        minimum_tickets_to_be_sold_in_lottery: 20,
        authority: config_account_authority.pubkey(),
        treasury: Pubkey::new_from_array([1; 32]),
        ..Config::default()
    };
    let config_account_data = config_account.try_to_vec().unwrap();
    
    let config_solana_account = SolanaAccount {
        owner: lottery_program_id,
        data: config_account_data,
        lamports: sol_to_lamports(0.0009),
        ..SolanaAccount::default()
    };
    
    pt.add_account(
        config_account_pda.0,
        config_solana_account
    );
    //////////////////////? add config account
    
    //////////////////////? add protocol mint account
    let protocol_mint_account = MintAccount {
        mint_authority: COption::None,
        freeze_authority: COption::None,
        decimals: 6,
        supply: 3000_000000,
        is_initialized: true
    };

    let mut protocol_mint_account_data: [u8; MintAccount::LEN] = [0; MintAccount::LEN];
    MintAccount::pack(
        protocol_mint_account,
        protocol_mint_account_data.as_mut_slice()
    ).unwrap();

    let protocol_mint_solana_account = SolanaAccount {
        data: protocol_mint_account_data.to_vec(),
        lamports: solana_sdk::native_token::sol_to_lamports(1.0),
        owner: TOKEN_STANDARD_PROGRAM_ID,
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account.usdc_mint_account,
        protocol_mint_solana_account
    );
    //////////////////////? add protocol mint account
    
    //////////////////////? add treasury token account
    let treasury_account_pubkey = Pubkey::new_from_array([1; 32]);
    let treasury_account = TokenAccount {
        amount: 195_000000,
        mint: config_account.usdc_mint_account,
        state: TokenAccountState::Initialized,
        ..TokenAccount::default()
    };

    let mut treasury_account_data: [u8; TokenAccount::LEN] = [0; TokenAccount::LEN];
    TokenAccount::pack(
        treasury_account,
        treasury_account_data.as_mut_slice()
    ).unwrap();

    let treasury_solana_account = SolanaAccount {
        owner: TOKEN_STANDARD_PROGRAM_ID,
        lamports: solana_sdk::native_token::sol_to_lamports(1.0),
        data: treasury_account_data.to_vec(),
        ..SolanaAccount::default()
    };

    pt.add_account(
        treasury_account_pubkey,
        treasury_solana_account
    );
    //////////////////////? add treasury token account
    
    //////////////////////? add lottery account (1)
    let lottery_account_1_pda = Pubkey::find_program_address(
        &[
            b"lottery_account",
            Pubkey::default().to_bytes().as_slice(),
            get_lottery_literal_seed(&String::from("1")).as_slice()
        ],
        &lottery_program_id
    );

    let mut lottery_account_1 = Lottery {
        discriminator: Lottery::get_discriminator(),
        canonical_bump: lottery_account_1_pda.1,
        lottery_creation_fee: 5_000000,
        protocol_fee: 300_000000,
        starting_time: 100,
        ending_time: 200,
        minimum_tickets_amount_required_to_be_sold: 100,
        tickets_total_amount: 100,
        lottery_description: String::from("1"),
        ..Lottery::default()
    };

    let lottey_solana_account_1 = SolanaAccount {
        owner: lottery_program_id,
        lamports: solana_sdk::native_token::sol_to_lamports(1.0),
        data: lottery_account_1.try_to_vec().unwrap(),
        ..SolanaAccount::default()
    };

    pt.add_account(
        lottery_account_1_pda.0,
        lottey_solana_account_1
    );
    //////////////////////? add lottery account (1)
    
    //////////////////////? add lottery account (2)
    let lottery_account_2_pda = Pubkey::find_program_address(
        &[
            b"lottery_account",
            Pubkey::default().to_bytes().as_slice(),
            get_lottery_literal_seed(&String::from("2")).as_slice()
        ],
        &lottery_program_id
    );

    let lottery_account_2 = Lottery {
        discriminator: Lottery::get_discriminator(),
        canonical_bump: lottery_account_2_pda.1,
        lottery_creation_fee: 5_000000,
        protocol_fee: 495_000000,
        starting_time: 100,
        ending_time: 200,
        minimum_tickets_amount_required_to_be_sold: 50,
        tickets_total_amount: 120,
        lottery_description: String::from("2"),
        ..Lottery::default()
    };

    let lottey_solana_account_2 = SolanaAccount {
        owner: lottery_program_id,
        lamports: solana_sdk::native_token::sol_to_lamports(1.0),
        data: lottery_account_2.try_to_vec().unwrap(),
        ..SolanaAccount::default()
    };

    pt.add_account(
        lottery_account_2_pda.0,
        lottey_solana_account_2
    );
    //////////////////////? add lottery account (2)
    
    //////////////////////? add lottery_accounts_1's associated token account
    let lottery_account_1_ata_pubkey = get_associated_token_address(
        &lottery_account_1_pda.0,
        &config_account.usdc_mint_account
    );
    let lottery_account_1_ata = TokenAccount {
        amount: 1500_000000,
        mint: config_account.usdc_mint_account,
        state: TokenAccountState::Initialized,
        owner: lottery_account_1_pda.0,
        ..TokenAccount::default()
    };

    let mut lottery_account_1_ata_data: [u8; TokenAccount::LEN] = [0; TokenAccount::LEN];
    TokenAccount::pack(
        lottery_account_1_ata,
        lottery_account_1_ata_data.as_mut_slice()
    ).unwrap();

    let lottery_account_1_ata_solana_account = SolanaAccount {
        owner: TOKEN_STANDARD_PROGRAM_ID,
        lamports: solana_sdk::native_token::sol_to_lamports(1.0),
        data: lottery_account_1_ata_data.to_vec(),
        ..SolanaAccount::default()
    };

    pt.add_account(
        lottery_account_1_ata_pubkey,
        lottery_account_1_ata_solana_account
    );
    //////////////////////? add lottery_accounts_1's associated token account
    
    //////////////////////? add lottery_accounts_2's associated token account
    let lottery_account_2_ata_pubkey = get_associated_token_address(
        &lottery_account_2_pda.0,
        &config_account.usdc_mint_account
    );
    let lottery_account_2_ata = TokenAccount {
        amount: 1500_000000,
        mint: config_account.usdc_mint_account,
        state: TokenAccountState::Initialized,
        owner: lottery_account_2_pda.0,
        ..TokenAccount::default()
    };

    let mut lottery_account_2_ata_data: [u8; TokenAccount::LEN] = [0; TokenAccount::LEN];
    TokenAccount::pack(
        lottery_account_2_ata,
        lottery_account_2_ata_data.as_mut_slice()
    ).unwrap();

    let lottery_account_2_ata_solana_account = SolanaAccount {
        owner: TOKEN_STANDARD_PROGRAM_ID,
        lamports: solana_sdk::native_token::sol_to_lamports(1.0),
        data: lottery_account_2_ata_data.to_vec(),
        ..SolanaAccount::default()
    };

    pt.add_account(
        lottery_account_2_ata_pubkey,
        lottery_account_2_ata_solana_account
    );
    //////////////////////? add lottery_accounts_2's associated token account
    
    let mut ptc = pt.start_with_context().await;

    // failure - invalid amount of accounts
    {
        let instruction_data = Instructions::ClaimProtocolFees { n: 2 };
        let instruction_accounts = vec![
            AccountMeta::new_readonly(config_account_pda.0, false),
            AccountMeta::new_readonly(config_account_authority.pubkey(), true),
            AccountMeta::new(treasury_account_pubkey, false),
            AccountMeta::new_readonly(config_account.usdc_mint_account, false),
            AccountMeta::new_readonly(TOKEN_STANDARD_PROGRAM_ID, false),
            AccountMeta::new(lottery_account_1_pda.0, false),
            AccountMeta::new(lottery_account_2_pda.0, false),
            AccountMeta::new(lottery_account_1_ata_pubkey, false)
        ];
        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &instruction_data,
            instruction_accounts
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_account_authority
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::NotEnoughAccountKeys
            )
        );
    }
    // failure - invalid amount of accounts

    // failure - invalid "n" patameter
    {
        let instruction_data = Instructions::ClaimProtocolFees { n: 0 };
        let instruction_accounts = vec![
            AccountMeta::new_readonly(config_account_pda.0, false),
            AccountMeta::new_readonly(config_account_authority.pubkey(), true),
            AccountMeta::new(treasury_account_pubkey, false),
            AccountMeta::new_readonly(config_account.usdc_mint_account, false),
            AccountMeta::new_readonly(TOKEN_STANDARD_PROGRAM_ID, false)
        ];
        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &instruction_data,
            instruction_accounts
        );
    
        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_account_authority
            ],
            ptc.last_blockhash
        );
    
        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidNParameter as u32
                )
            )
        );
    }
    // failure - invalid "n" patameter

    // failure - invalid config_account's authoirty
    {
        let unknown_authority = Keypair::new();
        let instruction_data = Instructions::ClaimProtocolFees { n: 2 };
        let instruction_accounts = vec![
            AccountMeta::new_readonly(config_account_pda.0, false),
            AccountMeta::new_readonly(unknown_authority.pubkey(), true),
            AccountMeta::new(treasury_account_pubkey, false),
            AccountMeta::new_readonly(config_account.usdc_mint_account, false),
            AccountMeta::new_readonly(TOKEN_STANDARD_PROGRAM_ID, false),
            AccountMeta::new(lottery_account_1_pda.0, false),
            AccountMeta::new(lottery_account_2_pda.0, false),
            AccountMeta::new(lottery_account_1_ata_pubkey, false),
            AccountMeta::new(lottery_account_2_ata_pubkey, false)
        ];
        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &instruction_data,
            instruction_accounts
        );
    
        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &unknown_authority
            ],
            ptc.last_blockhash
        );
    
        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidConfigAuthority as u32
                )
            )
        );
    }
    // failure - invalid config_account's authoirty

    // failure - invalid treasury account
    {
        let instruction_data = Instructions::ClaimProtocolFees { n: 2 };
        let instruction_accounts = vec![
            AccountMeta::new_readonly(config_account_pda.0, false),
            AccountMeta::new_readonly(config_account_authority.pubkey(), true),
            AccountMeta::new(Pubkey::new_unique(), false),
            AccountMeta::new_readonly(config_account.usdc_mint_account, false),
            AccountMeta::new_readonly(TOKEN_STANDARD_PROGRAM_ID, false),
            AccountMeta::new(lottery_account_1_pda.0, false),
            AccountMeta::new(lottery_account_2_pda.0, false),
            AccountMeta::new(lottery_account_1_ata_pubkey, false),
            AccountMeta::new(lottery_account_2_ata_pubkey, false)
        ];
        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &instruction_data,
            instruction_accounts
        );
    
        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_account_authority
            ],
            ptc.last_blockhash
        );
    
        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidTreasuryAccount as u32
                )
            )
        );
    }
    // failure - invalid treasury account

    // failure - invalid protocol's mint account
    {
        let instruction_data = Instructions::ClaimProtocolFees { n: 2 };
        let instruction_accounts = vec![
            AccountMeta::new_readonly(config_account_pda.0, false),
            AccountMeta::new_readonly(config_account_authority.pubkey(), true),
            AccountMeta::new(treasury_account_pubkey, false),
            AccountMeta::new_readonly(Pubkey::new_unique(), false),
            AccountMeta::new_readonly(TOKEN_STANDARD_PROGRAM_ID, false),
            AccountMeta::new(lottery_account_1_pda.0, false),
            AccountMeta::new(lottery_account_2_pda.0, false),
            AccountMeta::new(lottery_account_1_ata_pubkey, false),
            AccountMeta::new(lottery_account_2_ata_pubkey, false)
        ];
        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &instruction_data,
            instruction_accounts
        );
    
        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_account_authority
            ],
            ptc.last_blockhash
        );
    
        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidUsdcMintAccount as u32
                )
            )
        );
    }
    // failure - invalid protocol's mint account

    // failure - invalid lottery_account state 
    {
        let instruction_data = Instructions::ClaimProtocolFees { n: 2 };
        let instruction_accounts = vec![
            AccountMeta::new_readonly(config_account_pda.0, false),
            AccountMeta::new_readonly(config_account_authority.pubkey(), true),
            AccountMeta::new(treasury_account_pubkey, false),
            AccountMeta::new_readonly(config_account.usdc_mint_account, false),
            AccountMeta::new_readonly(TOKEN_STANDARD_PROGRAM_ID, false),
            AccountMeta::new(lottery_account_1_pda.0, false),
            AccountMeta::new(lottery_account_2_pda.0, false),
            AccountMeta::new(lottery_account_1_ata_pubkey, false),
            AccountMeta::new(lottery_account_2_ata_pubkey, false)
        ];
        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &instruction_data,
            instruction_accounts
        );
    
        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_account_authority
            ],
            ptc.last_blockhash
        );
    
        change_clock_sysvar(&ptc, 150);
    
        let error = ptc
            .banks_client
            .process_transaction(tx.clone())
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidLotteryState as u32
                )
            )
        );
    }
    // failure - invalid lottery_account state 

    // failure - lottery's protocol-fee already claimed
    {
        let instruction_data = Instructions::ClaimProtocolFees { n: 2 };
        let instruction_accounts = vec![
            AccountMeta::new_readonly(config_account_pda.0, false),
            AccountMeta::new_readonly(config_account_authority.pubkey(), true),
            AccountMeta::new(treasury_account_pubkey, false),
            AccountMeta::new_readonly(config_account.usdc_mint_account, false),
            AccountMeta::new_readonly(TOKEN_STANDARD_PROGRAM_ID, false),
            AccountMeta::new(lottery_account_1_pda.0, false),
            AccountMeta::new(lottery_account_2_pda.0, false),
            AccountMeta::new(lottery_account_1_ata_pubkey, false),
            AccountMeta::new(lottery_account_2_ata_pubkey, false)
        ];
        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &instruction_data,
            instruction_accounts
        );
        
        //? Rewrite previous lottery account
        lottery_account_1.is_protocol_fee_claimed = true;

        let mut lottery_1_solana_account = SolanaSharedDataAccount::new(
            sol_to_lamports(1.0),
            1000,
            &lottery_program_id
        );
        lottery_1_solana_account.set_data_from_slice(
            &lottery_account_1.try_to_vec().unwrap()
        );

        ptc.set_account(
            &lottery_account_1_pda.0,
            &lottery_1_solana_account
        );
        //? Rewrite previous lottery account

        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_account_authority
            ],
            ptc.last_blockhash
        );

        ptc.set_sysvar::<Clock>(
            &Clock {
                unix_timestamp: 1000,
                ..Clock::default()
            }
        );
    
        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::ProtocolFeeAlreadyClaimed as u32
                )
            )
        );

        //? Rewrite previous lottery account
        lottery_account_1.is_protocol_fee_claimed = false;

        let mut lottery_1_solana_account = SolanaSharedDataAccount::new(
            sol_to_lamports(1.0),
            1000,
            &lottery_program_id
        );
        lottery_1_solana_account.set_data_from_slice(
            &lottery_account_1.try_to_vec().unwrap()
        );

        ptc.set_account(
            &lottery_account_1_pda.0,
            &lottery_1_solana_account
        );
        //? Rewrite previous lottery account
    }
    // failure - lottery's protocol-fee already claimed

    // faiilure - invalid lottery's associated token account
    {
        let instruction_data = Instructions::ClaimProtocolFees { n: 2 };
        let instruction_accounts = vec![
            AccountMeta::new_readonly(config_account_pda.0, false),
            AccountMeta::new_readonly(config_account_authority.pubkey(), true),
            AccountMeta::new(treasury_account_pubkey, false),
            AccountMeta::new_readonly(config_account.usdc_mint_account, false),
            AccountMeta::new_readonly(TOKEN_STANDARD_PROGRAM_ID, false),
            AccountMeta::new(lottery_account_1_pda.0, false),
            AccountMeta::new(lottery_account_2_pda.0, false),
            AccountMeta::new(Pubkey::new_unique(), false),
            AccountMeta::new(lottery_account_2_ata_pubkey, false)
        ];
        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &instruction_data,
            instruction_accounts
        );
    
        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_account_authority
            ],
            ptc.last_blockhash
        );
    
        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidLotteryAssociatedUsdcTokenAccount as u32
                )
            )
        );
    }
    // faiilure - invalid lottery's associated token account

    // success
    {
        let instruction_data = Instructions::ClaimProtocolFees { n: 2 };
        let instruction_accounts = vec![
            AccountMeta::new_readonly(config_account_pda.0, false),
            AccountMeta::new_readonly(config_account_authority.pubkey(), true),
            AccountMeta::new(treasury_account_pubkey, false),
            AccountMeta::new_readonly(config_account.usdc_mint_account, false),
            AccountMeta::new_readonly(TOKEN_STANDARD_PROGRAM_ID, false),
            AccountMeta::new(lottery_account_1_pda.0, false),
            AccountMeta::new(lottery_account_2_pda.0, false),
            AccountMeta::new(lottery_account_1_ata_pubkey, false),
            AccountMeta::new(lottery_account_2_ata_pubkey, false)
        ];
        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &instruction_data,
            instruction_accounts
        );

        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
    
        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_account_authority
            ],
            ptc.last_blockhash
        );
    
        change_clock_sysvar(&ptc, 350);
    
        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();
    
        let SolanaAccount { data: lottery_account_1_data, .. } = ptc
            .banks_client
            .get_account(lottery_account_1_pda.0)
            .await
            .unwrap()
            .unwrap();
        let SolanaAccount { data: lottery_account_2_data, .. } = ptc
            .banks_client
            .get_account(lottery_account_2_pda.0)
            .await
            .unwrap()
            .unwrap();
    
        let SolanaAccount { data: lottery_account_1_ata_data, .. } = ptc
            .banks_client
            .get_account(lottery_account_1_ata_pubkey)
            .await
            .unwrap()
            .unwrap();
        let SolanaAccount { data: lottery_account_2_ata_data, .. } = ptc
            .banks_client
            .get_account(lottery_account_2_ata_pubkey)
            .await
            .unwrap()
            .unwrap();
    
        let SolanaAccount { data: treasury_account_data, .. } = ptc
            .banks_client
            .get_account(treasury_account_pubkey)
            .await
            .unwrap()
            .unwrap();
    
        let Lottery { is_protocol_fee_claimed, .. } = Lottery::deserialize(
            &mut &lottery_account_1_data[..]
        ).unwrap();
         assert_eq!(
            is_protocol_fee_claimed,
            true,
            "invalid is_claimed for lottery_account_1."
        );
    
        let Lottery { is_protocol_fee_claimed, .. } = Lottery::deserialize(
            &mut &lottery_account_2_data[..]
        ).unwrap();
        assert_eq!(
            is_protocol_fee_claimed,
            true,
            "invalid is_claimed for lottery_account_2."
        );
    
        let TokenAccount { amount, .. } = TokenAccount::unpack(
            &lottery_account_1_ata_data
        ).unwrap();
        assert_eq!(
            amount,
            1500_000000 - (300_000000 + 5_000000),
            "invalid token balance for ata_1."
        );
    
        let TokenAccount { amount, .. } = TokenAccount::unpack(
            &lottery_account_2_ata_data
        ).unwrap();
        assert_eq!(
            amount,
            1500_000000 - (495_000000 + 5_000000),
            "invalid token balance for ata_2."
        );
    
        let TokenAccount { amount, .. } = TokenAccount::unpack(
            &treasury_account_data
        ).unwrap();
        assert_eq!(
            amount,
            195_000000 + (300_000000 + 5_000000) + (495_000000 + 5_000000),
            "invalid token balance for treasury_account."
        );
    }
    // success
}

#[tokio::test]
async fn test_change_config_account_authority() {
    let lottery_program_id = LOTTERY_PROGRAM_ID;
    let mut pt = setup_program_test(lottery_program_id);

    //////////////////////? add config account
    let config_account_pda = Pubkey::find_program_address(
        &[
            b"solottery_program_config_account"
        ],
        &lottery_program_id
    );

    let config_authority = Keypair::new();
    let config_account = Config {
        discriminator: Config::get_discriminator(),
        canonical_bump: config_account_pda.1,
        lottery_creation_fee: 5_000000, // 5 USDC
        maximum_number_of_winners: 10,
        usdc_mint_account: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
        maximum_time_for_lottery_account: 1000,
        minimum_tickets_to_be_sold_in_lottery: 20,
        authority: config_authority.pubkey(),
        pyth_price_feed_ids: [
            "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d".to_string(),
            "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43".to_string(),
            "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace".to_string()
        ],
        pyth_price_feed_accounts: [
            Pubkey::from_str("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE").unwrap(),
            Pubkey::from_str("4cSM2e6rvbGQUFiJbqytoVMi5GgghSMr8LwVrT9VPSPo").unwrap(),
            Pubkey::from_str("42amVS4KgzR9rA28tkVYqVXjq9Qa8dcZQMbH5EYFX6XC").unwrap()
        ],
        ..Config::default()
    };
    let config_account_data = config_account.try_to_vec().unwrap();

    let config_solana_account = SolanaAccount {
        owner: lottery_program_id,
        data: config_account_data,
        lamports: sol_to_lamports(0.0009),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account_pda.0,
        config_solana_account
    );
    //////////////////////? add config account  
    
    let mut ptc = pt.start_with_context().await;

    // failure - invalid config account new authority
    {
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();

        let new_authority = config_account.authority;
        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &Instructions::ChangeConfigAccountAuthority,
            vec![
                AccountMeta::new(config_account_pda.0, false),
                AccountMeta::new_readonly(config_authority.pubkey(), true),
                AccountMeta::new_readonly(new_authority, false)
            ]
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_authority
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidNewConfigAccountAuthority as u32
                )
            )
        );
    }
    // failure - invalid config account new authority

    // failure - missing required signature
    {
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();

        let new_authority = Pubkey::new_from_array([1; 32]);
        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &Instructions::ChangeConfigAccountAuthority,
            vec![
                AccountMeta::new(config_account_pda.0, false),
                AccountMeta::new_readonly(config_authority.pubkey(), false),
                AccountMeta::new_readonly(new_authority, false)
            ]
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[ &ptc.payer ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::MissingRequiredSignature
            )
        );
    }
    // failure - missing required signature

    // failure - invalid config authority
    {
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();

        let new_authority = Pubkey::new_from_array([1; 32]);
        let unkown_user = Keypair::new();
        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &Instructions::ChangeConfigAccountAuthority,
            vec![
                AccountMeta::new(config_account_pda.0, false),
                AccountMeta::new_readonly(unkown_user.pubkey(), true),
                AccountMeta::new_readonly(new_authority, false)
            ]
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &unkown_user
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidConfigAuthority as u32
                )
            )
        );
    }
    // failure - invalid config authority

    // success
    {
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();

        let new_authority = Pubkey::new_from_array([1; 32]);
        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &Instructions::ChangeConfigAccountAuthority,
            vec![
                AccountMeta::new(config_account_pda.0, false),
                AccountMeta::new_readonly(config_authority.pubkey(), true),
                AccountMeta::new_readonly(new_authority, false)
            ]
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_authority
            ],
            ptc.last_blockhash
        );

        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        let SolanaAccount { data, .. } = ptc
            .banks_client
            .get_account(config_account_pda.0)
            .await
            .unwrap()
            .unwrap();

        let Config { authority, .. } = Config::deserialize(
            &mut &data[..]
        ).unwrap();

        assert_eq!(
            authority,
            new_authority,
            "invalid Config account new authority."
        );
    }
    // success
}

#[tokio::test]
async fn test_change_fee_of_tickets() {
    let lottery_program_id = LOTTERY_PROGRAM_ID;
    let mut pt = setup_program_test(lottery_program_id);

    //////////////////////? add config account
    let config_account_pda = Pubkey::find_program_address(
        &[
            b"solottery_program_config_account"
        ],
        &lottery_program_id
    );

    let config_authority = Keypair::new();
    let config_account = Config {
        discriminator: Config::get_discriminator(),
        canonical_bump: config_account_pda.1,
        lottery_creation_fee: 5_000000, // 5 USDC
        maximum_number_of_winners: 10,
        usdc_mint_account: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
        maximum_time_for_lottery_account: 1000,
        minimum_tickets_to_be_sold_in_lottery: 20,
        authority: config_authority.pubkey(),
        pyth_price_feed_ids: [
            "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d".to_string(),
            "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43".to_string(),
            "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace".to_string()
        ],
        pyth_price_feed_accounts: [
            Pubkey::from_str("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE").unwrap(),
            Pubkey::from_str("4cSM2e6rvbGQUFiJbqytoVMi5GgghSMr8LwVrT9VPSPo").unwrap(),
            Pubkey::from_str("42amVS4KgzR9rA28tkVYqVXjq9Qa8dcZQMbH5EYFX6XC").unwrap()
        ],
        lottery_tickets_fee: 3.5,
        ..Config::default()
    };
    let config_account_data = config_account.try_to_vec().unwrap();

    let config_solana_account = SolanaAccount {
        owner: lottery_program_id,
        data: config_account_data,
        lamports: sol_to_lamports(0.0009),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account_pda.0,
        config_solana_account
    );
    //////////////////////? add config account  
    
    let mut ptc = pt.start_with_context().await;

    // suucess
    {
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();

        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &Instructions::ChangeFeeOfTickets { new_fee: 5.5 },
            vec![
                AccountMeta::new(config_account_pda.0, false),
                AccountMeta::new_readonly(config_authority.pubkey(), true)
            ]
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_authority
            ],
            ptc.last_blockhash
        );

        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        let SolanaAccount { data, .. } = ptc
            .banks_client
            .get_account(config_account_pda.0)
            .await
            .unwrap()
            .unwrap();

        let Config { lottery_tickets_fee, .. } = Config::deserialize(
            &mut &data[..]
        ).unwrap();

        assert_eq!(
            lottery_tickets_fee,
            5.5,
            "invalid new lottery tickets fee."
        );
    }
    // success


    // faliure - invalid new lottery tickets fee
    {
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();

        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &Instructions::ChangeFeeOfTickets { new_fee: 100.0 },
            vec![
                AccountMeta::new(config_account_pda.0, false),
                AccountMeta::new_readonly(config_authority.pubkey(), true)
            ]
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_authority
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidLotteryTicketsFee as u32
                )
            )
        );
    }
    // failure - invalid new lottery tickets fee
}

#[tokio::test]
async fn test_change_maximum_number_of_winners() {
    let lottery_program_id = LOTTERY_PROGRAM_ID;
    let mut pt = setup_program_test(lottery_program_id);

    //////////////////////? add config account
    let config_account_pda = Pubkey::find_program_address(
        &[
            b"solottery_program_config_account"
        ],
        &lottery_program_id
    );

    let config_authority = Keypair::new();
    let config_account = Config {
        discriminator: Config::get_discriminator(),
        canonical_bump: config_account_pda.1,
        lottery_creation_fee: 5_000000, // 5 USDC
        maximum_number_of_winners: 8,
        usdc_mint_account: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
        maximum_time_for_lottery_account: 1000,
        minimum_tickets_to_be_sold_in_lottery: 20,
        authority: config_authority.pubkey(),
        pyth_price_feed_ids: [
            "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d".to_string(),
            "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43".to_string(),
            "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace".to_string()
        ],
        pyth_price_feed_accounts: [
            Pubkey::from_str("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE").unwrap(),
            Pubkey::from_str("4cSM2e6rvbGQUFiJbqytoVMi5GgghSMr8LwVrT9VPSPo").unwrap(),
            Pubkey::from_str("42amVS4KgzR9rA28tkVYqVXjq9Qa8dcZQMbH5EYFX6XC").unwrap()
        ],
        lottery_tickets_fee: 3.5,
        ..Config::default()
    };
    let config_account_data = config_account.try_to_vec().unwrap();

    let config_solana_account = SolanaAccount {
        owner: lottery_program_id,
        data: config_account_data,
        lamports: sol_to_lamports(0.0009),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account_pda.0,
        config_solana_account
    );
    //////////////////////? add config account  
    
    let mut ptc = pt.start_with_context().await;

    // failure - invalid new max number of winners
    {
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();

        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &Instructions::ChangeMaximumNumberOfWinners { new_max: 32 },
            vec![
                AccountMeta::new(config_account_pda.0, false),
                AccountMeta::new_readonly(config_authority.pubkey(), true)
            ]
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_authority
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidMaxNumberOfWinners as u32
                )
            )
        );
    }
    // failure - invalid new max number of winners

    // suucess
    {
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();

        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &Instructions::ChangeMaximumNumberOfWinners { new_max: 10 },
            vec![
                AccountMeta::new(config_account_pda.0, false),
                AccountMeta::new_readonly(config_authority.pubkey(), true)
            ]
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_authority
            ],
            ptc.last_blockhash
        );

        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        let SolanaAccount { data, .. } = ptc
            .banks_client
            .get_account(config_account_pda.0)
            .await
            .unwrap()
            .unwrap();

        let Config { maximum_number_of_winners, .. } = Config::deserialize(
            &mut &data[..]
        ).unwrap();

        assert_eq!(
            maximum_number_of_winners,
            10,
            "invalid new maximum number of winners."
        );
    }
    // success
}

#[tokio::test]
async fn test_change_maximum_age_of_price_feed() {
    let lottery_program_id = LOTTERY_PROGRAM_ID;
    let mut pt = setup_program_test(lottery_program_id);

    //////////////////////? add config account
    let config_account_pda = Pubkey::find_program_address(
        &[
            b"solottery_program_config_account"
        ],
        &lottery_program_id
    );

    let config_authority = Keypair::new();
    let config_account = Config {
        discriminator: Config::get_discriminator(),
        canonical_bump: config_account_pda.1,
        lottery_creation_fee: 5_000000, // 5 USDC
        maximum_number_of_winners: 8,
        usdc_mint_account: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
        maximum_time_for_lottery_account: 1000,
        minimum_tickets_to_be_sold_in_lottery: 20,
        authority: config_authority.pubkey(),
        pyth_price_feed_ids: [
            "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d".to_string(),
            "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43".to_string(),
            "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace".to_string()
        ],
        pyth_price_feed_accounts: [
            Pubkey::from_str("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE").unwrap(),
            Pubkey::from_str("4cSM2e6rvbGQUFiJbqytoVMi5GgghSMr8LwVrT9VPSPo").unwrap(),
            Pubkey::from_str("42amVS4KgzR9rA28tkVYqVXjq9Qa8dcZQMbH5EYFX6XC").unwrap()
        ],
        lottery_tickets_fee: 3.5,
        maximum_time_of_price_feed_age: 10,
        ..Config::default()
    };
    let config_account_data = config_account.try_to_vec().unwrap();

    let config_solana_account = SolanaAccount {
        owner: lottery_program_id,
        data: config_account_data,
        lamports: sol_to_lamports(0.0009),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account_pda.0,
        config_solana_account
    );
    //////////////////////? add config account  
    
    let mut ptc = pt.start_with_context().await;

    // failure - invalid new max number of winners
    {
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();

        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &Instructions::ChangeMaximumAgeOfPriceFeed { new_max: 0 },
            vec![
                AccountMeta::new(config_account_pda.0, false),
                AccountMeta::new_readonly(config_authority.pubkey(), true)
            ]
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_authority
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidMaxPriceFeedAge as u32
                )
            )
        );
    }
    // failure - invalid new max number of winners

    // suucess
    {
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();

        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &Instructions::ChangeMaximumAgeOfPriceFeed { new_max: 15 },
            vec![
                AccountMeta::new(config_account_pda.0, false),
                AccountMeta::new_readonly(config_authority.pubkey(), true)
            ]
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_authority
            ],
            ptc.last_blockhash
        );

        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        let SolanaAccount { data, .. } = ptc
            .banks_client
            .get_account(config_account_pda.0)
            .await
            .unwrap()
            .unwrap();

        let Config { maximum_time_of_price_feed_age, .. } = Config::deserialize(
            &mut &data[..]
        ).unwrap();

        assert_eq!(
            maximum_time_of_price_feed_age,
            15,
            "invalid new maximum age of price feed-id."
        );
    }
    // success
}

#[tokio::test]
async fn test_change_pause_state() {
    let lottery_program_id = LOTTERY_PROGRAM_ID;
    let mut pt = setup_program_test(lottery_program_id);

    //////////////////////? add config account
    let config_account_pda = Pubkey::find_program_address(
        &[
            b"solottery_program_config_account"
        ],
        &lottery_program_id
    );

    let config_authority = Keypair::new();
    let config_account = Config {
        discriminator: Config::get_discriminator(),
        canonical_bump: config_account_pda.1,
        lottery_creation_fee: 5_000000, // 5 USDC
        maximum_number_of_winners: 8,
        usdc_mint_account: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
        maximum_time_for_lottery_account: 1000,
        minimum_tickets_to_be_sold_in_lottery: 20,
        authority: config_authority.pubkey(),
        pyth_price_feed_ids: [
            "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d".to_string(),
            "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43".to_string(),
            "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace".to_string()
        ],
        pyth_price_feed_accounts: [
            Pubkey::from_str("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE").unwrap(),
            Pubkey::from_str("4cSM2e6rvbGQUFiJbqytoVMi5GgghSMr8LwVrT9VPSPo").unwrap(),
            Pubkey::from_str("42amVS4KgzR9rA28tkVYqVXjq9Qa8dcZQMbH5EYFX6XC").unwrap()
        ],
        lottery_tickets_fee: 3.5,
        maximum_time_of_price_feed_age: 10,
        is_pause: false,
        ..Config::default()
    };
    let config_account_data = config_account.try_to_vec().unwrap();

    let config_solana_account = SolanaAccount {
        owner: lottery_program_id,
        data: config_account_data,
        lamports: sol_to_lamports(0.0009),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account_pda.0,
        config_solana_account
    );
    //////////////////////? add config account  
    
    let mut ptc = pt.start_with_context().await;

    // suucess
    {
        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &Instructions::ChangePauseState { pause: true },
            vec![
                AccountMeta::new(config_account_pda.0, false),
                AccountMeta::new_readonly(config_authority.pubkey(), true)
            ]
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_authority
            ],
            ptc.last_blockhash
        );

        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();

        let SolanaAccount { data, .. } = ptc
            .banks_client
            .get_account(config_account_pda.0)
            .await
            .unwrap()
            .unwrap();

        let Config { is_pause, .. } = Config::deserialize(
            &mut &data[..]
        ).unwrap();

        assert_eq!(
            is_pause,
            true,
            "invalid new is_pause flag."
        );
    }
    // success
}

#[tokio::test]
async fn test_change_treasury_account() {
    let lottery_program_id = LOTTERY_PROGRAM_ID;
    let mut pt = setup_program_test(lottery_program_id);

    //////////////////////? add config account
    let config_account_pda = Pubkey::find_program_address(
        &[
            b"solottery_program_config_account"
        ],
        &lottery_program_id
    );

    let config_authority = Keypair::new();
    let config_account = Config {
        discriminator: Config::get_discriminator(),
        canonical_bump: config_account_pda.1,
        lottery_creation_fee: 5_000000, // 5 USDC
        maximum_number_of_winners: 8,
        usdc_mint_account: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
        maximum_time_for_lottery_account: 1000,
        minimum_tickets_to_be_sold_in_lottery: 20,
        authority: config_authority.pubkey(),
        pyth_price_feed_ids: [
            "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d".to_string(),
            "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43".to_string(),
            "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace".to_string()
        ],
        pyth_price_feed_accounts: [
            Pubkey::from_str("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE").unwrap(),
            Pubkey::from_str("4cSM2e6rvbGQUFiJbqytoVMi5GgghSMr8LwVrT9VPSPo").unwrap(),
            Pubkey::from_str("42amVS4KgzR9rA28tkVYqVXjq9Qa8dcZQMbH5EYFX6XC").unwrap()
        ],
        lottery_tickets_fee: 3.5,
        maximum_time_of_price_feed_age: 10,
        is_pause: false,
        treasury: Pubkey::new_from_array([33; 32]), // It must be a USDC token account !
        ..Config::default()
    };
    let config_account_data = config_account.try_to_vec().unwrap();

    let config_solana_account = SolanaAccount {
        owner: lottery_program_id,
        data: config_account_data,
        lamports: sol_to_lamports(0.0009),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account_pda.0,
        config_solana_account
    );
    //////////////////////? add config account  
    
    let mut ptc = pt.start_with_context().await;

    // suucess
    {
        let new_treasury_account = Pubkey::new_from_array([55; 32]);
        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &Instructions::ChangeTreasury,
            vec![
                AccountMeta::new(config_account_pda.0, false),
                AccountMeta::new_readonly(config_authority.pubkey(), true),
                AccountMeta::new_readonly(new_treasury_account, false)
            ]
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_authority
            ],
            ptc.last_blockhash
        );

        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();

        let SolanaAccount { data, .. } = ptc
            .banks_client
            .get_account(config_account_pda.0)
            .await
            .unwrap()
            .unwrap();

        let Config { treasury, .. } = Config::deserialize(
            &mut &data[..]
        ).unwrap();

        assert_eq!(
            treasury,
            new_treasury_account,
            "invalid new treasury account."
        );
    }
    // success
}

#[tokio::test]
async fn test_change_protocol_mint_account() {
    let lottery_program_id = LOTTERY_PROGRAM_ID;
    let mut pt = setup_program_test(lottery_program_id);

    //////////////////////? add config account
    let config_account_pda = Pubkey::find_program_address(
        &[
            b"solottery_program_config_account"
        ],
        &lottery_program_id
    );

    let config_authority = Keypair::new();
    let config_account = Config {
        discriminator: Config::get_discriminator(),
        canonical_bump: config_account_pda.1,
        lottery_creation_fee: 5_000000, // 5 USDC
        maximum_number_of_winners: 8,
        usdc_mint_account: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
        maximum_time_for_lottery_account: 1000,
        minimum_tickets_to_be_sold_in_lottery: 20,
        authority: config_authority.pubkey(),
        pyth_price_feed_ids: [
            "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d".to_string(),
            "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43".to_string(),
            "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace".to_string()
        ],
        pyth_price_feed_accounts: [
            Pubkey::from_str("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE").unwrap(),
            Pubkey::from_str("4cSM2e6rvbGQUFiJbqytoVMi5GgghSMr8LwVrT9VPSPo").unwrap(),
            Pubkey::from_str("42amVS4KgzR9rA28tkVYqVXjq9Qa8dcZQMbH5EYFX6XC").unwrap()
        ],
        lottery_tickets_fee: 3.5,
        maximum_time_of_price_feed_age: 10,
        is_pause: false,
        treasury: Pubkey::new_from_array([33; 32]), // It must be a USDC token account !
        ..Config::default()
    };
    let config_account_data = config_account.try_to_vec().unwrap();

    let config_solana_account = SolanaAccount {
        owner: lottery_program_id,
        data: config_account_data,
        lamports: sol_to_lamports(0.0009),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account_pda.0,
        config_solana_account
    );
    //////////////////////? add config account  
    
    let mut ptc = pt.start_with_context().await;

    // suucess
    {
        let new_mint_account = Pubkey::new_from_array([55; 32]);
        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &Instructions::ChangeProtocolMintAccount { new_mint_account },
            vec![
                AccountMeta::new(config_account_pda.0, false),
                AccountMeta::new_readonly(config_authority.pubkey(), true)
            ]
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_authority
            ],
            ptc.last_blockhash
        );

        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();

        let SolanaAccount { data, .. } = ptc
            .banks_client
            .get_account(config_account_pda.0)
            .await
            .unwrap()
            .unwrap();

        let Config { usdc_mint_account, .. } = Config::deserialize(
            &mut &data[..]
        ).unwrap();

        assert_eq!(
            usdc_mint_account,
            new_mint_account,
            "invalid new mint account."
        );
    }
    // success
}

#[tokio::test]
async fn test_change_pyth_price_receiver_program_account() {
    let lottery_program_id = LOTTERY_PROGRAM_ID;
    let mut pt = setup_program_test(lottery_program_id);

    //////////////////////? add config account
    let config_account_pda = Pubkey::find_program_address(
        &[
            b"solottery_program_config_account"
        ],
        &lottery_program_id
    );

    let config_authority = Keypair::new();
    let config_account = Config {
        discriminator: Config::get_discriminator(),
        canonical_bump: config_account_pda.1,
        lottery_creation_fee: 5_000000, // 5 USDC
        maximum_number_of_winners: 8,
        usdc_mint_account: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
        maximum_time_for_lottery_account: 1000,
        minimum_tickets_to_be_sold_in_lottery: 20,
        authority: config_authority.pubkey(),
        pyth_price_feed_ids: [
            "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d".to_string(),
            "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43".to_string(),
            "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace".to_string()
        ],
        pyth_price_feed_accounts: [
            Pubkey::from_str("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE").unwrap(),
            Pubkey::from_str("4cSM2e6rvbGQUFiJbqytoVMi5GgghSMr8LwVrT9VPSPo").unwrap(),
            Pubkey::from_str("42amVS4KgzR9rA28tkVYqVXjq9Qa8dcZQMbH5EYFX6XC").unwrap()
        ],
        lottery_tickets_fee: 3.5,
        maximum_time_of_price_feed_age: 10,
        is_pause: false,
        treasury: Pubkey::new_from_array([33; 32]), // It must be a USDC token account !
        pyth_price_receiver_programid: Pubkey::new_from_array([72; 32]),
        ..Config::default()
    };
    let config_account_data = config_account.try_to_vec().unwrap();

    let config_solana_account = SolanaAccount {
        owner: lottery_program_id,
        data: config_account_data,
        lamports: sol_to_lamports(0.0009),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account_pda.0,
        config_solana_account
    );
    //////////////////////? add config account  
    
    let mut ptc = pt.start_with_context().await;

    // suucess
    {
        let new_pyth_price_receiver_programid = Pubkey::new_from_array([13; 32]);
        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &Instructions::ChangePythPriceReceiverProgramAccount { new_pyth_price_receiver_programid },
            vec![
                AccountMeta::new(config_account_pda.0, false),
                AccountMeta::new_readonly(config_authority.pubkey(), true)
            ]
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_authority
            ],
            ptc.last_blockhash
        );

        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();

        let SolanaAccount { data, .. } = ptc
            .banks_client
            .get_account(config_account_pda.0)
            .await
            .unwrap()
            .unwrap();

        let Config { pyth_price_receiver_programid, .. } = Config::deserialize(
            &mut &data[..]
        ).unwrap();

        assert_eq!(
            pyth_price_receiver_programid,
            new_pyth_price_receiver_programid,
            "invalid new pyth .. program account."
        );
    }
    // success
}

#[tokio::test]
async fn test_change_price_feed_id() {
    let lottery_program_id = LOTTERY_PROGRAM_ID;
    let mut pt = setup_program_test(lottery_program_id);

    //////////////////////? add config account
    let config_account_pda = Pubkey::find_program_address(
        &[
            b"solottery_program_config_account"
        ],
        &lottery_program_id
    );

    let config_authority = Keypair::new();
    let config_account = Config {
        discriminator: Config::get_discriminator(),
        canonical_bump: config_account_pda.1,
        lottery_creation_fee: 5_000000, // 5 USDC
        maximum_number_of_winners: 8,
        usdc_mint_account: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
        maximum_time_for_lottery_account: 1000,
        minimum_tickets_to_be_sold_in_lottery: 20,
        authority: config_authority.pubkey(),
        pyth_price_feed_ids: [
            "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d".to_string(),
            "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43".to_string(),
            "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace".to_string()
        ],
        pyth_price_feed_accounts: [
            Pubkey::from_str("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE").unwrap(),
            Pubkey::from_str("4cSM2e6rvbGQUFiJbqytoVMi5GgghSMr8LwVrT9VPSPo").unwrap(),
            Pubkey::from_str("42amVS4KgzR9rA28tkVYqVXjq9Qa8dcZQMbH5EYFX6XC").unwrap()
        ],
        lottery_tickets_fee: 3.5,
        maximum_time_of_price_feed_age: 10,
        is_pause: false,
        treasury: Pubkey::new_from_array([33; 32]), // It must be a USDC token account !
        pyth_price_receiver_programid: Pubkey::new_from_array([72; 32]),
        ..Config::default()
    };
    let config_account_data = config_account.try_to_vec().unwrap();

    let config_solana_account = SolanaAccount {
        owner: lottery_program_id,
        data: config_account_data,
        lamports: sol_to_lamports(0.0009),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account_pda.0,
        config_solana_account
    );
    //////////////////////? add config account  
    
    let mut ptc = pt.start_with_context().await;

    // suucess
    {
        let index = 2u8;
        let new_price_feed_id = String::from("Ether");
        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &Instructions::ChangePriceFeedId { index, price_feed_id: new_price_feed_id.clone() },
            vec![
                AccountMeta::new(config_account_pda.0, false),
                AccountMeta::new_readonly(config_authority.pubkey(), true)
            ]
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_authority
            ],
            ptc.last_blockhash
        );

        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();

        let SolanaAccount { data, .. } = ptc
            .banks_client
            .get_account(config_account_pda.0)
            .await
            .unwrap()
            .unwrap();

        let Config { pyth_price_feed_ids, .. } = Config::deserialize(
            &mut &data[..]
        ).unwrap();

        assert_eq!(
            pyth_price_feed_ids[2],
            new_price_feed_id,
            "invalid new price feed id."
        );
    }
    // success
}

#[tokio::test]
async fn test_change_price_feed_account() {
    let lottery_program_id = LOTTERY_PROGRAM_ID;
    let mut pt = setup_program_test(lottery_program_id);

    //////////////////////? add config account
    let config_account_pda = Pubkey::find_program_address(
        &[
            b"solottery_program_config_account"
        ],
        &lottery_program_id
    );

    let config_authority = Keypair::new();
    let config_account = Config {
        discriminator: Config::get_discriminator(),
        canonical_bump: config_account_pda.1,
        lottery_creation_fee: 5_000000, // 5 USDC
        maximum_number_of_winners: 8,
        usdc_mint_account: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
        maximum_time_for_lottery_account: 1000,
        minimum_tickets_to_be_sold_in_lottery: 20,
        authority: config_authority.pubkey(),
        pyth_price_feed_ids: [
            "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d".to_string(),
            "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43".to_string(),
            "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace".to_string()
        ],
        pyth_price_feed_accounts: [
            Pubkey::from_str("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE").unwrap(),
            Pubkey::from_str("4cSM2e6rvbGQUFiJbqytoVMi5GgghSMr8LwVrT9VPSPo").unwrap(),
            Pubkey::from_str("42amVS4KgzR9rA28tkVYqVXjq9Qa8dcZQMbH5EYFX6XC").unwrap()
        ],
        lottery_tickets_fee: 3.5,
        maximum_time_of_price_feed_age: 10,
        is_pause: false,
        treasury: Pubkey::new_from_array([33; 32]), // It must be a USDC token account !
        pyth_price_receiver_programid: Pubkey::new_from_array([72; 32]),
        ..Config::default()
    };
    let config_account_data = config_account.try_to_vec().unwrap();

    let config_solana_account = SolanaAccount {
        owner: lottery_program_id,
        data: config_account_data,
        lamports: sol_to_lamports(0.0009),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account_pda.0,
        config_solana_account
    );
    //////////////////////? add config account  
    
    let mut ptc = pt.start_with_context().await;

    // suucess
    {
        let index = 2u8;
        let new_price_feed_account = Pubkey::new_from_array([40; 32]);
        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &Instructions::ChangePriceFeedAccount { index, price_feed_account: new_price_feed_account },
            vec![
                AccountMeta::new(config_account_pda.0, false),
                AccountMeta::new_readonly(config_authority.pubkey(), true)
            ]
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_authority
            ],
            ptc.last_blockhash
        );

        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();

        let SolanaAccount { data, .. } = ptc
            .banks_client
            .get_account(config_account_pda.0)
            .await
            .unwrap()
            .unwrap();

        let Config { pyth_price_feed_accounts, .. } = Config::deserialize(
            &mut &data[..]
        ).unwrap();

        assert_eq!(
            pyth_price_feed_accounts[2],
            new_price_feed_account,
            "invalid new price feed account."
        );
    }
    // success
}

#[tokio::test]
async fn test_change_max_lottery_description_length() {
    let lottery_program_id = LOTTERY_PROGRAM_ID;
    let mut pt = setup_program_test(lottery_program_id);

    //////////////////////? add config account
    let config_account_pda = Pubkey::find_program_address(
        &[
            b"solottery_program_config_account"
        ],
        &lottery_program_id
    );

    let config_authority = Keypair::new();
    let config_account = Config {
        discriminator: Config::get_discriminator(),
        canonical_bump: config_account_pda.1,
        lottery_creation_fee: 5_000000, // 5 USDC
        maximum_number_of_winners: 8,
        usdc_mint_account: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
        maximum_time_for_lottery_account: 1000,
        minimum_tickets_to_be_sold_in_lottery: 20,
        authority: config_authority.pubkey(),
        pyth_price_feed_ids: [
            "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d".to_string(),
            "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43".to_string(),
            "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace".to_string()
        ],
        pyth_price_feed_accounts: [
            Pubkey::from_str("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE").unwrap(),
            Pubkey::from_str("4cSM2e6rvbGQUFiJbqytoVMi5GgghSMr8LwVrT9VPSPo").unwrap(),
            Pubkey::from_str("42amVS4KgzR9rA28tkVYqVXjq9Qa8dcZQMbH5EYFX6XC").unwrap()
        ],
        lottery_tickets_fee: 3.5,
        maximum_time_of_price_feed_age: 10,
        is_pause: false,
        treasury: Pubkey::new_from_array([33; 32]), // It must be a USDC token account !
        pyth_price_receiver_programid: Pubkey::new_from_array([72; 32]),
        max_lottery_description_bytes: 200,
        ..Config::default()
    };
    let config_account_data = config_account.try_to_vec().unwrap();

    let config_solana_account = SolanaAccount {
        owner: lottery_program_id,
        data: config_account_data,
        lamports: sol_to_lamports(0.0009),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account_pda.0,
        config_solana_account
    );
    //////////////////////? add config account  
    
    let mut ptc = pt.start_with_context().await;

    // suucess
    {
        let new_max_lottery_description_length_in_bytes = 1000;
        let instruction = Instruction::new_with_borsh(
            lottery_program_id,
            &Instructions::ChangeMaxLotteryDescriptionLength { new_length: new_max_lottery_description_length_in_bytes },
            vec![
                AccountMeta::new(config_account_pda.0, false),
                AccountMeta::new_readonly(config_authority.pubkey(), true)
            ]
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &config_authority
            ],
            ptc.last_blockhash
        );

        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();

        let SolanaAccount { data, .. } = ptc
            .banks_client
            .get_account(config_account_pda.0)
            .await
            .unwrap()
            .unwrap();

        let Config { max_lottery_description_bytes, .. } = Config::deserialize(
            &mut &data[..]
        ).unwrap();

        assert_eq!(
            max_lottery_description_bytes,
            new_max_lottery_description_length_in_bytes,
            "invalid new max lottery description length."
        );
    }
    // success
}
////////////////////////////////////// Config Instructions

////////////////////////////////////// Lottery Instructions
#[tokio::test]
async fn test_create_and_initialize_lottery_account() {
    let lottery_program_id = LOTTERY_PROGRAM_ID;
    let mut pt = setup_program_test(lottery_program_id);

    //////////////////////? add config account
    let config_account_pda = Pubkey::find_program_address(
        &[
            b"solottery_program_config_account"
        ],
        &lottery_program_id
    );

    let config_account = Config {
        discriminator: Config::get_discriminator(),
        canonical_bump: config_account_pda.1,
        lottery_creation_fee: 5_000000, // 5 USDC
        maximum_number_of_winners: 10,
        usdc_mint_account: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
        maximum_time_for_lottery_account: 1000,
        minimum_tickets_to_be_sold_in_lottery: 20,
        max_lottery_description_bytes: 10,
        ..Config::default()
    };
    let config_account_data = config_account.try_to_vec().unwrap();

    let config_solana_account = SolanaAccount {
        owner: lottery_program_id,
        data: config_account_data,
        lamports: sol_to_lamports(0.0009),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account_pda.0,
        config_solana_account
    );
    //////////////////////? add config account

    //////////////////////? add USDC and Arbitrary mint accounts
    // 1. USDC mint account
    let usdc_mint_account = MintAccount {
        supply: 1000_000000,
        decimals: 6,
        is_initialized: true,
        ..MintAccount::default()
    };

    let mut usdc_mint_account_data: [u8; MintAccount::LEN] = [0; MintAccount::LEN];
    MintAccount::pack(
        usdc_mint_account,
        usdc_mint_account_data.as_mut_slice()
    ).unwrap();

    let uscd_mint_solana_account = SolanaAccount {
        lamports: sol_to_lamports(0.01),
        owner: TOKEN_STANDARD_PROGRAM_ID,
        data: usdc_mint_account_data.to_vec(),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account.usdc_mint_account,
        uscd_mint_solana_account
    );

    // 2. Arbitrary mint account
    let arbitrary_mint_account_addr = Pubkey::new_from_array([3; 32]);
    let arbitrary_mint_account = MintAccount {
        decimals: 3,
        supply: 100_000,
        is_initialized: true,
        ..MintAccount::default()
    };

    let mut arbitrary_mint_account_data: [u8; MintAccount::LEN] = [0; MintAccount::LEN];
    MintAccount::pack(
        arbitrary_mint_account,
        arbitrary_mint_account_data.as_mut_slice()
    ).unwrap();

    let arbitrary_mint_solana_account = SolanaAccount {
        owner: TOKEN_STANDARD_PROGRAM_ID,
        lamports: sol_to_lamports(0.01),
        data: arbitrary_mint_account_data.to_vec(),
        ..SolanaAccount::default()
    };

    pt.add_account(
        arbitrary_mint_account_addr,
        arbitrary_mint_solana_account
    );
    //////////////////////? add USDC and Arbitrary mint accounts

    //////////////////////? add token accounts
    // add funding account for rents, usdc_token_account and arbitrary_token_account
    // 1. add funding account
    let funding_account = Keypair::new();

    let funding_solana_account = SolanaAccount {
        owner: SYSTEM_PROGRAM_ID,
        lamports: sol_to_lamports(10.0),
        ..SolanaAccount::default()
    };

    pt.add_account(
        funding_account.pubkey(),
    funding_solana_account
    );
    
    // 2. add funding usdc token account
    let funding_usdc_token_account_pubkey = Pubkey::new_from_array([4; 32]);
    let funding_usdc_token_account = TokenAccount {
        mint: config_account.usdc_mint_account,
        owner: funding_account.pubkey(),
        amount: 1000_000000,
        state: TokenAccountState::Initialized,
        ..TokenAccount::default()
    };

    let mut funding_usdc_token_account_data: [u8; TokenAccount::LEN] = [0; TokenAccount::LEN];
    TokenAccount::pack(
        funding_usdc_token_account,
        funding_usdc_token_account_data.as_mut_slice()
    ).unwrap();

    let funding_usdc_token_solana_account = SolanaAccount {
        owner: TOKEN_STANDARD_PROGRAM_ID,
        data: funding_usdc_token_account_data.to_vec(),
        lamports: sol_to_lamports(0.01),
        ..SolanaAccount::default()
    };

    pt.add_account(
        funding_usdc_token_account_pubkey,
        funding_usdc_token_solana_account
    );

    // 3. add funding arbitrary token account
    let funding_arbitrary_token_account_pubkey = Pubkey::new_from_array([5; 32]);
    let funding_arbitrary_token_account = TokenAccount {
        mint: arbitrary_mint_account_addr,
        owner: funding_account.pubkey(),
        amount: 100_000,
        state: TokenAccountState::Initialized,
        ..TokenAccount::default()
    };

    let mut funding_arbitrary_token_account_data: [u8; TokenAccount::LEN] = [0; TokenAccount::LEN];
    TokenAccount::pack(
        funding_arbitrary_token_account,
        funding_arbitrary_token_account_data.as_mut_slice()
    ).unwrap();

    let funding_arbitrary_token_solana_account = SolanaAccount {
        owner: TOKEN_STANDARD_PROGRAM_ID,
        lamports: sol_to_lamports(0.01),
        data: funding_arbitrary_token_account_data.to_vec(),
        ..SolanaAccount::default()
    };

    pt.add_account(
        funding_arbitrary_token_account_pubkey,
        funding_arbitrary_token_solana_account
    );

    // add funding account for rents, usdc_token_account and arbitrary_token_account
    //////////////////////? add token accounts
    
    //////////////////////? add lottery_account authority
    let lottery_authority_account = Keypair::new();
    pt.add_account(
        lottery_authority_account.pubkey(),
        SolanaAccount {
            owner: SYSTEM_PROGRAM_ID,
            lamports: sol_to_lamports(1.0),
            ..SolanaAccount::default()
        }
    );
    //////////////////////? add lottery_account authority
    
    //? Start program_test_context
    let mut ptc = pt.start_with_context().await;

    // faliure - invalid lottery's description length
    {
        let lottery_description = String::from("PooriaGG.Solana");
        let lottery_account = Pubkey::find_program_address(
            &[
                b"lottery_account",
                lottery_authority_account.pubkey().to_bytes().as_slice(),
                get_lottery_literal_seed(&lottery_description).as_slice()
            ],
            &lottery_program_id
        ).0;
        
        change_clock_sysvar(&ptc, 550);

        let fund_amount = 100_u64;
        let winners_count = 1_u8;
        let starting_time = 1000_i64;
        let ending_time = 1350_i64;
        let minimum_tickets_amount_required_to_be_sold = 25u32;
        let ticket_price = 1_000000u64;
        let maximum_number_of_tickets_per_user = None;
        
        let instruction = instruction_create_and_initialize_lottery_account(
            lottery_account,
            lottery_authority_account.pubkey(),
            funding_account.pubkey(),
            config_account.usdc_mint_account,
            get_associated_token_address(
                &lottery_account,
                &config_account.usdc_mint_account
            ),
            funding_usdc_token_account_pubkey,
            arbitrary_mint_account_addr,
            get_associated_token_address(
                &lottery_account,
                &arbitrary_mint_account_addr
            ),
            funding_arbitrary_token_account_pubkey,
            TOKEN_STANDARD_PROGRAM_ID,
            spl_associated_token_account::ID,
            SYSTEM_PROGRAM_ID,
            config_account_pda.0,
            fund_amount,
            winners_count,
            starting_time,
            ending_time,
            minimum_tickets_amount_required_to_be_sold,
            ticket_price,
            maximum_number_of_tickets_per_user,
            lottery_description
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &funding_account,
                &lottery_authority_account
            ],
            ptc.last_blockhash
        );

        let error = ptc.
            banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::MaxLotteryDescriptionBytesExceeded as u32
                )
            )
        );
    }
    // faliure - invalid lottery's description length

    // faliure - invalid max time for lottery
    {
        let lottery_description = String::from("AABBCC");
        let lottery_account = Pubkey::find_program_address(
            &[
                b"lottery_account",
                lottery_authority_account.pubkey().to_bytes().as_slice(),
                get_lottery_literal_seed(&lottery_description).as_slice()
            ],
            &lottery_program_id
        ).0;
        
        change_clock_sysvar(&ptc, 550);

        let fund_amount = 100_u64;
        let winners_count = 1_u8;
        let starting_time = 1000_i64;
        let ending_time = 2350_i64;
        let minimum_tickets_amount_required_to_be_sold = 25u32;
        let ticket_price = 1_000000u64;
        let maximum_number_of_tickets_per_user = None;
        
        let instruction = instruction_create_and_initialize_lottery_account(
            lottery_account,
            lottery_authority_account.pubkey(),
            funding_account.pubkey(),
            config_account.usdc_mint_account,
            get_associated_token_address(
                &lottery_account,
                &config_account.usdc_mint_account
            ),
            funding_usdc_token_account_pubkey,
            arbitrary_mint_account_addr,
            get_associated_token_address(
                &lottery_account,
                &arbitrary_mint_account_addr
            ),
            funding_arbitrary_token_account_pubkey,
            TOKEN_STANDARD_PROGRAM_ID,
            spl_associated_token_account::ID,
            SYSTEM_PROGRAM_ID,
            config_account_pda.0,
            fund_amount,
            winners_count,
            starting_time,
            ending_time,
            minimum_tickets_amount_required_to_be_sold,
            ticket_price,
            maximum_number_of_tickets_per_user,
            lottery_description
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &funding_account,
                &lottery_authority_account
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::MaximumTimeExceed as u32
                )
            )
        );
    }
    // faliure - invalid max time for lottery

    // faliure - invalid seeds
    {
        let lottery_description = String::from("AABBCC");
        let lottery_account = Pubkey::find_program_address(
            &[
                b"lottery_accountt",
                lottery_authority_account.pubkey().to_bytes().as_slice(),
                get_lottery_literal_seed(&lottery_description).as_slice()
            ],
            &lottery_program_id
        ).0;
        
        change_clock_sysvar(&ptc, 550);

        let fund_amount = 100_u64;
        let winners_count = 1_u8;
        let starting_time = 1000_i64;
        let ending_time = 1350_i64;
        let minimum_tickets_amount_required_to_be_sold = 25u32;
        let ticket_price = 1_000000u64;
        let maximum_number_of_tickets_per_user = None;
        
        let instruction = instruction_create_and_initialize_lottery_account(
            lottery_account,
            lottery_authority_account.pubkey(),
            funding_account.pubkey(),
            config_account.usdc_mint_account,
            get_associated_token_address(
                &lottery_account,
                &config_account.usdc_mint_account
            ),
            funding_usdc_token_account_pubkey,
            arbitrary_mint_account_addr,
            get_associated_token_address(
                &lottery_account,
                &arbitrary_mint_account_addr
            ),
            funding_arbitrary_token_account_pubkey,
            TOKEN_STANDARD_PROGRAM_ID,
            spl_associated_token_account::ID,
            SYSTEM_PROGRAM_ID,
            config_account_pda.0,
            fund_amount,
            winners_count,
            starting_time,
            ending_time,
            minimum_tickets_amount_required_to_be_sold,
            ticket_price,
            maximum_number_of_tickets_per_user,
            lottery_description
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &funding_account,
                &lottery_authority_account
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::InvalidSeeds
            )
        );
    }
    // faliure - invalid seeds

    // failure - invalid winners count
    {
        let lottery_description = String::from("AABBCC");
        let lottery_account = Pubkey::find_program_address(
            &[
                b"lottery_account",
                lottery_authority_account.pubkey().to_bytes().as_slice(),
                get_lottery_literal_seed(&lottery_description).as_slice()
            ],
            &lottery_program_id
        ).0;
        
        change_clock_sysvar(&ptc, 550);

        let fund_amount = 100_u64;
        let winners_count = 100_u8;
        let starting_time = 1000_i64;
        let ending_time = 1350_i64;
        let minimum_tickets_amount_required_to_be_sold = 25u32;
        let ticket_price = 1_000000u64;
        let maximum_number_of_tickets_per_user = None;
        
        let instruction = instruction_create_and_initialize_lottery_account(
            lottery_account,
            lottery_authority_account.pubkey(),
            funding_account.pubkey(),
            config_account.usdc_mint_account,
            get_associated_token_address(
                &lottery_account,
                &config_account.usdc_mint_account
            ),
            funding_usdc_token_account_pubkey,
            arbitrary_mint_account_addr,
            get_associated_token_address(
                &lottery_account,
                &arbitrary_mint_account_addr
            ),
            funding_arbitrary_token_account_pubkey,
            TOKEN_STANDARD_PROGRAM_ID,
            spl_associated_token_account::ID,
            SYSTEM_PROGRAM_ID,
            config_account_pda.0,
            fund_amount,
            winners_count,
            starting_time,
            ending_time,
            minimum_tickets_amount_required_to_be_sold,
            ticket_price,
            maximum_number_of_tickets_per_user,
            lottery_description
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &funding_account,
                &lottery_authority_account
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidWinnersAmount as u32
                )
            )
        );
    }
    // failure - invalid winners count

    // failure - invalid min tickets required to be sold 
    {
        let lottery_description = String::from("AABBCC");
        let lottery_account = Pubkey::find_program_address(
            &[
                b"lottery_account",
                lottery_authority_account.pubkey().to_bytes().as_slice(),
                get_lottery_literal_seed(&lottery_description).as_slice()
            ],
            &lottery_program_id
        ).0;
        
        change_clock_sysvar(&ptc, 550);

        let fund_amount = 100_u64;
        let winners_count = 1_u8;
        let starting_time = 1000_i64;
        let ending_time = 1350_i64;
        let minimum_tickets_amount_required_to_be_sold = 1u32;
        let ticket_price = 1_000000u64;
        let maximum_number_of_tickets_per_user = None;
        
        let instruction = instruction_create_and_initialize_lottery_account(
            lottery_account,
            lottery_authority_account.pubkey(),
            funding_account.pubkey(),
            config_account.usdc_mint_account,
            get_associated_token_address(
                &lottery_account,
                &config_account.usdc_mint_account
            ),
            funding_usdc_token_account_pubkey,
            arbitrary_mint_account_addr,
            get_associated_token_address(
                &lottery_account,
                &arbitrary_mint_account_addr
            ),
            funding_arbitrary_token_account_pubkey,
            TOKEN_STANDARD_PROGRAM_ID,
            spl_associated_token_account::ID,
            SYSTEM_PROGRAM_ID,
            config_account_pda.0,
            fund_amount,
            winners_count,
            starting_time,
            ending_time,
            minimum_tickets_amount_required_to_be_sold,
            ticket_price,
            maximum_number_of_tickets_per_user,
            lottery_description
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &funding_account,
                &lottery_authority_account
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidMinimumAmountTickets as u32
                )
            )
        );
    }
    // failure - invalid min tickets required to be sold 

    // failure - invalid starting and ending time
    {
        // 1. starting and ending time mismatched
        let lottery_description = String::from("AABBCC");
        let lottery_account = Pubkey::find_program_address(
            &[
                b"lottery_account",
                lottery_authority_account.pubkey().to_bytes().as_slice(),
                get_lottery_literal_seed(&lottery_description).as_slice()
            ],
            &lottery_program_id
        ).0;
        
        change_clock_sysvar(&ptc, 550);

        let fund_amount = 100_u64;
        let winners_count = 1_u8;
        let starting_time = 1500_i64;
        let ending_time = 1350_i64;
        let minimum_tickets_amount_required_to_be_sold = 25u32;
        let ticket_price = 1_000000u64;
        let maximum_number_of_tickets_per_user = None;
        
        let instruction = instruction_create_and_initialize_lottery_account(
            lottery_account,
            lottery_authority_account.pubkey(),
            funding_account.pubkey(),
            config_account.usdc_mint_account,
            get_associated_token_address(
                &lottery_account,
                &config_account.usdc_mint_account
            ),
            funding_usdc_token_account_pubkey,
            arbitrary_mint_account_addr,
            get_associated_token_address(
                &lottery_account,
                &arbitrary_mint_account_addr
            ),
            funding_arbitrary_token_account_pubkey,
            TOKEN_STANDARD_PROGRAM_ID,
            spl_associated_token_account::ID,
            SYSTEM_PROGRAM_ID,
            config_account_pda.0,
            fund_amount,
            winners_count,
            starting_time,
            ending_time,
            minimum_tickets_amount_required_to_be_sold,
            ticket_price,
            maximum_number_of_tickets_per_user,
            lottery_description
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &funding_account,
                &lottery_authority_account
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidTime as u32
                )
            )
        );

        // 2. starting and current_time does not match
        let lottery_description = String::from("AABBCC");
        let lottery_account = Pubkey::find_program_address(
            &[
                b"lottery_account",
                lottery_authority_account.pubkey().to_bytes().as_slice(),
                get_lottery_literal_seed(&lottery_description).as_slice()
            ],
            &lottery_program_id
        ).0;
        
        change_clock_sysvar(&ptc, 550);

        let fund_amount = 100_u64;
        let winners_count = 1_u8;
        let starting_time = 450_i64;
        let ending_time = 800_i64;
        let minimum_tickets_amount_required_to_be_sold = 25u32;
        let ticket_price = 1_000000u64;
        let maximum_number_of_tickets_per_user = None;
        
        let instruction = instruction_create_and_initialize_lottery_account(
            lottery_account,
            lottery_authority_account.pubkey(),
            funding_account.pubkey(),
            config_account.usdc_mint_account,
            get_associated_token_address(
                &lottery_account,
                &config_account.usdc_mint_account
            ),
            funding_usdc_token_account_pubkey,
            arbitrary_mint_account_addr,
            get_associated_token_address(
                &lottery_account,
                &arbitrary_mint_account_addr
            ),
            funding_arbitrary_token_account_pubkey,
            TOKEN_STANDARD_PROGRAM_ID,
            spl_associated_token_account::ID,
            SYSTEM_PROGRAM_ID,
            config_account_pda.0,
            fund_amount,
            winners_count,
            starting_time,
            ending_time,
            minimum_tickets_amount_required_to_be_sold,
            ticket_price,
            maximum_number_of_tickets_per_user,
            lottery_description
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &funding_account,
                &lottery_authority_account
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidTime as u32
                )
            )
        );
    }
    // failure - invalid starting and ending time

    // failure - invalid fund amount
    {
        // 1. invalid fund_amount
        let lottery_description = String::from("AABBCC");
        let lottery_account = Pubkey::find_program_address(
            &[
                b"lottery_account",
                lottery_authority_account.pubkey().to_bytes().as_slice(),
                get_lottery_literal_seed(&lottery_description).as_slice()
            ],
            &lottery_program_id
        ).0;
        
        change_clock_sysvar(&ptc, 150);

        let fund_amount = 0_u64;
        let winners_count = 1_u8;
        let starting_time = 250_i64;
        let ending_time = 500_i64;
        let minimum_tickets_amount_required_to_be_sold = 25u32;
        let ticket_price = 1_000000u64;
        let maximum_number_of_tickets_per_user = None;
        
        let instruction = instruction_create_and_initialize_lottery_account(
            lottery_account,
            lottery_authority_account.pubkey(),
            funding_account.pubkey(),
            config_account.usdc_mint_account,
            get_associated_token_address(
                &lottery_account,
                &config_account.usdc_mint_account
            ),
            funding_usdc_token_account_pubkey,
            arbitrary_mint_account_addr,
            get_associated_token_address(
                &lottery_account,
                &arbitrary_mint_account_addr
            ),
            funding_arbitrary_token_account_pubkey,
            TOKEN_STANDARD_PROGRAM_ID,
            spl_associated_token_account::ID,
            SYSTEM_PROGRAM_ID,
            config_account_pda.0,
            fund_amount,
            winners_count,
            starting_time,
            ending_time,
            minimum_tickets_amount_required_to_be_sold,
            ticket_price,
            maximum_number_of_tickets_per_user,
            lottery_description
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &funding_account,
                &lottery_authority_account
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidFundAmount as u32
                )
            )
        );

        // 2. fund_amount & winners_count mismatched
        let lottery_description = String::from("AABBCC");
        let lottery_account = Pubkey::find_program_address(
            &[
                b"lottery_account",
                lottery_authority_account.pubkey().to_bytes().as_slice(),
                get_lottery_literal_seed(&lottery_description).as_slice()
            ],
            &lottery_program_id
        ).0;
        
        change_clock_sysvar(&ptc, 150);

        let fund_amount = 100_u64;
        let winners_count = 3_u8;
        let starting_time = 250_i64;
        let ending_time = 500_i64;
        let minimum_tickets_amount_required_to_be_sold = 25u32;
        let ticket_price = 1_000000u64;
        let maximum_number_of_tickets_per_user = None;
        
        let instruction = instruction_create_and_initialize_lottery_account(
            lottery_account,
            lottery_authority_account.pubkey(),
            funding_account.pubkey(),
            config_account.usdc_mint_account,
            get_associated_token_address(
                &lottery_account,
                &config_account.usdc_mint_account
            ),
            funding_usdc_token_account_pubkey,
            arbitrary_mint_account_addr,
            get_associated_token_address(
                &lottery_account,
                &arbitrary_mint_account_addr
            ),
            funding_arbitrary_token_account_pubkey,
            TOKEN_STANDARD_PROGRAM_ID,
            spl_associated_token_account::ID,
            SYSTEM_PROGRAM_ID,
            config_account_pda.0,
            fund_amount,
            winners_count,
            starting_time,
            ending_time,
            minimum_tickets_amount_required_to_be_sold,
            ticket_price,
            maximum_number_of_tickets_per_user,
            lottery_description
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &funding_account,
                &lottery_authority_account
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::WinnersAndFundAmountMismatch as u32
                )
            )
        );
    }
    // failure - invalid fund amount

    // failure - invalid lottery's ata for usdc and arbitrary token accounts
    {
        // 1. invalid usdc ata
        let lottery_description = String::from("AABBCC");
        let lottery_account = Pubkey::find_program_address(
            &[
                b"lottery_account",
                lottery_authority_account.pubkey().to_bytes().as_slice(),
                get_lottery_literal_seed(&lottery_description).as_slice()
            ],
            &lottery_program_id
        ).0;
        
        change_clock_sysvar(&ptc, 150);

        let fund_amount = 100_u64;
        let winners_count = 1_u8;
        let starting_time = 250_i64;
        let ending_time = 500_i64;
        let minimum_tickets_amount_required_to_be_sold = 25u32;
        let ticket_price = 1_000000u64;
        let maximum_number_of_tickets_per_user = None;
        
        let instruction = instruction_create_and_initialize_lottery_account(
            lottery_account,
            lottery_authority_account.pubkey(),
            funding_account.pubkey(),
            config_account.usdc_mint_account,
            Pubkey::new_unique(),
            funding_usdc_token_account_pubkey,
            arbitrary_mint_account_addr,
            get_associated_token_address(
                &lottery_account,
                &arbitrary_mint_account_addr
            ),
            funding_arbitrary_token_account_pubkey,
            TOKEN_STANDARD_PROGRAM_ID,
            spl_associated_token_account::ID,
            SYSTEM_PROGRAM_ID,
            config_account_pda.0,
            fund_amount,
            winners_count,
            starting_time,
            ending_time,
            minimum_tickets_amount_required_to_be_sold,
            ticket_price,
            maximum_number_of_tickets_per_user,
            lottery_description
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &funding_account,
                &lottery_authority_account
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidLotteryAssociatedUsdcTokenAccount as u32
                )
            )
        );
        
        // 2. invalid arbitrary ata
        let lottery_description = String::from("AABBCC");
        let lottery_account = Pubkey::find_program_address(
            &[
                b"lottery_account",
                lottery_authority_account.pubkey().to_bytes().as_slice(),
                get_lottery_literal_seed(&lottery_description).as_slice()
            ],
            &lottery_program_id
        ).0;
        
        change_clock_sysvar(&ptc, 150);

        let fund_amount = 100_u64;
        let winners_count = 1_u8;
        let starting_time = 250_i64;
        let ending_time = 500_i64;
        let minimum_tickets_amount_required_to_be_sold = 25u32;
        let ticket_price = 1_000000u64;
        let maximum_number_of_tickets_per_user = None;
        
        let instruction = instruction_create_and_initialize_lottery_account(
            lottery_account,
            lottery_authority_account.pubkey(),
            funding_account.pubkey(),
            config_account.usdc_mint_account,
            get_associated_token_address(
                &lottery_account,
                &config_account.usdc_mint_account
            ),
            funding_usdc_token_account_pubkey,
            arbitrary_mint_account_addr,
            Pubkey::new_unique(),
            funding_arbitrary_token_account_pubkey,
            TOKEN_STANDARD_PROGRAM_ID,
            spl_associated_token_account::ID,
            SYSTEM_PROGRAM_ID,
            config_account_pda.0,
            fund_amount,
            winners_count,
            starting_time,
            ending_time,
            minimum_tickets_amount_required_to_be_sold,
            ticket_price,
            maximum_number_of_tickets_per_user,
            lottery_description
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &funding_account,
                &lottery_authority_account
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidLotteryArbitraryAssociatedTokenAccount as u32
                )
            )
        );
    }
    // failure - invalid lottery's ata for usdc and arbitrary token accounts

    // faliure - invalid usdc mint account
    {
        let lottery_description = String::from("AABBCC");
        let lottery_account = Pubkey::find_program_address(
            &[
                b"lottery_account",
                lottery_authority_account.pubkey().to_bytes().as_slice(),
                get_lottery_literal_seed(&lottery_description).as_slice()
            ],
            &lottery_program_id
        ).0;
        
        change_clock_sysvar(&ptc, 150);

        let fund_amount = 100_u64;
        let winners_count = 1_u8;
        let starting_time = 250_i64;
        let ending_time = 500_i64;
        let minimum_tickets_amount_required_to_be_sold = 25u32;
        let ticket_price = 1_000000u64;
        let maximum_number_of_tickets_per_user = None;
        
        let instruction = instruction_create_and_initialize_lottery_account(
            lottery_account,
            lottery_authority_account.pubkey(),
            funding_account.pubkey(),
            Pubkey::new_unique(),
            get_associated_token_address(
                &lottery_account,
                &config_account.usdc_mint_account
            ),
            funding_usdc_token_account_pubkey,
            arbitrary_mint_account_addr,
            get_associated_token_address(
                &lottery_account,
                &arbitrary_mint_account_addr
            ),
            funding_arbitrary_token_account_pubkey,
            TOKEN_STANDARD_PROGRAM_ID,
            spl_associated_token_account::ID,
            SYSTEM_PROGRAM_ID,
            config_account_pda.0,
            fund_amount,
            winners_count,
            starting_time,
            ending_time,
            minimum_tickets_amount_required_to_be_sold,
            ticket_price,
            maximum_number_of_tickets_per_user,
            lottery_description
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &funding_account,
                &lottery_authority_account
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidUsdcMintAccount as u32
                )
            )
        );
    }
    // faliure - invalid usdc mint account
    
    // success
    {
        let lottery_description = String::from("AABBCC");
        let lottery_account = Pubkey::find_program_address(
            &[
                b"lottery_account",
                lottery_authority_account.pubkey().to_bytes().as_slice(),
                get_lottery_literal_seed(&lottery_description).as_slice()
            ],
            &lottery_program_id
        ).0;
        
        change_clock_sysvar(&ptc, 550);

        let fund_amount = 100_u64;
        let winners_count = 1_u8;
        let starting_time = 1000_i64;
        let ending_time = 1350_i64;
        let minimum_tickets_amount_required_to_be_sold = 25u32;
        let ticket_price = 1_000000u64;
        let maximum_number_of_tickets_per_user = None;
        
        let instruction = instruction_create_and_initialize_lottery_account(
            lottery_account,
            lottery_authority_account.pubkey(),
            funding_account.pubkey(),
            config_account.usdc_mint_account,
            get_associated_token_address(
                &lottery_account,
                &config_account.usdc_mint_account
            ),
            funding_usdc_token_account_pubkey,
            arbitrary_mint_account_addr,
            get_associated_token_address(
                &lottery_account,
                &arbitrary_mint_account_addr
            ),
            funding_arbitrary_token_account_pubkey,
            TOKEN_STANDARD_PROGRAM_ID,
            spl_associated_token_account::ID,
            SYSTEM_PROGRAM_ID,
            config_account_pda.0,
            fund_amount,
            winners_count,
            starting_time,
            ending_time,
            minimum_tickets_amount_required_to_be_sold,
            ticket_price,
            maximum_number_of_tickets_per_user,
            lottery_description
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &funding_account,
                &lottery_authority_account
            ],
            ptc.last_blockhash
        );

        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        let SolanaAccount { data: lottery_account_data, owner: lottery_account_owner, .. } = ptc
            .banks_client
            .get_account(lottery_account)
            .await
            .unwrap()
            .unwrap();

        let lottery = Lottery::deserialize(
            &mut &lottery_account_data[..]
        ).unwrap();

        assert_eq!(
            lottery_account_owner,
            lottery_program_id,
            "Invalid lottery-account's owner"
        );

        assert_eq!(
            starting_time,
            lottery.starting_time,
            "Invalid starting-time."
        );

        assert_eq!(
            ending_time,
            lottery.ending_time,
            "Invalid ending-time."
        );

        assert_eq!(
            fund_amount,
            lottery.fund_amount,
            "Invalid fund-amount."
        );

        assert_eq!(
            winners_count,
            lottery.winners_count,
            "Invalid winners-count."
        );

        assert_eq!(
            maximum_number_of_tickets_per_user,
            lottery.maximum_number_of_tickets_per_user,
            "Invalid max number of tickets per user."
        );

        assert_eq!(
            ticket_price,
            lottery.ticket_price,
            "Invalid ticket price."
        );

        assert_eq!(
            minimum_tickets_amount_required_to_be_sold,
            lottery.minimum_tickets_amount_required_to_be_sold,
            "Invalid min tickets amount required to be sold."
        );

        assert_eq!(
            Lottery::get_discriminator(),
            lottery.discriminator,
            "invalid lottery account discriminator."
        );

        assert_eq!(
            lottery.is_creator_withdrawed_when_lottery_was_failed,
            bool::default(),
            "invalid is_creator_withdrawed_when_lottery_was_failed flag."
        );

        assert_eq!(
            lottery.is_creator_withdrawed_when_lottery_was_successful,
            bool::default(),
            "invalid is_creator_withdrawed_when_lottery_was_successful."
        );

        assert_eq!(
            lottery.is_ended_successfuly,
            bool::default(),
            "invalid is_ended_successfuly flag."
        );

        assert_eq!(
            lottery.is_protocol_fee_claimed,
            bool::default(),
            "invalid is_protocol_fee_claimed flag."
        );

        assert_eq!(
            lottery.protocol_fee,
            u64::default(),
            "invalid protocol fee in lottery_account."
        );

        let lottery_usdc_associated_token_account = ptc.banks_client.get_account(
            get_associated_token_address(
                &lottery_account,
                &config_account.usdc_mint_account
            )
        ).await.unwrap().unwrap();

        assert_eq!(
            lottery_usdc_associated_token_account.owner,
            TOKEN_STANDARD_PROGRAM_ID,
            "invalid owner for usdc token account."
        );

        let usdc_token_account = TokenAccount::unpack(
            &mut &lottery_usdc_associated_token_account.data
        ).unwrap();

        assert_eq!(
            usdc_token_account.state,
            TokenAccountState::Initialized,
            "invalid usdc-token-account's state."
        );

        assert_eq!(
            usdc_token_account.mint,
            config_account.usdc_mint_account,
            "invalid mint account for usdc-token-account."
        );

        assert_eq!(
            usdc_token_account.amount,
            config_account.lottery_creation_fee,
            "invalid usdc token balance."
        );

        assert_eq!(
            usdc_token_account.owner,
            lottery_account,
            "invalid owner for usdc token account."
        );

        let lottery_arbitrary_associated_token_account = ptc.banks_client.get_account(
            get_associated_token_address(
                &lottery_account,
                &arbitrary_mint_account_addr
            )
        ).await.unwrap().unwrap();

        assert_eq!(
            lottery_arbitrary_associated_token_account.owner,
            TOKEN_STANDARD_PROGRAM_ID,
            "invalid owner for arbitrary token account."
        );

        let arbitrary_token_account = TokenAccount::unpack(
            &mut &lottery_arbitrary_associated_token_account.data
        ).unwrap();

        assert_eq!(
            arbitrary_token_account.state,
            TokenAccountState::Initialized,
            "invalid arbitrary-token-account's state."
        );

        assert_eq!(
            arbitrary_token_account.mint,
            arbitrary_mint_account_addr,
            "invalid mint account for arbitrary-token-account."
        );

        assert_eq!(
            arbitrary_token_account.amount,
            fund_amount,
            "invalid arbitrary token balance."
        );

        assert_eq!(
            arbitrary_token_account.owner,
            lottery_account,
            "invalid owner for arbitrary token account."
        );
    }
    // success
}

#[tokio::test]
async fn test_change_lottery_ticket_price() {
    let lottery_program_id = LOTTERY_PROGRAM_ID;
    let mut pt = setup_program_test(lottery_program_id);

    //////////////////////? add config account
    let config_account_pda = Pubkey::find_program_address(
        &[
            b"solottery_program_config_account"
        ],
        &lottery_program_id
    );

    let config_account = Config {
        discriminator: Config::get_discriminator(),
        canonical_bump: config_account_pda.1,
        lottery_creation_fee: 5_000000, // 5 USDC
        maximum_number_of_winners: 10,
        usdc_mint_account: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
        maximum_time_for_lottery_account: 1000,
        minimum_tickets_to_be_sold_in_lottery: 20,
        max_lottery_description_bytes: 300,
        ..Config::default()
    };
    let config_account_data = config_account.try_to_vec().unwrap();

    let config_solana_account = SolanaAccount {
        owner: lottery_program_id,
        data: config_account_data,
        lamports: sol_to_lamports(0.0009),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account_pda.0,
        config_solana_account
    );
    //////////////////////? add config account
    
    //////////////////////? add lottery account
    let lottery_account_pda = Pubkey::find_program_address(
        &[
            b"lottery_account",
            Pubkey::default().to_bytes().as_slice(),
            get_lottery_literal_seed(&String::from("1")).as_slice()
        ],
        &lottery_program_id
    );

    let lottery_authority_account = Keypair::new();
    let lottery_account = Lottery {
        discriminator: Lottery::get_discriminator(),
        canonical_bump: lottery_account_pda.1,
        lottery_creation_fee: 5_000000,
        protocol_fee: 300_000000,
        starting_time: 100,
        ending_time: 200,
        ticket_price: 5_000000, // 5 USDC
        authority: lottery_authority_account.pubkey(),
        lottery_description: String::from("1"),
        ..Lottery::default()
    };

    let lottey_solana_account = SolanaAccount {
        owner: lottery_program_id,
        lamports: solana_sdk::native_token::sol_to_lamports(1.0),
        data: lottery_account.try_to_vec().unwrap(),
        ..SolanaAccount::default()
    };

    pt.add_account(
        lottery_account_pda.0,
        lottey_solana_account
    );
    //////////////////////? add lottery account
    
    let mut ptc = pt.start_with_context().await;

    // failure - invalid lottery state
    {
        // lottery is ended
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(
            &ptc,
            300
        );

        let new_ticket_price = 10_000000u64;
        let instruction = instruction_change_lottery_ticket_price(
            lottery_account_pda.0, 
            lottery_authority_account.pubkey(), 
            config_account_pda.0, 
            new_ticket_price // USDC
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ], 
            Some(&ptc.payer.pubkey()), 
            &[
                &ptc.payer,
                &lottery_authority_account
            ], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidLotteryState as u32
                )
            )
        );
    }
    // failure - invalid lottery state

    // failure - invalid lottery's new ticket-price
    {
        // lottery is started
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(
            &ptc,
            120
        );

        let new_ticket_price = 5_000000u64;
        let instruction = instruction_change_lottery_ticket_price(
            lottery_account_pda.0,
            lottery_authority_account.pubkey(),
            config_account_pda.0,
            new_ticket_price
        );

        let tx = Transaction::new_signed_with_payer(
            &[
                instruction
            ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &lottery_authority_account
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidTicketPrice as u32
                )
            )
        );
    }
    // failure - invalid lottery's new ticket-price

    // success
    {
        // lottery is started
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(
            &ptc,
            120
        );

        let new_ticket_price = 10_000000u64;
        let instruction = instruction_change_lottery_ticket_price(
            lottery_account_pda.0, 
            lottery_authority_account.pubkey(), 
            config_account_pda.0, 
            new_ticket_price // USDC
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ], 
            Some(&ptc.payer.pubkey()), 
            &[
                &ptc.payer,
                &lottery_authority_account
            ], 
            ptc.last_blockhash
        );

        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        let SolanaAccount { data, .. } = ptc
            .banks_client
            .get_account(lottery_account_pda.0)
            .await
            .unwrap()
            .unwrap();

        let Lottery { ticket_price, .. } = Lottery::deserialize(
            &mut &data[..]
        ).unwrap();

        assert_eq!(
            ticket_price,
            new_ticket_price,
            "1. invalid new lottery's ticket price."
        );

        // 2. lottery is not started yet
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(
            &ptc,
            65
        );

        let new_ticket_price = 15_000000u64;
        let instruction = instruction_change_lottery_ticket_price(
            lottery_account_pda.0, 
            lottery_authority_account.pubkey(), 
            config_account_pda.0, 
            new_ticket_price // USDC
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ], 
            Some(&ptc.payer.pubkey()), 
            &[
                &ptc.payer,
                &lottery_authority_account
            ], 
            ptc.last_blockhash
        );

        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        let SolanaAccount { data, .. } = ptc
            .banks_client
            .get_account(lottery_account_pda.0)
            .await
            .unwrap()
            .unwrap();

        let Lottery { ticket_price, .. } = Lottery::deserialize(
            &mut &data[..]
        ).unwrap();

        assert_eq!(
            ticket_price,
            new_ticket_price,
            "2. invalid new lottery's ticket price."
        );
    }
    // success
}

#[tokio::test]
async fn test_end_lottery_and_pick_winners() {
    let lottery_program_id = LOTTERY_PROGRAM_ID;
    let mut pt = setup_program_test(lottery_program_id);

    //////////////////////? add config account
    let config_account_pda = Pubkey::find_program_address(
        &[
            b"solottery_program_config_account"
        ],
        &lottery_program_id
    );

    let config_account = Config {
        discriminator: Config::get_discriminator(),
        canonical_bump: config_account_pda.1,
        pyth_price_feed_accounts: [
            Pubkey::from_str("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE").unwrap(), // SOl
            Pubkey::from_str("4cSM2e6rvbGQUFiJbqytoVMi5GgghSMr8LwVrT9VPSPo").unwrap(), // BTC
            Pubkey::from_str("42amVS4KgzR9rA28tkVYqVXjq9Qa8dcZQMbH5EYFX6XC").unwrap() // ETH
        ],
        pyth_price_feed_ids: [
            "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d".to_string(), // SOL
            "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43".to_string(), // BTC
            "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace".to_string() // ETH
        ],
        maximum_time_of_price_feed_age: 10,
        ..Config::default()
    };
    let config_account_data = config_account.try_to_vec().unwrap();

    let config_solana_account = SolanaAccount {
        owner: lottery_program_id,
        data: config_account_data,
        lamports: sol_to_lamports(0.0009),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account_pda.0,
        config_solana_account
    );
    //////////////////////? add config account
    
    //////////////////////? add lottery account
    let lottery_account_pda = Pubkey::find_program_address(
        &[
            b"lottery_account",
            Pubkey::default().to_bytes().as_slice(),
            get_lottery_literal_seed(&String::from("1")).as_slice()
        ],
        &lottery_program_id
    );

    let mut lottery_account = Lottery {
        discriminator: Lottery::get_discriminator(),
        canonical_bump: lottery_account_pda.1,
        starting_time: 100,
        ending_time: 200,
        lottery_description: String::from("1"),
        winners_count: 5,
        minimum_tickets_amount_required_to_be_sold: 5,
        tickets_total_amount: 10,
        ..Lottery::default()
    };
    lottery_account.initial_bytes = lottery_account.try_to_vec().unwrap().len() as u64 + (lottery_account.winners_count as u64 * 33);

    let lottey_solana_account = SolanaAccount {
        owner: lottery_program_id,
        lamports: solana_sdk::native_token::sol_to_lamports(1.0),
        data: vec![
            lottery_account.try_to_vec().unwrap(),
            vec![0u8; lottery_account.winners_count as usize * 33],
            Pubkey::new_from_array([10; 32]).to_bytes().to_vec(),
            Pubkey::new_from_array([5; 32]).to_bytes().to_vec(),
            Pubkey::new_from_array([8; 32]).to_bytes().to_vec(),
            Pubkey::new_from_array([2; 32]).to_bytes().to_vec(),
            Pubkey::new_from_array([1; 32]).to_bytes().to_vec(),
            Pubkey::new_from_array([3; 32]).to_bytes().to_vec(),
            Pubkey::new_from_array([4; 32]).to_bytes().to_vec(),
            Pubkey::new_from_array([9; 32]).to_bytes().to_vec(),
            Pubkey::new_from_array([7; 32]).to_bytes().to_vec(),
            Pubkey::new_from_array([6; 32]).to_bytes().to_vec()
        ].concat(),
        ..SolanaAccount::default()
    };

    pt.add_account(
        lottery_account_pda.0,
        lottey_solana_account
    );
    //////////////////////? add lottery account
    
    //////////////////////? add SOL pyth price feed account
    let sol_price_feed_account_pubkey = config_account.pyth_price_feed_accounts[0];
    let sol_price_feed_account = PriceUpdateV2 {
        verification_level: VerificationLevel::Full,
        price_message: PriceFeedMessage {
            publish_time: 300,
            price: 250,
            conf: u64::default(),
            ema_conf: u64::default(),
            ema_price: i64::default(),
            exponent: i32::default(),
            feed_id: get_feed_id_from_hex(
                &config_account.pyth_price_feed_ids[0]
            ).unwrap(),
            prev_publish_time: i64::default()
        },
        write_authority: Pubkey::default(),
        posted_slot: u64::default()
    };

    let sol_price_feed_solana_account = SolanaAccount {
        lamports: sol_to_lamports(1.0),
        owner: PYTH_PRICE_RECEIVER_PROGRAM_ID,
        data: vec![
            vec![ 34, 241, 35, 99, 157, 126, 244, 205 ],
            sol_price_feed_account.try_to_vec().unwrap()
        ].concat(),
        ..SolanaAccount::default()
    };

    pt.add_account(
        sol_price_feed_account_pubkey,
        sol_price_feed_solana_account
    );
    //////////////////////? add SOL pyth price feed account
    
    //////////////////////? add BTC pyth price feed account
    let btc_price_feed_account_pubkey = config_account.pyth_price_feed_accounts[1];
    let btc_price_feed_account = PriceUpdateV2 {
        verification_level: VerificationLevel::Full,
        price_message: PriceFeedMessage {
            publish_time: 300,
            price: 150,
            conf: u64::default(),
            ema_conf: u64::default(),
            ema_price: i64::default(),
            exponent: i32::default(),
            feed_id: get_feed_id_from_hex(
                &config_account.pyth_price_feed_ids[1]
            ).unwrap(),
            prev_publish_time: i64::default()
        },
        write_authority: Pubkey::default(),
        posted_slot: u64::default()
    };

    let btc_price_feed_solana_account = SolanaAccount {
        lamports: sol_to_lamports(1.0),
        owner: PYTH_PRICE_RECEIVER_PROGRAM_ID,
        data: vec![
            vec![ 34, 241, 35, 99, 157, 126, 244, 205 ],
            btc_price_feed_account.try_to_vec().unwrap()
        ].concat(),
        ..SolanaAccount::default()
    };

    pt.add_account(
        btc_price_feed_account_pubkey,
        btc_price_feed_solana_account
    );
    //////////////////////? add BTC pyth price feed account
    
    //////////////////////? add ETH pyth price feed account
    let eth_price_feed_account_pubkey = config_account.pyth_price_feed_accounts[2];
    let eth_price_feed_account = PriceUpdateV2 {
        verification_level: VerificationLevel::Full,
        price_message: PriceFeedMessage {
            publish_time: 500,
            price: 250,
            conf: u64::default(),
            ema_conf: u64::default(),
            ema_price: i64::default(),
            exponent: i32::default(),
            feed_id: get_feed_id_from_hex(
                &config_account.pyth_price_feed_ids[2]
            ).unwrap(),
            prev_publish_time: i64::default()
        },
        write_authority: Pubkey::default(),
        posted_slot: u64::default()
    };

    let eth_price_feed_solana_account = SolanaAccount {
        lamports: sol_to_lamports(1.0),
        owner: PYTH_PRICE_RECEIVER_PROGRAM_ID,
        data: vec![
            vec![ 34, 241, 35, 99, 157, 126, 244, 205 ],
            eth_price_feed_account.try_to_vec().unwrap()
        ].concat(),
        ..SolanaAccount::default()
    };

    pt.add_account(
        eth_price_feed_account_pubkey,
        eth_price_feed_solana_account
    );
    //////////////////////? add ETH pyth price feed account
    
    let mut ptc = pt.start_with_context().await;

    // success
    {
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 250);

        let instruction = instruction_end_lottery_and_pick_winners(
            lottery_account_pda.0, 
            config_account_pda.0, 
            sol_price_feed_account_pubkey, 
            btc_price_feed_account_pubkey, 
            eth_price_feed_account_pubkey
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ], 
            Some(&ptc.payer.pubkey()), 
            &[&ptc.payer], 
            ptc.last_blockhash
        );

        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        let SolanaAccount { data, .. } = ptc
            .banks_client
            .get_account(lottery_account_pda.0)
            .await
            .unwrap()
            .unwrap();

        let Lottery { is_ended_successfuly, winners, random_numbers_info, .. } = Lottery::deserialize(
            &mut &data[..]
        ).unwrap();

        assert_eq!(
            is_ended_successfuly,
            true,
            "invalid is_ended_suc.. flag."
        );

        let winners_index: [u8; 5] = [ 3, 6, 4, 2, 8 ];
        for index in 0..5 {
            assert_eq!(
                winners[index].0,
                Pubkey::new_from_array([winners_index[index]; 32]),
                "invalid winner pubkey -> {} - {}", index, winners_index[index]
            );

            assert_eq!(
                winners[index].1,
                false,
                "invalid winner flag -> {}", index
            );
        };

        let (
            price_feed_account,
            price_publish_time,
            price 
        ) = random_numbers_info;

        assert_eq!(
            price_feed_account,
            btc_price_feed_account_pubkey,
            "invalid price feed account pubkey."
        );

        assert_eq!(
            price_publish_time,
            btc_price_feed_account.price_message.publish_time,
            "invalid price feed publish time."
        );

        assert_eq!(
            price,
            btc_price_feed_account.price_message.price,
            "invalid price."
        );
    }   
    // success

    // failure - lottery already ended
    {
        let lottery_account_pda = Pubkey::find_program_address(
            &[
                b"lottery_account",
                Pubkey::default().to_bytes().as_slice(),
                get_lottery_literal_seed(&String::from("1")).as_slice()
            ],
            &lottery_program_id
        );
    
        let lottery_account = Lottery {
            discriminator: Lottery::get_discriminator(),
            canonical_bump: lottery_account_pda.1,
            starting_time: 100,
            ending_time: 200,
            lottery_description: String::from("1"),
            winners_count: 5,
            minimum_tickets_amount_required_to_be_sold: 5,
            tickets_total_amount: 10,
            is_ended_successfuly: true,
            ..Lottery::default()
        };
    
        let lottey_solana_account = SolanaAccount {
            owner: lottery_program_id,
            lamports: solana_sdk::native_token::sol_to_lamports(1.0),
            data: lottery_account.try_to_vec().unwrap(),
            ..SolanaAccount::default()
        };
    
        ptc.set_account(
            &lottery_account_pda.0,
            &SolanaSharedDataAccount::from(lottey_solana_account)
        );

        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 250);

        let instruction = instruction_end_lottery_and_pick_winners(
            lottery_account_pda.0, 
            config_account_pda.0, 
            sol_price_feed_account_pubkey, 
            btc_price_feed_account_pubkey, 
            eth_price_feed_account_pubkey
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ], 
            Some(&ptc.payer.pubkey()), 
            &[&ptc.payer], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::LotteryAlreadyEnded as u32
                )
            )
        );
    }
    // failure - lottery already ended

    // failure - lottery was failed
    {
        // 1. LotteryState::Failed
        let lottery_account_pda = Pubkey::find_program_address(
            &[
                b"lottery_account",
                Pubkey::default().to_bytes().as_slice(),
                get_lottery_literal_seed(&String::from("1")).as_slice()
            ],
            &lottery_program_id
        );
    
        let lottery_account = Lottery {
            discriminator: Lottery::get_discriminator(),
            canonical_bump: lottery_account_pda.1,
            starting_time: 100,
            ending_time: 200,
            lottery_description: String::from("1"),
            winners_count: 5,
            minimum_tickets_amount_required_to_be_sold: 50,
            tickets_total_amount: 10,
            ..Lottery::default()
        };
    
        let lottey_solana_account = SolanaAccount {
            owner: lottery_program_id,
            lamports: solana_sdk::native_token::sol_to_lamports(1.0),
            data: lottery_account.try_to_vec().unwrap(),
            ..SolanaAccount::default()
        };
    
        ptc.set_account(
            &lottery_account_pda.0,
            &SolanaSharedDataAccount::from(lottey_solana_account)
        );

        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 250);

        let instruction = instruction_end_lottery_and_pick_winners(
            lottery_account_pda.0, 
            config_account_pda.0, 
            sol_price_feed_account_pubkey, 
            btc_price_feed_account_pubkey, 
            eth_price_feed_account_pubkey
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ], 
            Some(&ptc.payer.pubkey()), 
            &[&ptc.payer], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::LotteryWasNotSuccessfull as u32
                )
            )
        );

        // 2. LotteryState::Unknown
        let lottery_account_pda = Pubkey::find_program_address(
            &[
                b"lottery_account",
                Pubkey::default().to_bytes().as_slice(),
                get_lottery_literal_seed(&String::from("1")).as_slice()
            ],
            &lottery_program_id
        );
    
        let lottery_account = Lottery {
            discriminator: Lottery::get_discriminator(),
            canonical_bump: lottery_account_pda.1,
            starting_time: 100,
            ending_time: 200,
            lottery_description: String::from("1"),
            winners_count: 5,
            minimum_tickets_amount_required_to_be_sold: 50,
            tickets_total_amount: 100,
            ..Lottery::default()
        };
    
        let lottey_solana_account = SolanaAccount {
            owner: lottery_program_id,
            lamports: solana_sdk::native_token::sol_to_lamports(1.0),
            data: lottery_account.try_to_vec().unwrap(),
            ..SolanaAccount::default()
        };
    
        ptc.set_account(
            &lottery_account_pda.0,
            &SolanaSharedDataAccount::from(lottey_solana_account)
        );

        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 50);

        let instruction = instruction_end_lottery_and_pick_winners(
            lottery_account_pda.0, 
            config_account_pda.0, 
            sol_price_feed_account_pubkey, 
            btc_price_feed_account_pubkey, 
            eth_price_feed_account_pubkey
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ], 
            Some(&ptc.payer.pubkey()), 
            &[&ptc.payer], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::LotteryWasNotSuccessfull as u32
                )
            )
        );

        // 3. LotteryState::Unknown
        let lottery_account_pda = Pubkey::find_program_address(
            &[
                b"lottery_account",
                Pubkey::default().to_bytes().as_slice(),
                get_lottery_literal_seed(&String::from("1")).as_slice()
            ],
            &lottery_program_id
        );
    
        let lottery_account = Lottery {
            discriminator: Lottery::get_discriminator(),
            canonical_bump: lottery_account_pda.1,
            starting_time: 100,
            ending_time: 200,
            lottery_description: String::from("1"),
            winners_count: 5,
            minimum_tickets_amount_required_to_be_sold: 50,
            tickets_total_amount: 100,
            ..Lottery::default()
        };
    
        let lottey_solana_account = SolanaAccount {
            owner: lottery_program_id,
            lamports: solana_sdk::native_token::sol_to_lamports(1.0),
            data: lottery_account.try_to_vec().unwrap(),
            ..SolanaAccount::default()
        };
    
        ptc.set_account(
            &lottery_account_pda.0,
            &SolanaSharedDataAccount::from(lottey_solana_account)
        );

        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 150);

        let instruction = instruction_end_lottery_and_pick_winners(
            lottery_account_pda.0, 
            config_account_pda.0, 
            sol_price_feed_account_pubkey, 
            btc_price_feed_account_pubkey, 
            eth_price_feed_account_pubkey
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ], 
            Some(&ptc.payer.pubkey()), 
            &[&ptc.payer], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::LotteryWasNotSuccessfull as u32
                )
            )
        );
    }
    // failure - lottery was failed

    // failure - insufficient random numbers
    {
        let lottery_account_pda = Pubkey::find_program_address(
            &[
                b"lottery_account",
                Pubkey::default().to_bytes().as_slice(),
                get_lottery_literal_seed(&String::from("1")).as_slice()
            ],
            &lottery_program_id
        );
    
        let lottery_account = Lottery {
            discriminator: Lottery::get_discriminator(),
            canonical_bump: lottery_account_pda.1,
            starting_time: 100,
            ending_time: 200,
            lottery_description: String::from("1"),
            winners_count: 35,
            minimum_tickets_amount_required_to_be_sold: 5,
            tickets_total_amount: 10,
            ..Lottery::default()
        };
    
        let lottey_solana_account = SolanaAccount {
            owner: lottery_program_id,
            lamports: solana_sdk::native_token::sol_to_lamports(1.0),
            data: lottery_account.try_to_vec().unwrap(),
            ..SolanaAccount::default()
        };
    
        ptc.set_account(
            &lottery_account_pda.0,
            &SolanaSharedDataAccount::from(lottey_solana_account)
        );

        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 250);

        let instruction = instruction_end_lottery_and_pick_winners(
            lottery_account_pda.0, 
            config_account_pda.0, 
            sol_price_feed_account_pubkey, 
            btc_price_feed_account_pubkey, 
            eth_price_feed_account_pubkey
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ], 
            Some(&ptc.payer.pubkey()), 
            &[&ptc.payer], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InsufficientRandomNumbers as u32
                )
            )
        );
    }
    // failure - insufficient random numbers
}

#[tokio::test]
async fn test_withdraw_succeed_lottery() {
    let lottery_program_id = LOTTERY_PROGRAM_ID;
    let mut pt = setup_program_test(lottery_program_id);

    //////////////////////? add config account
    let config_account_pda = Pubkey::find_program_address(
        &[
            b"solottery_program_config_account"
        ],
        &lottery_program_id
    );

    let config_account = Config {
        discriminator: Config::get_discriminator(),
        canonical_bump: config_account_pda.1,
        lottery_creation_fee: 5_000000, // 5 USDC
        maximum_number_of_winners: 10,
        usdc_mint_account: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
        maximum_time_for_lottery_account: 1000,
        minimum_tickets_to_be_sold_in_lottery: 20,
        max_lottery_description_bytes: 10,
        ..Config::default()
    };
    let config_account_data = config_account.try_to_vec().unwrap();

    let config_solana_account = SolanaAccount {
        owner: lottery_program_id,
        data: config_account_data,
        lamports: sol_to_lamports(0.0009),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account_pda.0,
        config_solana_account
    );
    //////////////////////? add config account

    //////////////////////? add lottery_account authority
    let lottery_authority_account = Keypair::new();
    pt.add_account(
        lottery_authority_account.pubkey(),
        SolanaAccount {
            owner: SYSTEM_PROGRAM_ID,
            lamports: sol_to_lamports(1.0),
            ..SolanaAccount::default()
        }
    );
    //////////////////////? add lottery_account authority
    
    //////////////////////? add lottery account
    let lottery_account_pda = Pubkey::find_program_address(
        &[
            b"lottery_account",
            lottery_authority_account.pubkey().to_bytes().as_slice(),
            get_lottery_literal_seed(&String::from("1")).as_slice()
        ],
        &lottery_program_id
    );

    let lottery_account = Lottery {
        discriminator: Lottery::get_discriminator(),
        canonical_bump: lottery_account_pda.1,
        starting_time: 100,
        ending_time: 200,
        lottery_description: String::from("1"),
        winners_count: 5,
        minimum_tickets_amount_required_to_be_sold: 5,
        tickets_total_amount: 10,
        protocol_fee: 95_000000,
        lottery_creation_fee: 5_000000,
        authority: lottery_authority_account.pubkey(),
        is_ended_successfuly: true,
        ..Lottery::default()
    };

    let lottey_solana_account = SolanaAccount {
        owner: lottery_program_id,
        lamports: solana_sdk::native_token::sol_to_lamports(1.0),
        data: lottery_account.try_to_vec().unwrap(),
        ..SolanaAccount::default()
    };

    pt.add_account(
        lottery_account_pda.0,
        lottey_solana_account
    );
    //////////////////////? add lottery account

    //////////////////////? add USDC mint account
    let usdc_mint_account = MintAccount {
        supply: 1000_000000,
        decimals: 6,
        is_initialized: true,
        ..MintAccount::default()
    };

    let mut usdc_mint_account_data: [u8; MintAccount::LEN] = [0; MintAccount::LEN];
    MintAccount::pack(
        usdc_mint_account,
        usdc_mint_account_data.as_mut_slice()
    ).unwrap();

    let uscd_mint_solana_account = SolanaAccount {
        lamports: sol_to_lamports(0.01),
        owner: TOKEN_STANDARD_PROGRAM_ID,
        data: usdc_mint_account_data.to_vec(),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account.usdc_mint_account,
        uscd_mint_solana_account
    );
    //////////////////////? add USDC mint account
    
    //////////////////////? add fund receiver USDC token account
    let funding_usdc_token_account_pubkey = Pubkey::new_from_array([1; 32]);
    let funding_usdc_token_account = TokenAccount {
        mint: config_account.usdc_mint_account,
        amount: 0_000000,
        state: TokenAccountState::Initialized,
        ..TokenAccount::default()
    };

    let mut funding_usdc_token_account_data: [u8; TokenAccount::LEN] = [0; TokenAccount::LEN];
    TokenAccount::pack(
        funding_usdc_token_account,
        funding_usdc_token_account_data.as_mut_slice()
    ).unwrap();

    let funding_usdc_token_solana_account = SolanaAccount {
        owner: TOKEN_STANDARD_PROGRAM_ID,
        data: funding_usdc_token_account_data.to_vec(),
        lamports: sol_to_lamports(0.01),
        ..SolanaAccount::default()
    };

    pt.add_account(
        funding_usdc_token_account_pubkey,
        funding_usdc_token_solana_account
    );
    //////////////////////? add fund receiver USDC token account
    
    //////////////////////? add lottery's usdc ata
    let lottery_ata_pda = get_associated_token_address(
        &lottery_account_pda.0,
        &config_account.usdc_mint_account
    );
    let lottery_ata = TokenAccount {
        state: TokenAccountState::Initialized,
        mint: config_account.usdc_mint_account,
        amount: 1000_000000,
        owner: lottery_account_pda.0,
        ..TokenAccount::default()
    };

    let mut lottery_ata_data = [0u8; TokenAccount::LEN];
    TokenAccount::pack(
        lottery_ata,
        lottery_ata_data.as_mut_slice()
    ).unwrap();

    let lottery_ata_solana_account = SolanaAccount {
        owner: spl_token::ID,
        data: lottery_ata_data.to_vec(),
        lamports: sol_to_lamports(1.0),
        ..SolanaAccount::default()
    };

    pt.add_account(
        lottery_ata_pda,
        lottery_ata_solana_account
    );
    //////////////////////? add lottery's usdc ata
    
    let mut ptc = pt.start_with_context().await;

    // failure - invalid USDC mint account
    {
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 350);

        let instruction = instruction_withdraw_succeed_lottery(
            lottery_account_pda.0, 
            config_account_pda.0, 
            lottery_authority_account.pubkey(), 
            lottery_ata_pda, 
            funding_usdc_token_account_pubkey, 
            Pubkey::default(), 
            TOKEN_STANDARD_PROGRAM_ID
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ], 
            Some(&ptc.payer.pubkey()), 
            &[
                &ptc.payer,
                &lottery_authority_account
            ], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidUsdcMintAccount as u32
                )
            )
        );
    }
    // failure - invalid USDC mint account

    // failure - invalid lottery state
    {
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 35);

        let instruction = instruction_withdraw_succeed_lottery(
            lottery_account_pda.0, 
            config_account_pda.0, 
            lottery_authority_account.pubkey(), 
            lottery_ata_pda, 
            funding_usdc_token_account_pubkey, 
            config_account.usdc_mint_account, 
            TOKEN_STANDARD_PROGRAM_ID
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ], 
            Some(&ptc.payer.pubkey()), 
            &[
                &ptc.payer,
                &lottery_authority_account
            ], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::LotteryWasNotSuccessfull as u32
                )
            )
        );
    }
    // failure - invalid lottery state

    // failure - trying to withdraw funds twice
    {
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 350);

        let instruction = instruction_withdraw_succeed_lottery(
            lottery_account_pda.0, 
            config_account_pda.0, 
            lottery_authority_account.pubkey(), 
            lottery_ata_pda, 
            funding_usdc_token_account_pubkey, 
            config_account.usdc_mint_account, 
            TOKEN_STANDARD_PROGRAM_ID
        );

        let tx = Transaction::new_signed_with_payer(
            &[ 
                instruction.clone(),
                instruction
            ], 
            Some(&ptc.payer.pubkey()), 
            &[
                &ptc.payer,
                &lottery_authority_account
            ], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                1,
                InstructionError::Custom(
                    LotteryError::FundsAlreadyWithdrawed as u32
                )
            )
        );
    }
    // failure - trying to withdraw funds twice

    // failure - invalid lottery authority
    {
        // 1. lottery authority in not signed the transaction
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 350);

        let mut instruction = instruction_withdraw_succeed_lottery(
            lottery_account_pda.0, 
            config_account_pda.0, 
            lottery_authority_account.pubkey(), 
            lottery_ata_pda, 
            funding_usdc_token_account_pubkey, 
            config_account.usdc_mint_account, 
            TOKEN_STANDARD_PROGRAM_ID
        );
        instruction.accounts[2].is_signer = false;

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ], 
            Some(&ptc.payer.pubkey()), 
            &[ &ptc.payer ], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::MissingRequiredSignature
            )
        );

        // 2. lottery authority mismatched
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 350);

        let unkown_signer = Keypair::new();
        let instruction = instruction_withdraw_succeed_lottery(
            lottery_account_pda.0, 
            config_account_pda.0, 
            unkown_signer.pubkey(), 
            lottery_ata_pda, 
            funding_usdc_token_account_pubkey, 
            config_account.usdc_mint_account, 
            TOKEN_STANDARD_PROGRAM_ID
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ], 
            Some(&ptc.payer.pubkey()), 
            &[ 
                &ptc.payer,
                &unkown_signer
            ], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidLotteryAccountAuthority as u32
                )
            )
        );
    }
    // failure - invalid lottery authority

    // failure - invalid lottery USDC ata
    {
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 350);

        let instruction = instruction_withdraw_succeed_lottery(
            lottery_account_pda.0, 
            config_account_pda.0, 
            lottery_authority_account.pubkey(), 
            lottery_authority_account.pubkey(), 
            funding_usdc_token_account_pubkey, 
            config_account.usdc_mint_account, 
            TOKEN_STANDARD_PROGRAM_ID
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ], 
            Some(&ptc.payer.pubkey()), 
            &[ 
                &ptc.payer,
                &lottery_authority_account
            ], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidLotteryAssociatedUsdcTokenAccount as u32
                )
            )
        );
    }
    // failure - invalid lottery USDC ata
    
    // success
    {
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 350);

        let instruction = instruction_withdraw_succeed_lottery(
            lottery_account_pda.0, 
            config_account_pda.0, 
            lottery_authority_account.pubkey(), 
            lottery_ata_pda, 
            funding_usdc_token_account_pubkey, 
            config_account.usdc_mint_account, 
            TOKEN_STANDARD_PROGRAM_ID
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ], 
            Some(&ptc.payer.pubkey()), 
            &[
                &ptc.payer,
                &lottery_authority_account
            ], 
            ptc.last_blockhash
        );

        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        let SolanaAccount { data: lottery_account_data, .. } = ptc
            .banks_client
            .get_account(lottery_account_pda.0)
            .await
            .unwrap()
            .unwrap();

        let SolanaAccount { data: fund_receiver_usdc_token_account_data, .. } = ptc
            .banks_client
            .get_account(funding_usdc_token_account_pubkey)
            .await
            .unwrap()
            .unwrap();

        let SolanaAccount { data: lottery_usdc_ata_data, .. } = ptc
            .banks_client
            .get_account(lottery_ata_pda)
            .await
            .unwrap()
            .unwrap();

        let Lottery { is_creator_withdrawed_when_lottery_was_successful, .. } = Lottery::deserialize(
            &mut &lottery_account_data[..]
        ).unwrap();

        assert_eq!(
            is_creator_withdrawed_when_lottery_was_successful,
            true,
            "invalid lottery's flag."
        );

        let TokenAccount { amount, .. } = TokenAccount::unpack(
            &fund_receiver_usdc_token_account_data
        ).unwrap();

        assert_eq!(
            amount,
            900_000000,
            "invalid fund receiver token account balance."
        );

        let TokenAccount { amount, .. } = TokenAccount::unpack(
            &lottery_usdc_ata_data
        ).unwrap();

        assert_eq!(
            amount,
            100_000000,
            "invalid lottery ata balance."
        );
    }
    // success

    // failure - trying to withdraw funds before ending lottery and picking winners
    {
        let lottery_account_pda = Pubkey::find_program_address(
            &[
                b"lottery_account",
                lottery_authority_account.pubkey().to_bytes().as_slice(),
                get_lottery_literal_seed(&String::from("1")).as_slice()
            ],
            &lottery_program_id
        );
    
        let lottery_account = Lottery {
            discriminator: Lottery::get_discriminator(),
            canonical_bump: lottery_account_pda.1,
            starting_time: 100,
            ending_time: 200,
            lottery_description: String::from("1"),
            winners_count: 5,
            minimum_tickets_amount_required_to_be_sold: 5,
            tickets_total_amount: 10,
            protocol_fee: 95_000000,
            lottery_creation_fee: 5_000000,
            authority: lottery_authority_account.pubkey(),
            is_ended_successfuly: false,
            ..Lottery::default()
        };
    
        let lottey_solana_account = SolanaAccount {
            owner: lottery_program_id,
            lamports: solana_sdk::native_token::sol_to_lamports(1.0),
            data: lottery_account.try_to_vec().unwrap(),
            ..SolanaAccount::default()
        };
    
        ptc.set_account(
            &lottery_account_pda.0,
            &SolanaSharedDataAccount::from(lottey_solana_account)
        );

        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 350);

        let instruction = instruction_withdraw_succeed_lottery(
            lottery_account_pda.0, 
            config_account_pda.0, 
            lottery_authority_account.pubkey(), 
            lottery_ata_pda, 
            funding_usdc_token_account_pubkey, 
            config_account.usdc_mint_account, 
            TOKEN_STANDARD_PROGRAM_ID
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ], 
            Some(&ptc.payer.pubkey()), 
            &[
                &ptc.payer,
                &lottery_authority_account
            ], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::WinnersNotSelected as u32
                )
            )
        );
    }
    // failure - trying to withdraw funds before ending lottery and picking winners
}

#[tokio::test]
async fn test_withdraw_lottery_winners() {
    let lottery_program_id = LOTTERY_PROGRAM_ID;
    let mut pt = setup_program_test(lottery_program_id);

    //////////////////////? add config account
    let config_account_pda = Pubkey::find_program_address(
        &[
            b"solottery_program_config_account"
        ],
        &lottery_program_id
    );

    let config_account = Config {
        discriminator: Config::get_discriminator(),
        canonical_bump: config_account_pda.1,
        lottery_creation_fee: 5_000000, // 5 USDC
        maximum_number_of_winners: 10,
        usdc_mint_account: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
        maximum_time_for_lottery_account: 1000,
        minimum_tickets_to_be_sold_in_lottery: 20,
        max_lottery_description_bytes: 10,
        ..Config::default()
    };
    let config_account_data = config_account.try_to_vec().unwrap();

    let config_solana_account = SolanaAccount {
        owner: lottery_program_id,
        data: config_account_data,
        lamports: sol_to_lamports(0.0009),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account_pda.0,
        config_solana_account
    );
    //////////////////////? add config account
    
    //////////////////////? add user account authority
    let user_account_auth = Keypair::new();
    pt.add_account(
        user_account_auth.pubkey(),
        SolanaAccount {
            lamports: sol_to_lamports(1.0),
            ..SolanaAccount::default()
        }
    );
    //////////////////////? add user account authority
    
    //////////////////////? add lottery account
    let lottery_account_pda = Pubkey::find_program_address(
        &[
            b"lottery_account",
            Pubkey::default().to_bytes().as_slice(),
            get_lottery_literal_seed(&String::from("1")).as_slice()
        ],
        &lottery_program_id
    );

    let user_account_pda = Pubkey::find_program_address(
        &[
            b"user_account",
            user_account_auth.pubkey().to_bytes().as_slice(),
            &lottery_account_pda.0.to_bytes().as_slice()
        ],
        &lottery_program_id
    );

    let lottery_account = Lottery {
        discriminator: Lottery::get_discriminator(),
        canonical_bump: lottery_account_pda.1,
        starting_time: 100,
        ending_time: 200,
        lottery_description: String::from("1"),
        winners_count: 5,
        minimum_tickets_amount_required_to_be_sold: 5,
        tickets_total_amount: 10,
        protocol_fee: 95_000000,
        lottery_creation_fee: 5_000000,
        is_ended_successfuly: true,
        arbitrary_mint_account_address: Pubkey::new_from_array([5; 32]),
        winners: vec![
            (user_account_pda.0, false),
            (Pubkey::new_from_array([56; 32]), false),
            (user_account_pda.0, false),
            (Pubkey::new_from_array([58; 32]), false),
            (user_account_pda.0, false)
        ],
        fund_amount: 1000_000000,
        ..Lottery::default()
    };

    let lottey_solana_account = SolanaAccount {
        owner: lottery_program_id,
        lamports: solana_sdk::native_token::sol_to_lamports(1.0),
        data: lottery_account.try_to_vec().unwrap(),
        ..SolanaAccount::default()
    };

    pt.add_account(
        lottery_account_pda.0,
        lottey_solana_account
    );
    //////////////////////? add lottery account
    
    //////////////////////? add user account
    let user_account = User {
        discriminator: User::get_discriminator(),
        canonical_bump: user_account_pda.1,
        authority: user_account_auth.pubkey(),
        lottery: lottery_account_pda.0,
        ..User::default()
    };

    let user_solana_account = SolanaAccount {
        owner: lottery_program_id,
        lamports: sol_to_lamports(1.0),
        data: user_account.try_to_vec().unwrap(),
        ..SolanaAccount::default()
    };
    
    pt.add_account(
        user_account_pda.0,
        user_solana_account
    );
    //////////////////////? add user account
    
    //////////////////////? add unkown user account
    let unknown_user_account_auth = Keypair::new();
    let unknown_user_account_pda = Pubkey::find_program_address(
        &[
            b"user_account",
            unknown_user_account_auth.pubkey().to_bytes().as_slice(),
            &lottery_account_pda.0.to_bytes().as_slice()
        ],
        &lottery_program_id
    );

    let unknown_user_account = User {
        discriminator: User::get_discriminator(),
        canonical_bump: unknown_user_account_pda.1,
        authority: unknown_user_account_auth.pubkey(),
        lottery: lottery_account_pda.0,
        ..User::default()
    };

    let unknown_user_solana_account = SolanaAccount {
        owner: lottery_program_id,
        lamports: sol_to_lamports(1.0),
        data: unknown_user_account.try_to_vec().unwrap(),
        ..SolanaAccount::default()
    };
    
    pt.add_account(
        unknown_user_account_pda.0,
        unknown_user_solana_account
    );
    //////////////////////? add unkown user account

    //////////////////////? add Arbitrary mint account
    let arbitrary_mint_account = MintAccount {
        supply: 1000_000000,
        decimals: 6,
        is_initialized: true,
        ..MintAccount::default()
    };

    let mut arbitrary_mint_account_data: [u8; MintAccount::LEN] = [0; MintAccount::LEN];
    MintAccount::pack(
        arbitrary_mint_account,
        arbitrary_mint_account_data.as_mut_slice()
    ).unwrap();

    let arbitrary_mint_solana_account = SolanaAccount {
        lamports: sol_to_lamports(0.01),
        owner: TOKEN_STANDARD_PROGRAM_ID,
        data: arbitrary_mint_account_data.to_vec(),
        ..SolanaAccount::default()
    };

    pt.add_account(
        lottery_account.arbitrary_mint_account_address,
        arbitrary_mint_solana_account
    );
    //////////////////////? add Arbitrary mint account
    
    //////////////////////? add fund receiver Arbitrary token account
    let funding_arbitrary_token_account_pubkey = Pubkey::new_from_array([1; 32]);
    let funding_arbitrary_token_account = TokenAccount {
        mint: lottery_account.arbitrary_mint_account_address,
        amount: 0_000000,
        state: TokenAccountState::Initialized,
        ..TokenAccount::default()
    };

    let mut funding_arbitrary_token_account_data: [u8; TokenAccount::LEN] = [0; TokenAccount::LEN];
    TokenAccount::pack(
        funding_arbitrary_token_account,
        funding_arbitrary_token_account_data.as_mut_slice()
    ).unwrap();

    let funding_arbitrary_token_solana_account = SolanaAccount {
        owner: TOKEN_STANDARD_PROGRAM_ID,
        data: funding_arbitrary_token_account_data.to_vec(),
        lamports: sol_to_lamports(0.01),
        ..SolanaAccount::default()
    };

    pt.add_account(
        funding_arbitrary_token_account_pubkey,
        funding_arbitrary_token_solana_account
    );
    //////////////////////? add fund receiver Arbitrary token account
    
    //////////////////////? add lottery's Arbitrary ata
    let lottery_ata_pda = get_associated_token_address(
        &lottery_account_pda.0,
        &lottery_account.arbitrary_mint_account_address
    );
    let lottery_ata = TokenAccount {
        state: TokenAccountState::Initialized,
        mint: lottery_account.arbitrary_mint_account_address,
        amount: 1000_000000,
        owner: lottery_account_pda.0,
        ..TokenAccount::default()
    };

    let mut lottery_ata_data = [0u8; TokenAccount::LEN];
    TokenAccount::pack(
        lottery_ata,
        lottery_ata_data.as_mut_slice()
    ).unwrap();

    let lottery_ata_solana_account = SolanaAccount {
        owner: spl_token::ID,
        data: lottery_ata_data.to_vec(),
        lamports: sol_to_lamports(1.0),
        ..SolanaAccount::default()
    };

    pt.add_account(
        lottery_ata_pda,
        lottery_ata_solana_account
    );
    //////////////////////? add lottery's Arbitrary ata
    
    let mut ptc = pt.start_with_context().await;

    // failure - invalid arbitrary mint account
    {
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 350);

        let instruction = instruction_withdraw_lottery_winners(
            lottery_account_pda.0, 
            user_account_pda.0, 
            user_account_auth.pubkey(), 
            lottery_ata_pda, 
            funding_arbitrary_token_account_pubkey, 
            Pubkey::new_unique(), 
            TOKEN_STANDARD_PROGRAM_ID, 
            config_account_pda.0
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ], 
            Some(&ptc.payer.pubkey()), 
            &[
                &ptc.payer,
                &user_account_auth
            ], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidArbitraryMintAccount as u32
                )
            )
        );
    }
    // failure - invalid arbitrary mint account

    // failure - invalid lottery arbitrary ata
    {
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 350);

        let instruction = instruction_withdraw_lottery_winners(
            lottery_account_pda.0, 
            user_account_pda.0, 
            user_account_auth.pubkey(), 
            Pubkey::new_unique(), 
            funding_arbitrary_token_account_pubkey, 
            lottery_account.arbitrary_mint_account_address, 
            TOKEN_STANDARD_PROGRAM_ID, 
            config_account_pda.0
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ], 
            Some(&ptc.payer.pubkey()), 
            &[
                &ptc.payer,
                &user_account_auth
            ], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidLotteryArbitraryAssociatedTokenAccount as u32
                )
            )
        );
    }
    // failure - invalid lottery arbitrary ata

    // failure - winner not found
    {
        // 1. winner tries to double call the instruction
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 350);

        let instruction = instruction_withdraw_lottery_winners(
            lottery_account_pda.0, 
            user_account_pda.0, 
            user_account_auth.pubkey(), 
            lottery_ata_pda, 
            funding_arbitrary_token_account_pubkey, 
            lottery_account.arbitrary_mint_account_address, 
            TOKEN_STANDARD_PROGRAM_ID, 
            config_account_pda.0
        );

        let tx = Transaction::new_signed_with_payer(
            &[
                instruction.clone(),
                instruction
            ], 
            Some(&ptc.payer.pubkey()), 
            &[
                &ptc.payer,
                &user_account_auth
            ], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                1,
                InstructionError::Custom(
                    LotteryError::WinnerNotFound as u32
                )
            )
        );

        // 2. unknown user (which is not one of the winners) calls the instruction
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 350);

        let instruction = instruction_withdraw_lottery_winners(
            lottery_account_pda.0, 
            unknown_user_account_pda.0, 
            unknown_user_account_auth.pubkey(), 
            lottery_ata_pda, 
            funding_arbitrary_token_account_pubkey, 
            lottery_account.arbitrary_mint_account_address, 
            TOKEN_STANDARD_PROGRAM_ID, 
            config_account_pda.0
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ], 
            Some(&ptc.payer.pubkey()), 
            &[
                &ptc.payer,
                &unknown_user_account_auth
            ], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::WinnerNotFound as u32
                )
            )
        );
    }
    // failure - winner not found

    // success
    {
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 350);

        let instruction = instruction_withdraw_lottery_winners(
            lottery_account_pda.0, 
            user_account_pda.0, 
            user_account_auth.pubkey(), 
            lottery_ata_pda, 
            funding_arbitrary_token_account_pubkey, 
            lottery_account.arbitrary_mint_account_address, 
            TOKEN_STANDARD_PROGRAM_ID, 
            config_account_pda.0
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ], 
            Some(&ptc.payer.pubkey()), 
            &[
                &ptc.payer,
                &user_account_auth
            ], 
            ptc.last_blockhash
        );

        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        let SolanaAccount { data: lottery_data_account, .. } = ptc
            .banks_client
            .get_account(lottery_account_pda.0)
            .await
            .unwrap()
            .unwrap();
    
        let SolanaAccount { data: fund_receiver_token_account_data, .. } = ptc
            .banks_client
            .get_account(funding_arbitrary_token_account_pubkey)
            .await
            .unwrap()
            .unwrap();

        let SolanaAccount { data: lottery_ata_data, .. } = ptc
            .banks_client
            .get_account(lottery_ata_pda)
            .await
            .unwrap()
            .unwrap();

        let Lottery { winners, .. } = Lottery::deserialize(
            &mut &lottery_data_account[..]
        ).unwrap();

        assert_eq!(
            winners[0].1,
            true,
            "invalid winners[0].1"
        );
        assert_eq!(
            winners[2].1,
            true,
            "invalid winners[2].1"
        );
        assert_eq!(
            winners[4].1,
            true,
            "invalid winners[4].1"
        );

        let TokenAccount { amount, .. } = TokenAccount::unpack(
            &fund_receiver_token_account_data
        ).unwrap();

        assert_eq!(
            amount,
            600_000000,
            "invalid fund receiver token account balance."
        );

        let TokenAccount { amount, .. } = TokenAccount::unpack(
            &lottery_ata_data
        ).unwrap();

        assert_eq!(
            amount,
            400_000000,
            "invalid lottery ata balance."
        );
    }
    // success
}

#[tokio::test]
async fn test_withdraw_failed_lottery() {
    let lottery_program_id = LOTTERY_PROGRAM_ID;
    let mut pt = setup_program_test(lottery_program_id);

    //////////////////////? add config account
    let config_account_pda = Pubkey::find_program_address(
        &[
            b"solottery_program_config_account"
        ],
        &lottery_program_id
    );

    let config_account = Config {
        discriminator: Config::get_discriminator(),
        canonical_bump: config_account_pda.1,
        usdc_mint_account: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
        ..Config::default()
    };
    let config_account_data = config_account.try_to_vec().unwrap();

    let config_solana_account = SolanaAccount {
        owner: lottery_program_id,
        data: config_account_data,
        lamports: sol_to_lamports(0.0009),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account_pda.0,
        config_solana_account
    );
    //////////////////////? add config account
    
    //////////////////////? add lottery account
    let lottery_account_auth = Keypair::new();
    let lottery_account_pda = Pubkey::find_program_address(
        &[
            b"lottery_account",
            lottery_account_auth.pubkey().to_bytes().as_slice(),
            get_lottery_literal_seed(&String::from("1")).as_slice()
        ],
        &lottery_program_id
    );
    
    let lottery_account = Lottery {
        discriminator: Lottery::get_discriminator(),
        canonical_bump: lottery_account_pda.1,
        tickets_total_amount: 45,
        minimum_tickets_amount_required_to_be_sold: 50,
        lottery_description: String::from("1"),
        authority: lottery_account_auth.pubkey(),
        lottery_creation_fee: 10_000000, // 10 USDCs
        starting_time: 200,
        ending_time: 350,
        arbitrary_mint_account_address: Pubkey::new_from_array([1; 32]),
        ..Lottery::default()
    };

    let lottey_solana_account = SolanaAccount {
        owner: lottery_program_id,
        lamports: solana_sdk::native_token::sol_to_lamports(1.0),
        data: lottery_account.try_to_vec().unwrap(),
        ..SolanaAccount::default()
    };

    pt.add_account(
        lottery_account_pda.0,
        lottey_solana_account
    );
    //////////////////////? add lottery account
    
    //////////////////////? add UDSC mint account
    let usdc_mint_account = MintAccount {
        supply: 100_000000,
        decimals: 6,
        is_initialized: true,
        ..MintAccount::default()
    };

    let mut usdc_mint_account_data = [0u8; MintAccount::LEN];
    MintAccount::pack(
        usdc_mint_account,
        usdc_mint_account_data.as_mut_slice()
    ).unwrap();

    let usdc_mint_solana_account = SolanaAccount {
        data: usdc_mint_account_data.to_vec(),
        owner: TOKEN_STANDARD_PROGRAM_ID,
        lamports: sol_to_lamports(1.0),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account.usdc_mint_account,
        usdc_mint_solana_account
    );
    //////////////////////? add USDC mint account
    
    //////////////////////? add Arbitrary mint account
    let arbitrary_mint_account_pubkey = Pubkey::new_from_array([1; 32]);
    let arbitrary_mint_account = MintAccount {
        supply: 500_000,
        decimals: 3,
        is_initialized: true,
        ..MintAccount::default()
    };

    let mut arbitrary_mint_account_data = [0u8; MintAccount::LEN];
    MintAccount::pack(
        arbitrary_mint_account,
        arbitrary_mint_account_data.as_mut_slice()
    ).unwrap();

    let arbitrary_mint_solana_account = SolanaAccount {
        data: arbitrary_mint_account_data.to_vec(),
        owner: TOKEN_STANDARD_PROGRAM_ID,
        lamports: sol_to_lamports(1.0),
        ..SolanaAccount::default()
    };

    pt.add_account(
        arbitrary_mint_account_pubkey,
        arbitrary_mint_solana_account
    );
    //////////////////////? add Arbitrary mint account
    
    //////////////////////? add lottery's USDC ata
    let lottery_usdc_ata_pubkey = get_associated_token_address(
        &lottery_account_pda.0, 
        &config_account.usdc_mint_account
    );
    let lottery_usdc_ata = TokenAccount {
        amount: 55_000000,
        owner: lottery_account_pda.0,
        state: TokenAccountState::Initialized,
        mint: config_account.usdc_mint_account,
        ..TokenAccount::default()
    };

    let mut lottery_usdc_ata_data = [0u8; TokenAccount::LEN];
    TokenAccount::pack(
        lottery_usdc_ata,
        lottery_usdc_ata_data.as_mut_slice()
    ).unwrap();

    let lottery_usdc_ata_solana_account = SolanaAccount {
        data: lottery_usdc_ata_data.to_vec(),
        owner: TOKEN_STANDARD_PROGRAM_ID,
        lamports: sol_to_lamports(1.0),
        ..SolanaAccount::default()
    };

    pt.add_account(
        lottery_usdc_ata_pubkey,
        lottery_usdc_ata_solana_account
    );
    //////////////////////? add lottery's USDC ata
    
    //////////////////////? add lottery's Arbitrary ata
    let lottery_arbitrary_ata_pubkey = get_associated_token_address(
        &lottery_account_pda.0, 
        &arbitrary_mint_account_pubkey
    );
    let lottery_arbitrary_ata = TokenAccount {
        amount: 350_000,
        owner: lottery_account_pda.0,
        state: TokenAccountState::Initialized,
        mint: lottery_account.arbitrary_mint_account_address,
        ..TokenAccount::default()
    };

    let mut lottery_arbitrary_ata_data = [0u8; TokenAccount::LEN];
    TokenAccount::pack(
        lottery_arbitrary_ata,
        lottery_arbitrary_ata_data.as_mut_slice()
    ).unwrap();

    let lottery_arbitrary_ata_solana_account = SolanaAccount {
        data: lottery_arbitrary_ata_data.to_vec(),
        owner: TOKEN_STANDARD_PROGRAM_ID,
        lamports: 2039280,
        ..SolanaAccount::default()
    };

    pt.add_account(
        lottery_arbitrary_ata_pubkey,
        lottery_arbitrary_ata_solana_account
    );
    //////////////////////? add lottery's Arbitrary ata
    
    //////////////////////? add fund receiver, USDC token account
    let fund_receiver_usdc_token_account_pubkey = Pubkey::new_from_array([2; 32]);
    let fund_receiver_usdc_token_account = TokenAccount {
        amount: 0_000000,
        owner: Pubkey::new_unique(),
        state: TokenAccountState::Initialized,
        mint: config_account.usdc_mint_account,
        ..TokenAccount::default()
    };

    let mut fund_receiver_usdc_token_account_data = [0u8; TokenAccount::LEN];
    TokenAccount::pack(
        fund_receiver_usdc_token_account,
        fund_receiver_usdc_token_account_data.as_mut_slice()
    ).unwrap();

    let fund_receiver_usdc_token_solana_account = SolanaAccount {
        data: fund_receiver_usdc_token_account_data.to_vec(),
        owner: TOKEN_STANDARD_PROGRAM_ID,
        lamports: sol_to_lamports(1.0),
        ..SolanaAccount::default()
    };

    pt.add_account(
        fund_receiver_usdc_token_account_pubkey,
        fund_receiver_usdc_token_solana_account
    );
    //////////////////////? add fund receiver, USDC token account
    
    //////////////////////? add fund receiver, Arbitrary token account
    let fund_receiver_arbitrary_token_account_pubkey = Pubkey::new_from_array([3; 32]);
    let fund_receiver_arbitrary_token_account = TokenAccount {
        amount: 0_000,
        owner: Pubkey::new_unique(),
        state: TokenAccountState::Initialized,
        mint: lottery_account.arbitrary_mint_account_address,
        ..TokenAccount::default()
    };

    let mut fund_receiver_arbitrary_token_account_data = [0u8; TokenAccount::LEN];
    TokenAccount::pack(
        fund_receiver_arbitrary_token_account,
        fund_receiver_arbitrary_token_account_data.as_mut_slice()
    ).unwrap();

    let fund_receiver_arbitrary_token_solana_account = SolanaAccount {
        data: fund_receiver_arbitrary_token_account_data.to_vec(),
        owner: TOKEN_STANDARD_PROGRAM_ID,
        lamports: sol_to_lamports(1.0),
        ..SolanaAccount::default()
    };

    pt.add_account(
        fund_receiver_arbitrary_token_account_pubkey,
        fund_receiver_arbitrary_token_solana_account
    );
    //////////////////////? add fund receiver, Arbitrary token account
    
    //////////////////////? add fund receiver, rent-exempt's lamports account
    let fund_receiver_rent_exempt_lamports = Pubkey::new_from_array([4; 32]);
    pt.add_account(
        fund_receiver_rent_exempt_lamports,
        SolanaAccount {
            owner: SYSTEM_PROGRAM_ID,
            lamports: sol_to_lamports(0.1),
            ..SolanaAccount::default()
        }
    );
    //////////////////////? add fund receiver, rent-exempt's lamports account
    
    let mut ptc = pt.start_with_context().await;

    // failure - funds already withdrawed
    {
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 555);

        let instruction = instruction_withdraw_failed_lottery(
            config_account_pda.0, 
            lottery_account_pda.0, 
            lottery_account_auth.pubkey(), 
            config_account.usdc_mint_account, 
            lottery_account.arbitrary_mint_account_address, 
            lottery_usdc_ata_pubkey, 
            lottery_arbitrary_ata_pubkey, 
            fund_receiver_usdc_token_account_pubkey, 
            fund_receiver_arbitrary_token_account_pubkey, 
            fund_receiver_rent_exempt_lamports, 
            TOKEN_STANDARD_PROGRAM_ID
        );

        let tx = Transaction::new_signed_with_payer(
            &[ 
                instruction.clone(),
                instruction
            ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &lottery_account_auth
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                1,
                InstructionError::Custom(
                    LotteryError::FundsAlreadyWithdrawed as u32
                )
            )
        );
    }
    // failure - funds already withdrawed

    // failure - invalid lottery account auth
    {
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 555);

        let unkown_account = Keypair::new();
        let instruction = instruction_withdraw_failed_lottery(
            config_account_pda.0, 
            lottery_account_pda.0, 
            unkown_account.pubkey(), 
            config_account.usdc_mint_account, 
            lottery_account.arbitrary_mint_account_address, 
            lottery_usdc_ata_pubkey, 
            lottery_arbitrary_ata_pubkey, 
            fund_receiver_usdc_token_account_pubkey, 
            fund_receiver_arbitrary_token_account_pubkey, 
            fund_receiver_rent_exempt_lamports, 
            TOKEN_STANDARD_PROGRAM_ID
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &unkown_account
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidLotteryAccountAuthority as u32
                )
            )
        );
    }
    // failure - invalid lottery account auth

    // success
    {
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 555);

        // let instruction_1 = spl_associated_token_account::instruction::create_associated_token_account_idempotent(
        //     &ptc.payer.pubkey(), 
        //     &fund_receiver_usdc_token_account_pubkey, 
        //     &config_account.usdc_mint_account, 
        //     &TOKEN_STANDARD_PROGRAM_ID
        // );

        // let instruction_2 = spl_associated_token_account::instruction::create_associated_token_account_idempotent(
        //     &ptc.payer.pubkey(), 
        //     &fund_receiver_arbitrary_token_account_pubkey, 
        //     &lottery_account.arbitrary_mint_account_address, 
        //     &TOKEN_STANDARD_PROGRAM_ID
        // );

        let instruction = instruction_withdraw_failed_lottery(
            config_account_pda.0, 
            lottery_account_pda.0, 
            lottery_account_auth.pubkey(), 
            config_account.usdc_mint_account, 
            lottery_account.arbitrary_mint_account_address, 
            lottery_usdc_ata_pubkey, 
            lottery_arbitrary_ata_pubkey, 
            fund_receiver_usdc_token_account_pubkey, 
            fund_receiver_arbitrary_token_account_pubkey, 
            fund_receiver_rent_exempt_lamports, 
            TOKEN_STANDARD_PROGRAM_ID
        );

        let tx = Transaction::new_signed_with_payer(
            &[
                // instruction_1,
                // instruction_2,
                instruction
            ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &lottery_account_auth
            ],
            ptc.last_blockhash
        );

        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        let SolanaAccount { data: lottery_account_data, .. } = ptc
            .banks_client
            .get_account(lottery_account_pda.0)
            .await
            .unwrap()
            .unwrap();

        let Lottery { is_creator_withdrawed_when_lottery_was_failed, .. } = Lottery::deserialize(
            &mut &lottery_account_data[..]
        ).unwrap();

        assert_eq!(
            is_creator_withdrawed_when_lottery_was_failed,
            true,
            "lottery account was not updated."
        );

        let SolanaAccount { data: lottery_usdc_ata_data, .. } = ptc
            .banks_client
            .get_account(lottery_usdc_ata_pubkey)
            .await
            .unwrap()
            .unwrap();

        let error = ptc
            .banks_client
            .get_account(lottery_arbitrary_ata_pubkey)
            .await
            .unwrap();
        if error.is_some() {
            panic!("Account must be closed and we cannot get its data from blockchain.");
        };

        let SolanaAccount { data: fund_receiver_usdc_token_account_data, .. } = ptc
            .banks_client
            .get_account(fund_receiver_usdc_token_account_pubkey)
            .await
            .unwrap()
            .unwrap();

        let SolanaAccount { data: fund_receiver_arbitrary_token_account_data, .. } = ptc
            .banks_client
            .get_account(fund_receiver_arbitrary_token_account_pubkey)
            .await
            .unwrap()
            .unwrap();

        let SolanaAccount { lamports, .. } = ptc
            .banks_client
            .get_account(fund_receiver_rent_exempt_lamports)
            .await
            .unwrap()
            .unwrap();
        let rent_exempt_lamports = (
            (
                (ACCOUNT_STORAGE_OVERHEAD + TokenAccount::LEN as u64) * DEFAULT_LAMPORTS_PER_BYTE_YEAR
            ) as f64 * DEFAULT_EXEMPTION_THRESHOLD
        ) as u64;

        assert_eq!(
            lamports,
            rent_exempt_lamports + sol_to_lamports(0.1),
            "invalid rent_exempt receiver lamport balance."
        );

        let TokenAccount { amount: lottery_usdc_ata_balance, .. } = TokenAccount::unpack(
            &lottery_usdc_ata_data
        ).unwrap();

        let TokenAccount { amount: fund_receiver_usdc_token_account_balance, .. } = TokenAccount::unpack(
            &fund_receiver_usdc_token_account_data
        ).unwrap();

        let TokenAccount { amount: fund_receiver_arbitrary_token_account_balance, .. } = TokenAccount::unpack(
            &fund_receiver_arbitrary_token_account_data
        ).unwrap();

        assert_eq!(
            55_000000 - 10_000000,
            lottery_usdc_ata_balance,
            "invalid lottery_usdc_ata_balance."
        );

        assert_eq!(
            fund_receiver_usdc_token_account_balance,
            10_000000,
            "invalid fund_receiver_usdc_token_account_balance."
        );

        assert_eq!(
            fund_receiver_arbitrary_token_account_balance,
            350_000,
            "invalid fund_receiver_arbitrary_token_account_balance."
        );
    }
    // success

    // failure - lottery is not in correct state
    {
        let mut new_lottery_account = lottery_account.clone();
        new_lottery_account.minimum_tickets_amount_required_to_be_sold = 80;
        new_lottery_account.tickets_total_amount = 99;

        ptc.set_account(
           &lottery_account_pda.0,
           &SolanaSharedDataAccount::from(
                SolanaAccount {
                    data: new_lottery_account.try_to_vec().unwrap(),
                    owner: LOTTERY_PROGRAM_ID,
                    lamports: sol_to_lamports(1.0),
                    ..SolanaAccount::default()
                }
           )
        );

        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 555);

        let instruction = instruction_withdraw_failed_lottery(
            config_account_pda.0, 
            lottery_account_pda.0, 
            lottery_account_auth.pubkey(), 
            config_account.usdc_mint_account, 
            lottery_account.arbitrary_mint_account_address, 
            lottery_usdc_ata_pubkey, 
            lottery_arbitrary_ata_pubkey, 
            fund_receiver_usdc_token_account_pubkey, 
            fund_receiver_arbitrary_token_account_pubkey, 
            fund_receiver_rent_exempt_lamports, 
            TOKEN_STANDARD_PROGRAM_ID
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &lottery_account_auth
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidLotteryState as u32
                )
            )
        );
    }
    // failure - lottery is not in correct state
}

#[tokio::test]
async fn test_withdraw_and_close_failed_user() {
    let lottery_program_id = LOTTERY_PROGRAM_ID;
    let mut pt = setup_program_test(lottery_program_id);

    //////////////////////? add config account
    let config_account_pda = Pubkey::find_program_address(
        &[
            b"solottery_program_config_account"
        ],
        &lottery_program_id
    );

    let config_account = Config {
        discriminator: Config::get_discriminator(),
        canonical_bump: config_account_pda.1,
        usdc_mint_account: Pubkey::new_from_array([1; 32]),
        ..Config::default()
    };
    let config_account_data = config_account.try_to_vec().unwrap();

    let config_solana_account = SolanaAccount {
        owner: lottery_program_id,
        data: config_account_data,
        lamports: sol_to_lamports(0.0009),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account_pda.0,
        config_solana_account
    );
    //////////////////////? add config account
    
    //////////////////////? add lottery account
    let lottery_account_pda = Pubkey::find_program_address(
        &[
            b"lottery_account",
            Pubkey::default().to_bytes().as_slice(),
            get_lottery_literal_seed(&String::from("1")).as_slice()
        ],
        &lottery_program_id
    );

    let mut lottery_account = Lottery {
        discriminator: Lottery::get_discriminator(),
        canonical_bump: lottery_account_pda.1,
        starting_time: 100,
        ending_time: 200,
        lottery_description: String::from("1"),
        minimum_tickets_amount_required_to_be_sold: 100,
        tickets_total_amount: 90,
        ..Lottery::default()
    };
    lottery_account.initial_bytes = lottery_account.try_to_vec().unwrap().len() as u64;

    let lottey_solana_account = SolanaAccount {
        owner: lottery_program_id,
        lamports: solana_sdk::native_token::sol_to_lamports(1.0),
        data: vec![
            lottery_account.try_to_vec().unwrap(),
            vec![0u8; 320]
        ].concat(),
        ..SolanaAccount::default()
    };

    pt.add_account(
        lottery_account_pda.0,
        lottey_solana_account
    );
    //////////////////////? add lottery account
    
    //////////////////////? add user account
    let user_account_auth = Keypair::new();
    pt.add_account(
        user_account_auth.pubkey(),
        SolanaAccount {
            lamports: sol_to_lamports(1.0),
            ..SolanaAccount::default()
        }
    );

    let user_account_pda = Pubkey::find_program_address(
        &[
            b"user_account",
            user_account_auth.pubkey().to_bytes().as_slice(),
            &lottery_account_pda.0.to_bytes().as_slice()
        ],
        &lottery_program_id
    );

    let user_account = User {
        discriminator: User::get_discriminator(),
        canonical_bump: user_account_pda.1,
        authority: user_account_auth.pubkey(),
        lottery: lottery_account_pda.0,
        total_rent_exempt_paied: 100000,
        total_tickets_acquired: 10,
        total_tickets_value: 100_000000, // USDC
        ..User::default()
    };

    let user_solana_account = SolanaAccount {
        owner: lottery_program_id,
        lamports: sol_to_lamports(1.0),
        data: user_account.try_to_vec().unwrap(),
        ..SolanaAccount::default()
    };
    
    pt.add_account(
        user_account_pda.0,
        user_solana_account
    );
    //////////////////////? add user account
    
    //////////////////////? add UDSC mint account
    let usdc_mint_account = MintAccount {
        supply: 1000_000000,
        decimals: 6,
        is_initialized: true,
        ..MintAccount::default()
    };

    let mut usdc_mint_account_data = [0u8; MintAccount::LEN];
    MintAccount::pack(
        usdc_mint_account,
        usdc_mint_account_data.as_mut_slice()
    ).unwrap();

    let usdc_mint_solana_account = SolanaAccount {
        data: usdc_mint_account_data.to_vec(),
        owner: TOKEN_STANDARD_PROGRAM_ID,
        lamports: sol_to_lamports(1.0),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account.usdc_mint_account,
        usdc_mint_solana_account
    );
    //////////////////////? add USDC mint account
    
    //////////////////////? add lottery's USDC ata
    let lottery_usdc_ata_pubkey = get_associated_token_address(
        &lottery_account_pda.0, 
        &config_account.usdc_mint_account
    );
    let lottery_usdc_ata = TokenAccount {
        amount: 1000_000000,
        owner: lottery_account_pda.0,
        state: TokenAccountState::Initialized,
        mint: config_account.usdc_mint_account,
        ..TokenAccount::default()
    };

    let mut lottery_usdc_ata_data = [0u8; TokenAccount::LEN];
    TokenAccount::pack(
        lottery_usdc_ata,
        lottery_usdc_ata_data.as_mut_slice()
    ).unwrap();

    let lottery_usdc_ata_solana_account = SolanaAccount {
        data: lottery_usdc_ata_data.to_vec(),
        owner: TOKEN_STANDARD_PROGRAM_ID,
        lamports: sol_to_lamports(1.0),
        ..SolanaAccount::default()
    };

    pt.add_account(
        lottery_usdc_ata_pubkey,
        lottery_usdc_ata_solana_account
    );
    //////////////////////? add lottery's USDC ata
    
    //////////////////////? add fund receiver, USDC token account
    let fund_receiver_usdc_token_account_pubkey = Pubkey::new_from_array([2; 32]);
    let fund_receiver_usdc_token_account = TokenAccount {
        amount: 0_000000,
        owner: user_account_auth.pubkey(),
        state: TokenAccountState::Initialized,
        mint: config_account.usdc_mint_account,
        ..TokenAccount::default()
    };

    let mut fund_receiver_usdc_token_account_data = [0u8; TokenAccount::LEN];
    TokenAccount::pack(
        fund_receiver_usdc_token_account,
        fund_receiver_usdc_token_account_data.as_mut_slice()
    ).unwrap();

    let fund_receiver_usdc_token_solana_account = SolanaAccount {
        data: fund_receiver_usdc_token_account_data.to_vec(),
        owner: TOKEN_STANDARD_PROGRAM_ID,
        lamports: sol_to_lamports(1.0),
        ..SolanaAccount::default()
    };

    pt.add_account(
        fund_receiver_usdc_token_account_pubkey,
        fund_receiver_usdc_token_solana_account
    );
    //////////////////////? add fund receiver, USDC token account
    
    let mut ptc = pt.start_with_context().await;

    // success
    {
        ptc.
            get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 350);

        let instruction = instruction_withdraw_and_close_failed_user(
            config_account_pda.0, 
            lottery_account_pda.0, 
            user_account_pda.0, 
            user_account_auth.pubkey(), 
            config_account.usdc_mint_account, 
            lottery_usdc_ata_pubkey, 
            fund_receiver_usdc_token_account_pubkey, 
            user_account_auth.pubkey(), 
            user_account_auth.pubkey(), 
            TOKEN_STANDARD_PROGRAM_ID
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &user_account_auth
            ],
            ptc.last_blockhash
        );

        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        let error = ptc
            .banks_client
            .get_account(user_account_pda.0)
            .await
            .unwrap();
        if error.is_some() {
            panic!("Account must be closed.");
        };

        let SolanaAccount { lamports: lottery_account_lamport_balance, data, .. } = ptc
            .banks_client
            .get_account(lottery_account_pda.0)
            .await
            .unwrap()
            .unwrap();
        
        assert_eq!(
            lottery_account_lamport_balance,
            sol_to_lamports(1.0) - 100000,
            "invalid lottery account lamport balance."
        );

        assert_eq!(
            data.len(),
            lottery_account.try_to_vec().unwrap().len(),
            "invalid lottery account data size."
        );

        let SolanaAccount { lamports: user_account_auth_lamport_balance, .. } = ptc
            .banks_client
            .get_account(user_account_auth.pubkey())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            user_account_auth_lamport_balance,
            sol_to_lamports(1.0) +
            100000 + // tickets rent exempt lamports
            sol_to_lamports(1.0), // user account's lamport balance
            "invalid user account authority's lamport balance."
        );

        let SolanaAccount { data: lottery_usdc_ata_data, .. } = ptc
            .banks_client
            .get_account(lottery_usdc_ata_pubkey)
            .await
            .unwrap()
            .unwrap();

        let TokenAccount { amount: lottery_usdc_ata_balance, .. } = TokenAccount::unpack(
            &lottery_usdc_ata_data
        ).unwrap();

        assert_eq!(
            lottery_usdc_ata_balance,
            1000_000000 - 100_000000,
            "invalid lottery USDC ata token balance."
        );

        let SolanaAccount { data: fund_receiver_usdc_token_account_data, .. } = ptc
            .banks_client
            .get_account(fund_receiver_usdc_token_account_pubkey)
            .await
            .unwrap()
            .unwrap();

        let TokenAccount { amount: fund_receiver_usdc_token_account_balance, .. } = TokenAccount::unpack(
            &fund_receiver_usdc_token_account_data
        ).unwrap();

        assert_eq!(
            fund_receiver_usdc_token_account_balance,
            100_000000,
            "invalid fund receiver usdc token account balance."
        );
    }
    // success
}

#[tokio::test]
async fn test_close_lottery_account_usdc_token_account() {
    let lottery_program_id = LOTTERY_PROGRAM_ID;
    let mut pt = setup_program_test(lottery_program_id);

    //////////////////////? add config account
    let config_account_pda = Pubkey::find_program_address(
        &[
            b"solottery_program_config_account"
        ],
        &lottery_program_id
    );

    let config_account = Config {
        discriminator: Config::get_discriminator(),
        canonical_bump: config_account_pda.1,
        usdc_mint_account: Pubkey::new_from_array([1; 32]),
        ..Config::default()
    };
    let config_account_data = config_account.try_to_vec().unwrap();

    let config_solana_account = SolanaAccount {
        owner: lottery_program_id,
        data: config_account_data,
        lamports: sol_to_lamports(0.0009),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account_pda.0,
        config_solana_account
    );
    //////////////////////? add config account
    
    //////////////////////? add lottery account
    let lottery_auth = Keypair::new();
    pt.add_account(
        lottery_auth.pubkey(),
        SolanaAccount {
            lamports: sol_to_lamports(1.0),
            owner: SYSTEM_PROGRAM_ID,
            ..SolanaAccount::default()
        }
    );

    let lottery_account_pda = Pubkey::find_program_address(
        &[
            b"lottery_account",
            lottery_auth.pubkey().to_bytes().as_slice(),
            get_lottery_literal_seed(&String::from("lottery for fun!")).as_slice()
        ],
        &lottery_program_id
    );

    let mut lottery_account = Lottery {
        discriminator: Lottery::get_discriminator(),
        canonical_bump: lottery_account_pda.1,
        starting_time: 100,
        ending_time: 200,
        lottery_description: String::from("lottery for fun!"),
        minimum_tickets_amount_required_to_be_sold: 100,
        tickets_total_amount: 90,
        is_creator_withdrawed_when_lottery_was_failed: true,
        winners_count: 10,
        authority: lottery_auth.pubkey(),
        ..Lottery::default()
    };
    lottery_account.initial_bytes = lottery_account.try_to_vec().unwrap().len() as u64 + 330;

    let lottey_solana_account = SolanaAccount {
        owner: lottery_program_id,
        lamports: solana_sdk::native_token::sol_to_lamports(1.0),
        data: vec![
            lottery_account.try_to_vec().unwrap(),
            vec![0u8; 330]
        ].concat(),
        ..SolanaAccount::default()
    };

    pt.add_account(
        lottery_account_pda.0,
        lottey_solana_account
    );
    //////////////////////? add lottery account
    
    //////////////////////? add UDSC mint account
    let usdc_mint_account = MintAccount {
        supply: 1000_000000,
        decimals: 6,
        is_initialized: true,
        ..MintAccount::default()
    };

    let mut usdc_mint_account_data = [0u8; MintAccount::LEN];
    MintAccount::pack(
        usdc_mint_account,
        usdc_mint_account_data.as_mut_slice()
    ).unwrap();

    let usdc_mint_solana_account = SolanaAccount {
        data: usdc_mint_account_data.to_vec(),
        owner: TOKEN_STANDARD_PROGRAM_ID,
        lamports: sol_to_lamports(1.0),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account.usdc_mint_account,
        usdc_mint_solana_account
    );
    //////////////////////? add USDC mint account
    
    //////////////////////? add lottery's USDC ata
    let lottery_usdc_ata_pubkey = get_associated_token_address(
        &lottery_account_pda.0, 
        &config_account.usdc_mint_account
    );
    let lottery_usdc_ata = TokenAccount {
        amount: 10_000000,
        owner: lottery_account_pda.0,
        state: TokenAccountState::Initialized,
        mint: config_account.usdc_mint_account,
        ..TokenAccount::default()
    };

    let mut lottery_usdc_ata_data = [0u8; TokenAccount::LEN];
    TokenAccount::pack(
        lottery_usdc_ata,
        lottery_usdc_ata_data.as_mut_slice()
    ).unwrap();

    let lottery_usdc_ata_solana_account = SolanaAccount {
        data: lottery_usdc_ata_data.to_vec(),
        owner: TOKEN_STANDARD_PROGRAM_ID,
        lamports: sol_to_lamports(1.0),
        ..SolanaAccount::default()
    };

    pt.add_account(
        lottery_usdc_ata_pubkey,
        lottery_usdc_ata_solana_account
    );
    //////////////////////? add lottery's USDC ata
    
    //////////////////////? add fund receiver, USDC token account
    let fund_receiver_usdc_token_account_pubkey = Pubkey::new_from_array([2; 32]);
    let fund_receiver_usdc_token_account = TokenAccount {
        amount: 0_000000,
        state: TokenAccountState::Initialized,
        mint: config_account.usdc_mint_account,
        ..TokenAccount::default()
    };

    let mut fund_receiver_usdc_token_account_data = [0u8; TokenAccount::LEN];
    TokenAccount::pack(
        fund_receiver_usdc_token_account,
        fund_receiver_usdc_token_account_data.as_mut_slice()
    ).unwrap();

    let fund_receiver_usdc_token_solana_account = SolanaAccount {
        data: fund_receiver_usdc_token_account_data.to_vec(),
        owner: TOKEN_STANDARD_PROGRAM_ID,
        lamports: sol_to_lamports(1.0),
        ..SolanaAccount::default()
    };

    pt.add_account(
        fund_receiver_usdc_token_account_pubkey,
        fund_receiver_usdc_token_solana_account
    );
    //////////////////////? add fund receiver, USDC token account
    
    let mut ptc = pt.start_with_context().await;

    // failure - lottery invalid state
    {
        // 1. LotteryState::Unknown
        ptc.
            get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 50);

        let instruction = instruction_close_lottery_account_and_usdc_token_account(
            config_account_pda.0, 
            lottery_account_pda.0, 
            lottery_auth.pubkey(), 
            config_account.usdc_mint_account, 
            lottery_usdc_ata_pubkey, 
            fund_receiver_usdc_token_account_pubkey, 
            lottery_auth.pubkey(), 
            TOKEN_STANDARD_PROGRAM_ID
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction.clone() ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &lottery_auth
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidLotteryState as u32
                )
            )
        );

        // 2. LotteryState::Successful
        ptc.
            get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 450);

        lottery_account.minimum_tickets_amount_required_to_be_sold = 100;
        lottery_account.tickets_total_amount = 150;
        ptc.set_account(
            &lottery_account_pda.0,
            &SolanaSharedDataAccount::from(
                SolanaAccount {
                    owner: LOTTERY_PROGRAM_ID,
                    lamports: sol_to_lamports(1.0),
                    data: vec![
                        lottery_account.try_to_vec().unwrap(),
                        vec![0u8; 330]
                    ].concat(),
                    ..SolanaAccount::default()
                }
            )
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &lottery_auth
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidLotteryState as u32
                )
            )
        );

        // return to default
        lottery_account.minimum_tickets_amount_required_to_be_sold = 100;
        lottery_account.tickets_total_amount = 90;
        ptc.set_account(
            &lottery_account_pda.0,
            &SolanaSharedDataAccount::from(
                SolanaAccount {
                    owner: LOTTERY_PROGRAM_ID,
                    lamports: sol_to_lamports(1.0),
                    data: vec![
                        lottery_account.try_to_vec().unwrap(),
                        vec![0u8; 330]
                    ].concat(),
                    ..SolanaAccount::default()
                }
            )
        );
    }
    // failure - lottery invalid state

    // success
    {
        ptc.
            get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 350);

        let instruction = instruction_close_lottery_account_and_usdc_token_account(
            config_account_pda.0, 
            lottery_account_pda.0, 
            lottery_auth.pubkey(), 
            config_account.usdc_mint_account, 
            lottery_usdc_ata_pubkey, 
            fund_receiver_usdc_token_account_pubkey, 
            lottery_auth.pubkey(), 
            TOKEN_STANDARD_PROGRAM_ID
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &lottery_auth
            ],
            ptc.last_blockhash
        );

        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        let error = ptc
            .banks_client
            .get_account(lottery_account_pda.0)
            .await
            .unwrap();
        if error.is_some() {
            panic!("Lottery account must be closed.");
        };

        let error = ptc
            .banks_client
            .get_account(lottery_usdc_ata_pubkey)
            .await
            .unwrap();
        if error.is_some() {
            panic!("Lottery USDC ata account must be closed.");
        };

        let SolanaAccount { data: fund_receiver_usdc_token_account_data, .. } = ptc
            .banks_client
            .get_account(fund_receiver_usdc_token_account_pubkey)
            .await
            .unwrap()
            .unwrap();

        let TokenAccount { amount: fund_receiver_usdc_token_account_balance, .. } = TokenAccount::unpack(
            &fund_receiver_usdc_token_account_data
        ).unwrap();

        assert_eq!(
            fund_receiver_usdc_token_account_balance,
            10_000000,
            "invalid fund receiver usdc token account balance."
        );

        let SolanaAccount { lamports: fund_receiver_rent_exempt_lamports_balance , ..} = ptc
            .banks_client
            .get_account(lottery_auth.pubkey())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            fund_receiver_rent_exempt_lamports_balance,
            sol_to_lamports(1.0) + sol_to_lamports(1.0) + sol_to_lamports(1.0),
            "invalid rent exempt fund receiver lamport balance."
        );
    }
    // success

    // failure - lottery owner must first close Arbitrary ata
    {
        ptc.
            get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 450);

        lottery_account.is_creator_withdrawed_when_lottery_was_failed = false;
        ptc.set_account(
            &lottery_account_pda.0,
            &SolanaSharedDataAccount::from(
                SolanaAccount {
                    owner: LOTTERY_PROGRAM_ID,
                    lamports: sol_to_lamports(1.0),
                    data: vec![
                        lottery_account.try_to_vec().unwrap(),
                        vec![0u8; 330]
                    ].concat(),
                    ..SolanaAccount::default()
                }
            )
        );

        let instruction = instruction_close_lottery_account_and_usdc_token_account(
            config_account_pda.0, 
            lottery_account_pda.0, 
            lottery_auth.pubkey(), 
            config_account.usdc_mint_account, 
            lottery_usdc_ata_pubkey, 
            fund_receiver_usdc_token_account_pubkey, 
            lottery_auth.pubkey(), 
            TOKEN_STANDARD_PROGRAM_ID
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &lottery_auth
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::FirstCloseLotteryArbitrartAssociatedTokenAccount as u32
                )
            )
        );

        // return to default
        lottery_account.is_creator_withdrawed_when_lottery_was_failed = true;
        ptc.set_account(
            &lottery_account_pda.0,
            &SolanaSharedDataAccount::from(
                SolanaAccount {
                    owner: LOTTERY_PROGRAM_ID,
                    lamports: sol_to_lamports(1.0),
                    data: vec![
                        lottery_account.try_to_vec().unwrap(),
                        vec![0u8; 330]
                    ].concat(),
                    ..SolanaAccount::default()
                }
            )
        );
    }
    // failure - lottery owner must first close Arbitrary ata

    // failure - users didn't claim their funds from lottery account
    {
        ptc.
            get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 450);

        ptc.set_account(
            &lottery_account_pda.0,
            &SolanaSharedDataAccount::from(
                SolanaAccount {
                    owner: LOTTERY_PROGRAM_ID,
                    lamports: sol_to_lamports(1.1),
                    data: vec![
                        lottery_account.try_to_vec().unwrap(),
                        vec![0u8; 330],
                        vec![0u8, 96]
                    ].concat(),
                    ..SolanaAccount::default()
                }
            )
        );

        let instruction = instruction_close_lottery_account_and_usdc_token_account(
            config_account_pda.0, 
            lottery_account_pda.0, 
            lottery_auth.pubkey(), 
            config_account.usdc_mint_account, 
            lottery_usdc_ata_pubkey, 
            fund_receiver_usdc_token_account_pubkey, 
            lottery_auth.pubkey(), 
            TOKEN_STANDARD_PROGRAM_ID
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &lottery_auth
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::CannotCloseAccounts as u32
                )
            )
        );
    }
    // failure - users didn't claim their funds from lottery account
}

#[tokio::test]
async fn test_withdraw_and_close_succeed_user() {
    let lottery_program_id = LOTTERY_PROGRAM_ID;
    let mut pt = setup_program_test(lottery_program_id);

    //////////////////////? add config account
    let config_account_pda = Pubkey::find_program_address(
        &[
            b"solottery_program_config_account"
        ],
        &lottery_program_id
    );

    let config_account = Config {
        discriminator: Config::get_discriminator(),
        canonical_bump: config_account_pda.1,
        usdc_mint_account: Pubkey::new_from_array([1; 32]),
        ..Config::default()
    };
    let config_account_data = config_account.try_to_vec().unwrap();

    let config_solana_account = SolanaAccount {
        owner: lottery_program_id,
        data: config_account_data,
        lamports: sol_to_lamports(0.0009),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account_pda.0,
        config_solana_account
    );
    //////////////////////? add config account
    
    //////////////////////? add lottery account
    let lottery_account_pda = Pubkey::find_program_address(
        &[
            b"lottery_account",
            Pubkey::default().to_bytes().as_slice(),
            get_lottery_literal_seed(&String::from("1")).as_slice()
        ],
        &lottery_program_id
    );

    let mut lottery_account = Lottery {
        discriminator: Lottery::get_discriminator(),
        canonical_bump: lottery_account_pda.1,
        starting_time: 100,
        ending_time: 200,
        lottery_description: String::from("1"),
        minimum_tickets_amount_required_to_be_sold: 100,
        tickets_total_amount: 900,
        is_ended_successfuly: true,
        ..Lottery::default()
    };
    lottery_account.initial_bytes = lottery_account.try_to_vec().unwrap().len() as u64;

    let lottey_solana_account = SolanaAccount {
        owner: lottery_program_id,
        lamports: solana_sdk::native_token::sol_to_lamports(1.0),
        data: vec![
            lottery_account.try_to_vec().unwrap(),
            vec![0u8; 320]
        ].concat(),
        ..SolanaAccount::default()
    };

    pt.add_account(
        lottery_account_pda.0,
        lottey_solana_account
    );
    //////////////////////? add lottery account
    
    //////////////////////? add user account
    let user_account_auth = Keypair::new();
    pt.add_account(
        user_account_auth.pubkey(),
        SolanaAccount {
            lamports: sol_to_lamports(1.0),
            ..SolanaAccount::default()
        }
    );

    let user_account_pda = Pubkey::find_program_address(
        &[
            b"user_account",
            user_account_auth.pubkey().to_bytes().as_slice(),
            &lottery_account_pda.0.to_bytes().as_slice()
        ],
        &lottery_program_id
    );

    let user_account = User {
        discriminator: User::get_discriminator(),
        canonical_bump: user_account_pda.1,
        authority: user_account_auth.pubkey(),
        lottery: lottery_account_pda.0,
        total_rent_exempt_paied: 100000,
        total_tickets_acquired: 10,
        total_tickets_value: 100_000000, // USDC
        ..User::default()
    };

    let user_solana_account = SolanaAccount {
        owner: lottery_program_id,
        lamports: sol_to_lamports(1.0),
        data: user_account.try_to_vec().unwrap(),
        ..SolanaAccount::default()
    };
    
    pt.add_account(
        user_account_pda.0,
        user_solana_account
    );
    //////////////////////? add user account
    
    let mut ptc = pt.start_with_context().await;

    // failure - user is one of the winners
    {
        ptc.
            get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 350);

        lottery_account.winners.push(
            (user_account_pda.0, false)
        );
        let lottey_solana_account = SolanaAccount {
            owner: lottery_program_id,
            lamports: solana_sdk::native_token::sol_to_lamports(1.0),
            data: vec![
                lottery_account.try_to_vec().unwrap(),
                vec![0u8; 320]
            ].concat(),
            ..SolanaAccount::default()
        };
    
        ptc.set_account(
            &lottery_account_pda.0,
            &SolanaSharedDataAccount::from(lottey_solana_account)
        );

        let instruction = instruction_withdraw_and_close_succeed_user(
            lottery_account_pda.0, 
            user_account_pda.0, 
            user_account_auth.pubkey(), 
            user_account_auth.pubkey(), 
            user_account_auth.pubkey(),
            config_account_pda.0
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &user_account_auth
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::UserIsOneOfTheWinners as u32
                )
            )
        );

        // return to default
        lottery_account.winners = vec![];
        let lottey_solana_account = SolanaAccount {
            owner: lottery_program_id,
            lamports: solana_sdk::native_token::sol_to_lamports(1.0),
            data: vec![
                lottery_account.try_to_vec().unwrap(),
                vec![0u8; 320]
            ].concat(),
            ..SolanaAccount::default()
        };

        ptc.set_account(
            &lottery_account_pda.0,
            &SolanaSharedDataAccount::from(lottey_solana_account)
        );
    }
    // failure - user is one of the winners

    // success
    {
        ptc.
            get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(&ptc, 350);

        let instruction = instruction_withdraw_and_close_succeed_user(
            lottery_account_pda.0, 
            user_account_pda.0, 
            user_account_auth.pubkey(), 
            user_account_auth.pubkey(), 
            user_account_auth.pubkey(),
            config_account_pda.0
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &user_account_auth
            ],
            ptc.last_blockhash
        );

        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        let error = ptc
            .banks_client
            .get_account(user_account_pda.0)
            .await
            .unwrap();
        if error.is_some() {
            panic!("Account must be closed.");
        };

        let SolanaAccount { lamports: lottery_account_lamport_balance, data, .. } = ptc
            .banks_client
            .get_account(lottery_account_pda.0)
            .await
            .unwrap()
            .unwrap();
        
        assert_eq!(
            lottery_account_lamport_balance,
            sol_to_lamports(1.0) - 100000,
            "invalid lottery account lamport balance."
        );

        assert_eq!(
            data.len(),
            lottery_account.try_to_vec().unwrap().len(),
            "invalid lottery account data size."
        );

        let SolanaAccount { lamports: user_account_auth_lamport_balance, .. } = ptc
            .banks_client
            .get_account(user_account_auth.pubkey())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            user_account_auth_lamport_balance,
            sol_to_lamports(1.0) +
            100000 + // tickets rent exempt lamports
            sol_to_lamports(1.0), // user account's lamport balance
            "invalid user account authority's lamport balance."
        );
    }
    // success
}
////////////////////////////////////// Lottery Instructions

////////////////////////////////////// User Instructions
#[tokio::test]
async fn test_create_and_initialize_user_account() {
    let lottery_program_id = LOTTERY_PROGRAM_ID;
    let mut pt = setup_program_test(lottery_program_id);

    //////////////////////? add config account
    let config_account_pda = Pubkey::find_program_address(
        &[
            b"solottery_program_config_account"
        ],
        &lottery_program_id
    );

    let config_account = Config {
        discriminator: Config::get_discriminator(),
        canonical_bump: config_account_pda.1,
        lottery_creation_fee: 5_000000, // 5 USDC
        maximum_number_of_winners: 10,
        usdc_mint_account: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
        maximum_time_for_lottery_account: 1000,
        minimum_tickets_to_be_sold_in_lottery: 20,
        max_lottery_description_bytes: 300,
        ..Config::default()
    };
    let config_account_data = config_account.try_to_vec().unwrap();

    let config_solana_account = SolanaAccount {
        owner: lottery_program_id,
        data: config_account_data,
        lamports: sol_to_lamports(0.0009),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account_pda.0,
        config_solana_account
    );
    //////////////////////? add config account
    
    //////////////////////? add lottery account
    let lottery_account_pda = Pubkey::find_program_address(
        &[
            b"lottery_account",
            Pubkey::default().to_bytes().as_slice(),
            get_lottery_literal_seed(&String::from("1")).as_slice()
        ],
        &lottery_program_id
    );

    let lottery_account = Lottery {
        discriminator: Lottery::get_discriminator(),
        canonical_bump: lottery_account_pda.1,
        lottery_creation_fee: 5_000000,
        protocol_fee: 300_000000,
        starting_time: 100,
        ending_time: 200,
        lottery_description: String::from("1"),
        ..Lottery::default()
    };

    let lottey_solana_account = SolanaAccount {
        owner: lottery_program_id,
        lamports: solana_sdk::native_token::sol_to_lamports(1.0),
        data: lottery_account.try_to_vec().unwrap(),
        ..SolanaAccount::default()
    };

    pt.add_account(
        lottery_account_pda.0,
        lottey_solana_account
    );
    //////////////////////? add lottery account
    
    let mut ptc = pt.start_with_context().await;

    // failure - invalid lottery state
    {
        change_clock_sysvar(&ptc, 35);
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();

        let user_account_authority = Keypair::new();
        let user_account_pda = Pubkey::find_program_address(
            &[
                b"user_account",
                user_account_authority.pubkey().to_bytes().as_slice(),
                lottery_account_pda.0.to_bytes().as_slice()
            ],
            &lottery_program_id
        );
    
        let instruction = instruction_create_and_initialize_user_account(
            user_account_pda.0,
            user_account_authority.pubkey(),
            ptc.payer.pubkey(), 
            lottery_account_pda.0,
            SYSTEM_PROGRAM_ID, 
            config_account_pda.0
        );
    
        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &user_account_authority
            ], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidLotteryState as u32
                )
            )
        );
    }
    // failure - invalid lottery state

    // failure - invalid seeds 
    {
        change_clock_sysvar(&ptc, 150);
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();

        let user_account_authority = Keypair::new();
        let user_account_pda = Pubkey::find_program_address(
            &[
                b"user_accounT",
                user_account_authority.pubkey().to_bytes().as_slice(),
                lottery_account_pda.0.to_bytes().as_slice()
            ],
            &lottery_program_id
        );
    
        let instruction = instruction_create_and_initialize_user_account(
            user_account_pda.0,
            user_account_authority.pubkey(),
            ptc.payer.pubkey(), 
            lottery_account_pda.0,
            SYSTEM_PROGRAM_ID, 
            config_account_pda.0
        );
    
        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &user_account_authority
            ], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::InvalidSeeds
            )
        );
    }
    // failure - invalid seeds

    // failure - trying to create user_account twice
    {
        change_clock_sysvar(&ptc, 150);
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();

        let user_account_authority = Keypair::new();
        let user_account_pda = Pubkey::find_program_address(
            &[
                b"user_account",
                user_account_authority.pubkey().to_bytes().as_slice(),
                lottery_account_pda.0.to_bytes().as_slice()
            ],
            &lottery_program_id
        );

        let instruction = instruction_create_and_initialize_user_account(
            user_account_pda.0,
            user_account_authority.pubkey(),
            ptc.payer.pubkey(), 
            lottery_account_pda.0,
            SYSTEM_PROGRAM_ID, 
            config_account_pda.0
        );

        let tx = Transaction::new_signed_with_payer(
            &[ 
                instruction.clone(),
                instruction
            ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &user_account_authority
            ], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                1,
                InstructionError::Custom(
                    LotteryError::AccountMustBeRaw as u32
                )
                // InstructionError::Custom(
                //     solana_sdk::system_instruction::SystemError::AccountAlreadyInUse as u32
                // )
            )
        );
    }
    // failure - trying to create user_account twice

    // success
    {
        change_clock_sysvar(&ptc, 150);
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();

        let user_account_authority = Keypair::new();
        let user_account_pda = Pubkey::find_program_address(
            &[
                b"user_account",
                user_account_authority.pubkey().to_bytes().as_slice(),
                lottery_account_pda.0.to_bytes().as_slice()
            ],
            &lottery_program_id
        );

        let instruction = instruction_create_and_initialize_user_account(
            user_account_pda.0,
            user_account_authority.pubkey(),
            ptc.payer.pubkey(), 
            lottery_account_pda.0,
            SYSTEM_PROGRAM_ID, 
            config_account_pda.0
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &user_account_authority
            ], 
            ptc.last_blockhash
        );

        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        let SolanaAccount { data: user_account_data, owner: user_account_owner, .. } = ptc
            .banks_client
            .get_account(user_account_pda.0)
            .await.unwrap().unwrap();

        assert_eq!(
            user_account_owner,
            LOTTERY_PROGRAM_ID,
            "invalid user_account's owner."
        );

        assert_eq!(
            User::LEN,
            user_account_data.len(),
            "invalid user_account's data len."
        );

        let User { authority, lottery, created_at, .. } = User::deserialize(
            &mut &user_account_data[..]
        ).unwrap();

        assert_eq!(
            authority,
            user_account_authority.pubkey(),
            "invalid user_account's authority."
        );

        assert_eq!(
            lottery,
            lottery_account_pda.0,
            "invalid user_account's lottery account."
        );

        assert_eq!(
            created_at,
            150,
            "invalid created_at timestamp."
        );
    }
    // success
}

#[tokio::test]
async fn test_buy_tickets() {
    let lottery_program_id = LOTTERY_PROGRAM_ID;
    let mut pt = setup_program_test(lottery_program_id);

    //////////////////////? add config account
    let config_account_pda = Pubkey::find_program_address(
        &[
            b"solottery_program_config_account"
        ],
        &lottery_program_id
    );

    let config_account = Config {
        discriminator: Config::get_discriminator(),
        canonical_bump: config_account_pda.1,
        lottery_creation_fee: 5_000000, // 5 USDC
        maximum_number_of_winners: 10,
        usdc_mint_account: Pubkey::new_from_array([1; 32]),
        maximum_time_for_lottery_account: 1000,
        minimum_tickets_to_be_sold_in_lottery: 20,
        max_lottery_description_bytes: 300,
        lottery_tickets_fee: 2.5,
        ..Config::default()
    };
    let config_account_data = config_account.try_to_vec().unwrap();

    let config_solana_account = SolanaAccount {
        owner: lottery_program_id,
        data: config_account_data,
        lamports: sol_to_lamports(0.0009),
        ..SolanaAccount::default()
    };

    pt.add_account(
        config_account_pda.0,
        config_solana_account
    );
    //////////////////////? add config account
    
    //////////////////////? add lottery account
    let lottery_account_pda = Pubkey::find_program_address(
        &[
            b"lottery_account",
            Pubkey::default().to_bytes().as_slice(),
            get_lottery_literal_seed(&String::from("1")).as_slice()
        ],
        &lottery_program_id
    );

    let mut lottery_account = Lottery {
        discriminator: Lottery::get_discriminator(),
        canonical_bump: lottery_account_pda.1,
        starting_time: 100,
        ending_time: 200,
        lottery_description: String::from("1"),
        ticket_price: 10_000000,
        maximum_number_of_tickets_per_user: Some(100),
        ..Lottery::default()
    };
    lottery_account.initial_bytes = lottery_account.try_to_vec().unwrap().len() as u64;

    let lottey_solana_account = SolanaAccount {
        owner: lottery_program_id,
        lamports: solana_sdk::native_token::sol_to_lamports(1.0),
        data: lottery_account.try_to_vec().unwrap(),
        ..SolanaAccount::default()
    };

    pt.add_account(
        lottery_account_pda.0,
        lottey_solana_account
    );
    //////////////////////? add lottery account
    
    //////////////////////? add user account
    let user_account_auth = Keypair::new();
    pt.add_account(
        user_account_auth.pubkey(),
        SolanaAccount {
            lamports: sol_to_lamports(1.0),
            ..SolanaAccount::default()
        }
    );

    let user_account_pda = Pubkey::find_program_address(
        &[
            b"user_account",
            user_account_auth.pubkey().to_bytes().as_slice(),
            &lottery_account_pda.0.to_bytes().as_slice()
        ],
        &lottery_program_id
    );

    let user_account = User {
        discriminator: User::get_discriminator(),
        canonical_bump: user_account_pda.1,
        authority: user_account_auth.pubkey(),
        lottery: lottery_account_pda.0,
        ..User::default()
    };

    let user_solana_account = SolanaAccount {
        owner: lottery_program_id,
        lamports: sol_to_lamports(1.0),
        data: user_account.try_to_vec().unwrap(),
        ..SolanaAccount::default()
    };
    
    pt.add_account(
        user_account_pda.0,
        user_solana_account
    );
    //////////////////////? add user account
    
    //////////////////////? add mint account
    let mint_account_pubkey = Pubkey::new_from_array([1; 32]);
    let mint_account = MintAccount {
        supply: 100_000000,
        decimals: 6,
        is_initialized: true,
        ..MintAccount::default()
    };

    let mut mint_account_data = [0u8; MintAccount::LEN];
    MintAccount::pack(
        mint_account,
        mint_account_data.as_mut_slice()
    ).unwrap();

    let mint_solana_account = SolanaAccount {
        owner: spl_token::ID,
        data: mint_account_data.to_vec(),
        lamports: sol_to_lamports(1.0),
        ..SolanaAccount::default()
    };

    pt.add_account(
        mint_account_pubkey,
        mint_solana_account
    );
    //////////////////////? add mint account
    
    //////////////////////? add lottery ata
    let lottery_ata_pda = get_associated_token_address(
        &lottery_account_pda.0,
        &mint_account_pubkey
    );
    let lottery_ata = TokenAccount {
        state: TokenAccountState::Initialized,
        mint: mint_account_pubkey,
        ..TokenAccount::default()
    };

    let mut lottery_ata_data = [0u8; TokenAccount::LEN];
    TokenAccount::pack(
        lottery_ata,
        lottery_ata_data.as_mut_slice()
    ).unwrap();

    let lottery_ata_solana_account = SolanaAccount {
        owner: spl_token::ID,
        data: lottery_ata_data.to_vec(),
        lamports: sol_to_lamports(1.0),
        ..SolanaAccount::default()
    };

    pt.add_account(
        lottery_ata_pda,
        lottery_ata_solana_account
    );
    //////////////////////? add lottery ata
    
    //////////////////////? add funding token account
    let funding_token_account_pubeky = Pubkey::new_from_array([2; 32]);
    let funding_token_account = TokenAccount {
        state: TokenAccountState::Initialized,
        mint: mint_account_pubkey,
        owner: user_account_auth.pubkey(),
        amount: 100_000000,
        ..TokenAccount::default()
    };

    let mut funding_token_account_data = [0u8; TokenAccount::LEN];
    TokenAccount::pack(
        funding_token_account,
        funding_token_account_data.as_mut_slice()
    ).unwrap();

    let funding_token_solana_account = SolanaAccount {
        owner: spl_token::ID,
        data: funding_token_account_data.to_vec(),
        lamports: sol_to_lamports(1.0),
        ..SolanaAccount::default()
    };

    pt.add_account(
        funding_token_account_pubeky,
        funding_token_solana_account
    );
    //////////////////////? add funding token account
    
    //////////////////////? add unknown lottery account
    let unknown_lottery_account_pda = Pubkey::find_program_address(
        &[
            b"lottery_account",
            Pubkey::default().to_bytes().as_slice(),
            get_lottery_literal_seed(&String::from("2")).as_slice()
        ],
        &lottery_program_id
    );
    
    let mut unkown_lottery_account = Lottery {
        discriminator: Lottery::get_discriminator(),
        canonical_bump: unknown_lottery_account_pda.1,
        starting_time: 100,
        ending_time: 200,
        lottery_description: String::from("2"),
        ticket_price: 10_000000,
        ..Lottery::default()
    };
    unkown_lottery_account.initial_bytes = unkown_lottery_account.try_to_vec().unwrap().len() as u64;
    
    let lottey_solana_account = SolanaAccount {
        owner: lottery_program_id,
        lamports: solana_sdk::native_token::sol_to_lamports(1.0),
        data: unkown_lottery_account.try_to_vec().unwrap(),
        ..SolanaAccount::default()
    };
    
    pt.add_account(
        unknown_lottery_account_pda.0,
        lottey_solana_account
    );
    //////////////////////? add unknown lottery account

    //////////////////////? add unkown lottery ata
    let unkown_lottery_ata_pda = get_associated_token_address(
        &unknown_lottery_account_pda.0,
        &mint_account_pubkey
    );
    let unkown_lottery_ata = TokenAccount {
        state: TokenAccountState::Initialized,
        mint: mint_account_pubkey,
        ..TokenAccount::default()
    };

    let mut unkown_lottery_ata_data = [0u8; TokenAccount::LEN];
    TokenAccount::pack(
        unkown_lottery_ata,
        unkown_lottery_ata_data.as_mut_slice()
    ).unwrap();

    let unkown_lottery_ata_solana_account = SolanaAccount {
        owner: spl_token::ID,
        data: lottery_ata_data.to_vec(),
        lamports: sol_to_lamports(1.0),
        ..SolanaAccount::default()
    };

    pt.add_account(
        unkown_lottery_ata_pda,
        unkown_lottery_ata_solana_account
    );
    //////////////////////? add unkown lottery ata
    
    let mut ptc = pt.start_with_context().await;

    // failure - invalid user's tickets amount
    {
        change_clock_sysvar(
            &ptc, 
            150
        );
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();

        let instruction_1 = instruction_buy_ticket(
            config_account_pda.0, 
            user_account_pda.0, 
            user_account_auth.pubkey(), 
            user_account_auth.pubkey(), 
            lottery_account_pda.0, 
            lottery_ata_pda, 
            funding_token_account_pubeky, 
            mint_account_pubkey, 
            SYSTEM_PROGRAM_ID, 
            spl_token::ID, 
            105,
            10_000000 // 10 USDC
        );

        let instruction_2 = instruction_buy_ticket(
            config_account_pda.0, 
            user_account_pda.0, 
            user_account_auth.pubkey(), 
            user_account_auth.pubkey(), 
            lottery_account_pda.0, 
            lottery_ata_pda, 
            funding_token_account_pubeky, 
            mint_account_pubkey, 
            SYSTEM_PROGRAM_ID, 
            spl_token::ID, 
            5,
            10_000000
        );

        let instruction_3 = instruction_buy_ticket(
            config_account_pda.0, 
            user_account_pda.0, 
            user_account_auth.pubkey(), 
            user_account_auth.pubkey(), 
            lottery_account_pda.0, 
            lottery_ata_pda, 
            funding_token_account_pubeky, 
            mint_account_pubkey, 
            SYSTEM_PROGRAM_ID, 
            spl_token::ID, 
            98,
            10_000000
        );

        let tx_1 = Transaction::new_signed_with_payer(
            &[
                instruction_1
            ], 
            Some(&ptc.payer.pubkey()), 
            &[
                &ptc.payer,
               &user_account_auth
            ], 
            ptc.last_blockhash
        );

        let tx_2 = Transaction::new_signed_with_payer(
            &[
                instruction_2,
                instruction_3
            ], 
            Some(&ptc.payer.pubkey()), 
            &[
                &ptc.payer,
               &user_account_auth
            ], 
            ptc.last_blockhash
        );

        let error_tx1 = ptc
            .banks_client
            .process_transaction(tx_1)
            .await
            .unwrap_err()
            .unwrap();

        let error_tx2 = ptc
            .banks_client
            .process_transaction(tx_2)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error_tx1,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::MaxTicketsAmountViolated as u32
                )
            )
        );

        assert_eq!(
            error_tx2,
            TransactionError::InstructionError(
                1,
                InstructionError::Custom(
                    LotteryError::MaxTicketsAmountViolated as u32
                )
            )
        );
    }
    // failure - invalid user's tickets amount

    // failure - invalid tickets amount
    {
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(
            &ptc, 
            150
        );

        let instruction = instruction_buy_ticket(
            config_account_pda.0, 
            user_account_pda.0, 
            user_account_auth.pubkey(), 
            user_account_auth.pubkey(), 
            lottery_account_pda.0, 
            lottery_ata_pda, 
            funding_token_account_pubeky, 
            mint_account_pubkey, 
            SYSTEM_PROGRAM_ID, 
            spl_token::ID, 
            0,
            10_000000
        );

        let tx = Transaction::new_signed_with_payer(
            &[
                instruction.clone(),
                instruction
            ], 
            Some(&ptc.payer.pubkey()), 
            &[
                &ptc.payer,
                &user_account_auth
            ], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidTicketAmount as u32
                )
            )
        );
    }
    // failure - invalid tickets amount

    // failure - lottery is in invalid state
    {
        // 1. lottery is not started yet
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(
            &ptc, 
            50
        );

        let instruction = instruction_buy_ticket(
            config_account_pda.0, 
            user_account_pda.0, 
            user_account_auth.pubkey(), 
            user_account_auth.pubkey(), 
            lottery_account_pda.0, 
            lottery_ata_pda, 
            funding_token_account_pubeky, 
            mint_account_pubkey, 
            SYSTEM_PROGRAM_ID, 
            spl_token::ID, 
            5,
            10_000000
        );

        let tx = Transaction::new_signed_with_payer(
            &[
                instruction.clone(),
                instruction
            ], 
            Some(&ptc.payer.pubkey()), 
            &[
                &ptc.payer,
                &user_account_auth
            ], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidLotteryState as u32
                )
            )
        );

        // 2. lottery is ended
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(
            &ptc, 
            350
        );

        let instruction = instruction_buy_ticket(
            config_account_pda.0, 
            user_account_pda.0, 
            user_account_auth.pubkey(), 
            user_account_auth.pubkey(), 
            lottery_account_pda.0, 
            lottery_ata_pda, 
            funding_token_account_pubeky, 
            mint_account_pubkey, 
            SYSTEM_PROGRAM_ID, 
            spl_token::ID, 
            5,
            10_000000
        );

        let tx = Transaction::new_signed_with_payer(
            &[
                instruction.clone(),
                instruction
            ], 
            Some(&ptc.payer.pubkey()), 
            &[
                &ptc.payer,
                &user_account_auth
            ], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::InvalidLotteryState as u32
                )
            )
        );
    }
    // failure - lottery is in invalid state

    // failure - invalid user account
    {
        // 1. invalid user_account's authority
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(
            &ptc, 
            150
        );

        let unknown_account = Keypair::new();
        let instruction = instruction_buy_ticket(
            config_account_pda.0, 
            user_account_pda.0, 
            unknown_account.pubkey(), 
            unknown_account.pubkey(), 
            lottery_account_pda.0, 
            lottery_ata_pda, 
            funding_token_account_pubeky, 
            mint_account_pubkey, 
            SYSTEM_PROGRAM_ID, 
            spl_token::ID, 
            5,
            10_000000
        );

        let tx = Transaction::new_signed_with_payer(
            &[
                instruction.clone(),
                instruction
            ], 
            Some(&ptc.payer.pubkey()), 
            &[
                &ptc.payer,
                &unknown_account
            ], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        if !(
            error == TransactionError::InstructionError(0, InstructionError::InvalidSeeds) ||
            error == TransactionError::InstructionError(0, InstructionError::Custom(LotteryError::FailedToFindProgramAddress as u32))
        ) {
            panic!("1. invalidSeeds Or failedToFindProgramAddress");
        };

        // 2. invalid lottery_account
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();
        change_clock_sysvar(
            &ptc, 
            150
        );

        let instruction = instruction_buy_ticket(
            config_account_pda.0, 
            user_account_pda.0, 
            user_account_auth.pubkey(), 
            user_account_auth.pubkey(), 
            unknown_lottery_account_pda.0, 
            unkown_lottery_ata_pda, 
            funding_token_account_pubeky, 
            mint_account_pubkey, 
            SYSTEM_PROGRAM_ID, 
            spl_token::ID, 
            5,
            10_000000
        );

        let tx = Transaction::new_signed_with_payer(
            &[
                instruction.clone(),
                instruction
            ], 
            Some(&ptc.payer.pubkey()), 
            &[
                &ptc.payer,
                &user_account_auth
            ], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        if !(
            error == TransactionError::InstructionError(0, InstructionError::InvalidSeeds) ||
            error == TransactionError::InstructionError(0, InstructionError::Custom(LotteryError::FailedToFindProgramAddress as u32))
        ) {
            panic!("2. invalidSeeds Or failedToFindProgramAddress");
        };
    }
    // failure - invalid user account

    // failure - expected ticket price is violated
    {
        change_clock_sysvar(
            &ptc, 
            150
        );
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();

        let instruction = instruction_buy_ticket(
            config_account_pda.0, 
            user_account_pda.0, 
            user_account_auth.pubkey(), 
            user_account_auth.pubkey(), 
            lottery_account_pda.0, 
            lottery_ata_pda, 
            funding_token_account_pubeky, 
            mint_account_pubkey, 
            SYSTEM_PROGRAM_ID, 
            spl_token::ID, 
            5,
            5_000000
        );

        let tx = Transaction::new_signed_with_payer(
            &[ instruction ], 
            Some(&ptc.payer.pubkey()), 
            &[
                &ptc.payer,
               &user_account_auth
            ], 
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    LotteryError::ExpectedTicketPriceViolated as u32
                )
            )
        );
    }
    // failure - expected ticket price is violated
    
    // success
    {
        change_clock_sysvar(
            &ptc, 
            150
        );
        ptc
            .get_new_latest_blockhash()
            .await
            .unwrap();

        let instruction = instruction_buy_ticket(
            config_account_pda.0, 
            user_account_pda.0, 
            user_account_auth.pubkey(), 
            user_account_auth.pubkey(), 
            lottery_account_pda.0, 
            lottery_ata_pda, 
            funding_token_account_pubeky, 
            mint_account_pubkey, 
            SYSTEM_PROGRAM_ID, 
            spl_token::ID, 
            5,
            10_000000
        );

        let tx = Transaction::new_signed_with_payer(
            &[
                instruction.clone(),
                instruction
            ], 
            Some(&ptc.payer.pubkey()), 
            &[
                &ptc.payer,
               &user_account_auth
            ], 
            ptc.last_blockhash
        );

        ptc
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        let SolanaAccount { data: lottery_account_data, .. } = ptc
            .banks_client
            .get_account(lottery_account_pda.0)
            .await
            .unwrap()
            .unwrap();

        let SolanaAccount { data: user_account_data, .. } = ptc
            .banks_client
            .get_account(user_account_pda.0)
            .await
            .unwrap()
            .unwrap();

        let SolanaAccount { data: funding_token_account_data, .. } = ptc
            .banks_client
            .get_account(funding_token_account_pubeky)
            .await
            .unwrap()
            .unwrap();

        let SolanaAccount { data: lottery_ata_data, .. } = ptc
            .banks_client
            .get_account(lottery_ata_pda)
            .await
            .unwrap()
            .unwrap();

        let lottery_account = Lottery::deserialize(
            &mut &lottery_account_data[..]
        ).unwrap();

        let user_account = User::deserialize(
            &mut &user_account_data[..]
        ).unwrap();

        let funding_token_account = TokenAccount::unpack(
            &funding_token_account_data
        ).unwrap();

        let lottery_ata = TokenAccount::unpack(
            &lottery_ata_data
        ).unwrap();

        // check lottery account
        assert_eq!(
            lottery_account.tickets_total_amount,
            10,
            "invalid tickets total amounts."
        );

        assert_eq!(
            lottery_account.protocol_fee,
            2_500000,
            "invalid protocol fee amounts."
        );

        assert_eq!(
            lottery_account_data.len(),
            lottery_account.initial_bytes as usize + (10 * size_of::<Pubkey>()),
            "invalid lottery_account's data length."
        );

        let mut lottery_account_d = lottery_account_data.clone();
        let mut lottery_account_balance = 1_000000000;
        let lottery_account_info = &solana_sdk::account_info::AccountInfo {
            lamports: Rc::new(RefCell::new(&mut lottery_account_balance)),
            owner: &lottery_program_id,
            key: &Pubkey::new_unique(),
            data: Rc::new(RefCell::new(&mut lottery_account_d)),
            rent_epoch: solana_sdk::clock::Epoch::default(),
            is_signer: false,
            is_writable: false,
            executable: false
        };
        for index in 0..10 {
            assert_eq!(
                Lottery::get_ticket(
                    lottery_account_info,
                    index
                ).unwrap(),
                user_account_pda.0,
                "invalid user_account's ticket."
            );
        };

        let total_tickets_rent_exempt = solana_sdk::rent::DEFAULT_EXEMPTION_THRESHOLD as u64 * 
            ( solana_sdk::rent::DEFAULT_LAMPORTS_PER_BYTE_YEAR * ( 10 * size_of::<Pubkey>() as u64 ) );

        let lottery_account_balance = sol_to_lamports(1.0) + total_tickets_rent_exempt;

        assert_eq!(
            lottery_account_balance,
            ptc.banks_client.get_balance(lottery_account_pda.0).await.unwrap(),
            "invalid lottery_account's lamport balance."
        );
        // check lottery account

        // check user account
        assert_eq!(
            user_account.total_rent_exempt_paied,
            total_tickets_rent_exempt,
            "invalid user_account's rent_exempt paied."
        );

        assert_eq!(
            user_account.total_tickets_value,
            10 * 10_000000,
            "invalid user_account's total_tickets_value."
        );

        assert_eq!(
            user_account.total_tickets_acquired,
            10,
            "invalid user_account's total_tickets_acquired."
        );
        // check user account

        // check funding token account balance
        assert_eq!(
            funding_token_account.amount,
            0_000000,
            "invalid funding token account balance."
        );
        // check funding token account balance

        // check lottery's ata balance
        assert_eq!(
            lottery_ata.amount,
            100_000000,
            "invalid lottery's ata token balance."
        );
        // check lottery's ata balance
    }
    // success
}
////////////////////////////////////// User Instructions