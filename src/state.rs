use {
    crate::{
        error::LotteryError,
        types::*
    },

    borsh::{
        BorshDeserialize,
        BorshSerialize
    },

    pyth_solana_receiver_sdk::ID_CONST as PYTH_PULL_ORACLE_RECEIVER_PROGRAM_ID,

    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        hash::{
            hash,
            HASH_BYTES
        },
        log::sol_log,
        msg,
        program_error::ProgramError, 
        program_memory::{
            sol_memcmp, 
            sol_memcpy, 
            sol_memset
        }, 
        pubkey::Pubkey,
    }, 
    
    std::mem::size_of
};

// Discriminators
pub(crate) const CONFIG_ACCOUNT_DISCRIMINATOR: &str = "account:Config";
pub(crate) const LOTTERY_ACCOUNT_DISCRIMINATOR: &str = "account:Lottery";
pub(crate) const USER_ACCOUNT_DISCRIMINATOR: &str = "account:User";
pub(crate) const CLOSED_USER_ACCOUNT_DISCRIMINATOR: &str = "CLOSED_USER_ACCOUNT";
pub(crate) const CLOSED_LOTTERY_ACCOUNT_DISCRIMINATOR: &str = "CLOSED_LOTTERY_ACCOUNT";

// Discriminator Length
pub(crate) const DISCRIMINATOR_LENTGH: usize = 8;
// Canonical_Bump Length
pub(crate) const CANONICAL_BUMP_LENGTH: usize = 1;

// Literal_Seeds
pub(crate) const CONFIG_ACCOUNT_SEED: &str = "solottery_program_config_account";
pub(crate) const LOTTERY_ACCOUNT_SEED: &str = "lottery_account";
pub(crate) const USER_ACCOUNT_SEED: &str = "user_account";

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, PartialEq, Default)]
pub struct Config {
    pub discriminator: [u8; DISCRIMINATOR_LENTGH],
    pub canonical_bump: u8,
    //? In the future, if we discover any vulnerability in the code_base we will pause the protocol to securely fix that.
    //? This flag is here to protect the users.
    pub is_pause: bool,
    pub authority: Pubkey,
    pub latest_update_time: Time,
    pub lottery_creation_fee: u64, // USDC
    pub lottery_tickets_fee: f64, // % //! But using "bps(u16)" is recommended
    pub maximum_number_of_winners: u8,
    pub pyth_price_receiver_programid: Pubkey,
    pub usdc_mint_account: Pubkey,
    pub maximum_time_of_price_feed_age: u8,
    pub minimum_tickets_to_be_sold_in_lottery: u8,
    pub pyth_price_feed_accounts: [PriceFeedAccount; 3], // SOL, BTC, ETH
    pub maximum_time_for_lottery_account: u32, // in seconds
    pub treasury: Pubkey, // USDC token account
    pub max_lottery_description_bytes: u64,
    pub pyth_price_feed_ids: [String; 3] // SOL, BTC, ETH
}
impl Config {
    pub const LEN: usize =
        DISCRIMINATOR_LENTGH +
        CANONICAL_BUMP_LENGTH +
        size_of::<bool>() +
        size_of::<Pubkey>() +
        size_of::<Time>() +
        size_of::<u64>() +
        size_of::<f64>() +
        size_of::<u8>() +
        size_of::<Pubkey>() +
        size_of::<Pubkey>() +
        size_of::<u8>() +
        size_of::<u8>() +
        size_of::<[PriceFeedAccount; 3]>() +
        size_of::<u32>() +
        size_of::<Pubkey>() +
        size_of::<u64>() +
        (3 * 70);

    pub fn new(
        canonical_bump: u8,
        authority: Pubkey,
        lottery_creation_fee: u64, // USDC
        lottery_tickets_fee: f64, // %
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
    ) -> Result<Self, ProgramError> {
        Ok(Self {
            discriminator: Self::get_discriminator(),
            canonical_bump,
            is_pause: bool::default(),
            authority,
            latest_update_time: i64::default(),
            lottery_creation_fee,
            lottery_tickets_fee,
            maximum_number_of_winners,
            pyth_price_feed_accounts,
            usdc_mint_account,
            maximum_time_of_price_feed_age,
            minimum_tickets_to_be_sold_in_lottery,
            pyth_price_receiver_programid,
            maximum_time_for_lottery_account,
            treasury,
            pyth_price_feed_ids,
            max_lottery_description_bytes
        })
    }

    pub fn check_is_pause(&self) -> ProgramResult {
        if self.is_pause == true {
            return Err(
                LotteryError::ProtocolIsPaused.into()
            );
        };

        Ok(())
    }

    pub fn check_is_pause_raw(config_account_info: &AccountInfo) -> ProgramResult {
        let data = config_account_info
            .data
            .try_borrow()
            .unwrap();

        let flag = data.get(
            DISCRIMINATOR_LENTGH + CANONICAL_BUMP_LENGTH
        ).unwrap();
        
        if flag == &1 {
            return Err(
                LotteryError::ProtocolIsPaused.into()
            );
        };

        Ok(())
    }

    pub fn validate_config_account(
        config_account_info: &AccountInfo,
        program_id: &Pubkey
    ) -> ProgramResult {
        if config_account_info.owner != program_id {
            return Err(
                ProgramError::IncorrectProgramId
            );
        };

        if sol_memcmp(
            &config_account_info
                .data
                .try_borrow()
                .unwrap(),
            &Self::get_discriminator(),
            DISCRIMINATOR_LENTGH
        ) != 0 {
            return Err(
                LotteryError::InvalidDiscriminator.into()
            );
        };

        let config_pda_addr = Pubkey::create_program_address(
            &[
                CONFIG_ACCOUNT_SEED.as_bytes(),
                &[
                    *config_account_info
                        .try_borrow_data()
                        .unwrap()
                        .get(8)
                        .unwrap()
                ]
            ],
            program_id
        ).map_err::<ProgramError, _>(|_| LotteryError::FailedToFindProgramAddress.into())?;
        
        if *config_account_info.key != config_pda_addr {
            return Err(
                LotteryError::InvalidConfigAccount.into()
            );
        };

        Ok(())
    }

    pub fn validate_price_feed_accounts(
        &self,
        sol_price_feed_account_info: &AccountInfo,
        btc_price_feed_account_info: &AccountInfo,
        eth_price_feed_account_info: &AccountInfo
    ) -> ProgramResult {
        if !(
            sol_price_feed_account_info.owner == &PYTH_PULL_ORACLE_RECEIVER_PROGRAM_ID &&
            btc_price_feed_account_info.owner == &PYTH_PULL_ORACLE_RECEIVER_PROGRAM_ID &&
            eth_price_feed_account_info.owner == &PYTH_PULL_ORACLE_RECEIVER_PROGRAM_ID
        ) {
            return Err(
                LotteryError::InvalidPriceFeedAccountsOwner.into()
            );
        };

        if sol_price_feed_account_info.key != self.pyth_price_feed_accounts.get(0).unwrap() {
            return Err(
                LotteryError::InvalidSolPriceFeedAccount.into()
            );
        } else if btc_price_feed_account_info.key != self.pyth_price_feed_accounts.get(1).unwrap() {
            return Err(
                LotteryError::InvalidBtcPriceFeedAccount.into()
            ); 
        } else if eth_price_feed_account_info.key != self.pyth_price_feed_accounts.get(2).unwrap() {
            return Err(
                LotteryError::InvalidEthPriceFeedAccount.into()
            );   
        };

        Ok(())
    }

    pub fn validate_fee_per_ticket(fee: &f64) -> ProgramResult {
        if fee >= &100_f64 {
            return Err(
                LotteryError::InvalidLotteryTicketsFee.into()
            );
        };

        Ok(())
    }

    pub fn get_sol_price_feed_id(&self) -> String {
        self.pyth_price_feed_ids[0].clone()
    }

    pub fn get_btc_price_feed_id(&self) -> String {
        self.pyth_price_feed_ids[1].clone()
    }

    pub fn get_eth_price_feed_id(&self) -> String {
        self.pyth_price_feed_ids[2].clone()
    }

    pub fn get_discriminator() -> [u8; DISCRIMINATOR_LENTGH] {
        hash(CONFIG_ACCOUNT_DISCRIMINATOR.as_bytes())
            .to_bytes()
            .get(..8)
            .and_then(|slice| slice.try_into().ok())
            .map(|dis: [u8; 8]| dis)
            .unwrap()
    }
}

// Maximum number of tickets that can be added to a lottery in a single instruction
const MAX_TICKETS_PER_INSTRUCTION: usize = 300;

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, PartialEq, Eq, Default)]
pub struct Lottery {
    pub discriminator: [u8; DISCRIMINATOR_LENTGH],
    pub canonical_bump: u8,
    pub initial_bytes: u64,
    pub authority: Pubkey,
    pub fund_amount: u64,
    pub arbitrary_mint_account_address: Pubkey,
    pub ticket_price: u64, // USDC
    pub lottery_creation_fee: u64, // USDC
    pub winners_count: u8, // Note: fund_amount % winners_count == 0
    pub minimum_tickets_amount_required_to_be_sold: u32,
    pub created_at: Time,
    pub starting_time: Time,
    pub ending_time: Time,
    pub protocol_fee: u64,
    pub is_creator_withdrawed_when_lottery_was_successful: bool,
    pub is_creator_withdrawed_when_lottery_was_failed: bool,
    pub is_ended_successfuly: bool,
    pub is_protocol_fee_claimed: bool,
    pub random_numbers_info: RandomNumberInfo,
    pub tickets_total_amount: u32,
    pub maximum_number_of_tickets_per_user: Option<u32>,
    pub lottery_description: String,
    pub winners: Vec<WinnerStatus>
    // tickets (user's pda-account pubkey) - zero_copy
}

#[derive(Debug, BorshDeserialize, BorshSerialize, PartialEq, Eq, Clone, Copy)]
pub enum LotteryState {
    Unknown,
    Successful,
    Failed,
    Invalid
}

impl Lottery {
    pub fn new(
        canonical_bump: u8,
        fund_amount: u64,
        lottery_creation_fee: u64,
        winners_count: u8,
        starting_time: i64,
        ending_time: i64,
        created_at: i64,
        minimum_tickets_amount_required_to_be_sold: u32,
        ticket_price: u64,
        arbitrary_mint_account_address: Pubkey,
        authority: Pubkey,
        maximum_number_of_tickets_per_user: Option<u32>,
        lottery_description: String
    ) -> Self {
        Self {
            discriminator: Self::get_discriminator(),
            canonical_bump,
            fund_amount,
            initial_bytes: u64::default(),
            arbitrary_mint_account_address,
            lottery_creation_fee,
            winners_count,
            starting_time,
            ending_time,
            created_at,
            minimum_tickets_amount_required_to_be_sold,
            ticket_price,
            authority,
            is_ended_successfuly: bool::default(),
            protocol_fee: u64::default(),
            is_creator_withdrawed_when_lottery_was_successful: bool::default(),
            is_creator_withdrawed_when_lottery_was_failed: bool::default(),
            maximum_number_of_tickets_per_user,
            lottery_description,
            random_numbers_info: (Pubkey::default(), i64::default(), i64::default()),
            is_protocol_fee_claimed: bool::default(),
            tickets_total_amount: u32::default(),
            winners: Vec::default()
        }
    }

    pub fn validate_lottery_account(
        lottery_account_info: &AccountInfo,
        program_id: &Pubkey
    ) -> ProgramResult {
        if lottery_account_info.owner != program_id {
            return Err(
                ProgramError::IncorrectProgramId
            );
        };

        if sol_memcmp(
            &lottery_account_info
                .data
                .try_borrow()
                .unwrap(),
            &Self::get_discriminator(),
            DISCRIMINATOR_LENTGH
        ) != 0 {
            return Err(
                LotteryError::InvalidDiscriminator.into()
            );
        };

        Ok(())
    }

    pub fn is_not_started(
        &self,
        current_time: Time
    ) -> bool {
        current_time < self.starting_time
    }

    pub fn is_started_and_not_ended(
        &self,
        current_time: Time
    ) -> bool {
        current_time >= self.starting_time && current_time < self.ending_time
    }

    pub fn is_ended(
        &self,
        current_time: Time
    ) -> bool {
        current_time >= self.ending_time
    }

    pub fn add_ticket(
        lottery_account_info: &AccountInfo,
        initial_bytes: u64,
        tickets_total_amount: u32,
        tickets_amount: u32,
        user_account_pda: Pubkey
    ) {
        let mut lottery_account_data = lottery_account_info
            .data
            .try_borrow_mut()
            .unwrap();

        let mut current_offset = initial_bytes.checked_add(
            (tickets_total_amount as usize).checked_mul(
                size_of::<Pubkey>()
            ).unwrap() as u64
        ).unwrap() as usize;

        let ticket_pubkey = &user_account_pda.to_bytes();

        for _ in 1..=tickets_amount {
            let from_offset = current_offset;
            let to_offset = current_offset.checked_add(
                size_of::<Pubkey>()
            ).unwrap();

            // add the ticket to the lottery account
            sol_memcpy(
                lottery_account_data
                    .get_mut(from_offset..to_offset)
                    .unwrap(),
                ticket_pubkey,
                size_of::<Pubkey>()
            );

            current_offset = current_offset.checked_add(
                size_of::<Pubkey>()
            ).unwrap();
        };

        let new_total_tickets_amount = tickets_total_amount.checked_add(
            tickets_amount
        ).unwrap();

        // update tickets_total_amount field
        sol_memcpy(
            lottery_account_data
                .get_mut(194..198)
                .unwrap(),
            new_total_tickets_amount.to_le_bytes().as_slice(),
            size_of::<u32>()
        );
    }

    pub fn get_lottery_state(
        &self,
        current_time: Time
    ) -> LotteryState {
        if Self::is_not_started(&self, current_time) == true {
            return LotteryState::Unknown;
        };

        if Self::is_started_and_not_ended(&self, current_time) == true {
            return LotteryState::Unknown;
        };

        if Self::is_ended(&self, current_time) == false {
            return LotteryState::Invalid;
        };

        if self.tickets_total_amount as usize >= (self.minimum_tickets_amount_required_to_be_sold as usize) {
            return LotteryState::Successful;
        } else {
            return LotteryState::Failed;
        };
    }

    pub fn pick_winners(
        &mut self,
        sha256_hash: &[u8; HASH_BYTES],
        lottery_account_info: &AccountInfo
    ) -> ProgramResult {
        let total_tickets = self.tickets_total_amount as usize;
        let mut random_numbers: Vec<u32> = Vec::with_capacity(HASH_BYTES);
        for random_number in sha256_hash.iter() {
            random_numbers.push(
                ((*random_number as usize) % total_tickets)
                    .try_into()
                    .unwrap()
            );
        };

        let mut random_numbers_without_duplication: Vec<u32> = Vec::with_capacity(HASH_BYTES);
        for rand_num in random_numbers.iter() {
            if random_numbers_without_duplication.len() == 0 {
                random_numbers_without_duplication.push(*rand_num);
            };

            let mut is_exist = false;
            for n in random_numbers_without_duplication.iter() {
                if n == rand_num {
                    is_exist = true;
                    break;
                };
            };

            if is_exist == false {
                random_numbers_without_duplication.push(*rand_num);
            };
        };

        let winners_count: u8 = self.winners_count;
        if winners_count as usize > random_numbers_without_duplication.len() {
            return Err(
                LotteryError::InsufficientRandomNumbers.into()
            );
        };

        let mut counter = 0u8;
        let mut winners_index: Vec<u32> = Vec::with_capacity(winners_count as usize);
        for w_i in random_numbers_without_duplication.iter() {
            counter += 1;

            if counter > winners_count {
                break;
            };

            winners_index.push(*w_i);
        };
        sol_log("Winners_Index :");
        msg!("{:?}", winners_index);

        sol_log("Winners :");
        for w in winners_index {
            // read winner pubkey
            let winner_pubkey = match Self::get_ticket(lottery_account_info, w as usize) {
                Ok(pubkey) => pubkey,
                Err(_) => return Err(
                    LotteryError::FailedToGetTicket.into()
                )
            };

            self.winners.push(
                (
                    winner_pubkey,
                    bool::default()
                )
            );

            winner_pubkey.log();
        };

        Ok(())
    }

    pub fn get_winner_info(
        &mut self,
        winner_account: &Pubkey
    ) -> Result<u8, ProgramError> {
        let mut winning_count = 0u8;
        for winner in self.winners.iter_mut() {
            if &winner.0 == winner_account && winner.1 == false {
                winning_count += 1;
                winner.1 = true;
            };
        };

        if winning_count == 0 {
            Err(
                LotteryError::WinnerNotFound.into()
            )
        } else {
            Ok(winning_count)
        }
    }

    pub fn get_ticket(
        lottery_account_info: &AccountInfo,
        ticket_index: usize
    ) -> Result<Pubkey, u8> {
        let lottery_account_data = lottery_account_info
            .data
            .try_borrow()
            .map_err(|_| 0u8)?;

        let initial_bytes = u64::from_le_bytes(
            lottery_account_data
                .get(9..17)
                .ok_or(1u8)?
                .try_into()
                .map_err(|_| 2u8)?
        );

        let ticket_offset = initial_bytes.checked_add(
            (ticket_index as usize).checked_mul(
                size_of::<Pubkey>()
            ).ok_or(3)? as u64
        ).ok_or(4)? as usize;

        let ticket = Pubkey::try_from_slice(
            lottery_account_info
                .data
                .try_borrow()
                .map_err(|_| 5u8)?
                .get(
                    ticket_offset..ticket_offset.checked_add(
                        size_of::<Pubkey>()
                    ).ok_or(6)?
                )
                .ok_or(7u8)?
        ).map_err(|_| 8u8)?;

        Ok(ticket)
    }

    pub fn check_max_tickets_per_instruction(tickets_amount: u32) -> ProgramResult {
        if tickets_amount as usize > MAX_TICKETS_PER_INSTRUCTION {
            return Err(
                LotteryError::MaxTicketsPerInstructionExceeded.into()
            )
        };

        Ok(())
    }

    pub fn close_lottery_account(
        lottery_account_info: &AccountInfo,
        rent_exempt_recepient_account_info: &AccountInfo
    ) -> ProgramResult {
        // send all lamports to the recepient
        let lottery_account_balance = lottery_account_info.lamports();
        let rent_exempt_recepient_account_balance = rent_exempt_recepient_account_info.lamports();
        
        **lottery_account_info.try_borrow_mut_lamports()? = 0;
        **rent_exempt_recepient_account_info.try_borrow_mut_lamports()? = (rent_exempt_recepient_account_balance)
            .checked_add(lottery_account_balance)
            .ok_or::<ProgramError>(LotteryError::Overflow.into())?;

        // clear data field and write "CLOSED_LOTTERY_ACCOUNT" discriminator
        //  clear data
        let initial_bytes = u64::from_le_bytes(
            lottery_account_info
                .try_borrow_data()
                .unwrap()
                .get(9..17)
                .unwrap()
                .try_into()
                .unwrap()
        );
        
        sol_memset(
            &mut lottery_account_info
                .data
                .try_borrow_mut()
                .unwrap(),
            0,
            initial_bytes
                .try_into()
                .unwrap()
        );

        //  write new discriminator
        sol_memcpy(
            &mut lottery_account_info
                .data
                .try_borrow_mut()
                .unwrap(),
            &Self::get_closed_account_discriminator(),
            DISCRIMINATOR_LENTGH
        );

        Ok(())
    }

    pub fn get_closed_account_discriminator() -> [u8; DISCRIMINATOR_LENTGH] {
        hash(CLOSED_LOTTERY_ACCOUNT_DISCRIMINATOR.as_bytes())
            .to_bytes()
            .get(..8)
            .and_then(|slice| slice.try_into().ok())
            .map(|dis: [u8; 8]| dis)
            .unwrap()
    }

    pub fn get_discriminator() -> [u8; 8] {
        hash(LOTTERY_ACCOUNT_DISCRIMINATOR.as_bytes())
            .to_bytes()
            .get(..8)
            .and_then(|slice| slice.try_into().ok())
            .map(|dis: [u8; 8]| dis)
            .unwrap()
    }
}

#[derive(Debug, Default, BorshDeserialize, BorshSerialize, Clone, Copy, PartialEq, Eq)]
pub struct User {
    pub discriminator: [u8; DISCRIMINATOR_LENTGH],
    pub canonical_bump: u8,
    pub lottery: Pubkey,
    pub authority: Pubkey,
    pub total_tickets_value: u64, // USDC
    pub total_rent_exempt_paied: u64, // Lamports
    pub total_tickets_acquired: u32,
    pub created_at: Time
}
impl User {
    pub const LEN: usize =
        DISCRIMINATOR_LENTGH +
        CANONICAL_BUMP_LENGTH +
        size_of::<Pubkey>() +
        size_of::<Pubkey>() +
        size_of::<u64>() +
        size_of::<u64>() +
        size_of::<u32>() +
        size_of::<Time>();

    pub fn validate_user_account(
        user_account_info: &AccountInfo,
        program_id: &Pubkey,
        lottery_account: &Pubkey,
        user_account_authority: &Pubkey
    ) -> ProgramResult {
        if user_account_info.owner != program_id {
            return Err(
                ProgramError::IncorrectProgramId
            );
        };

        if sol_memcmp(
            &user_account_info
                .data
                .try_borrow()
                .unwrap(),
            &Self::get_discriminator(),
            DISCRIMINATOR_LENTGH
        ) != 0 {
            return Err(
                LotteryError::InvalidDiscriminator.into()
            );
        };

        // Both user_account_authority & lottery_account are validated here
        let user_pda_addr = Pubkey::create_program_address(
            &[
                USER_ACCOUNT_SEED.as_bytes(),
                user_account_authority.to_bytes().as_slice(),
                lottery_account.to_bytes().as_slice(),
                &[
                    *user_account_info
                        .data
                        .try_borrow()
                        .unwrap()
                        .get(8)
                        .unwrap()
                ]
            ],
            program_id
        ).map_err::<ProgramError, _>(|_| LotteryError::FailedToFindProgramAddress.into())?;

        if user_pda_addr != *user_account_info.key {
            return Err(
                ProgramError::InvalidSeeds
            );
        };

        Ok(())
    }

    pub fn validate_user_holding_tickets_amount(
        &self,
        maximum_amount: &Option<u32>,
        tickets_amount_to_buy_now: u32
    ) -> ProgramResult {
        if maximum_amount.is_none() {
            return Ok(());
        };

        let max_permitted_tickets_to_hold = maximum_amount.unwrap();
        if 
            &self.total_tickets_acquired == &max_permitted_tickets_to_hold ||
            &self.total_tickets_acquired.checked_add(tickets_amount_to_buy_now).unwrap() > &max_permitted_tickets_to_hold
        {
            Err(
                LotteryError::MaxTicketsAmountViolated.into()
            )
        } else {
            Ok(())
        }
    }

    pub fn close_user_account(
        user_account_info: &AccountInfo,
        rent_exempt_recepient_account_info: &AccountInfo
    ) -> ProgramResult {
        // send all lamports to the recepient
        let user_account_balance = user_account_info.lamports();
        let rent_exempt_recepient_account_balance = rent_exempt_recepient_account_info.lamports();
        
        **user_account_info.try_borrow_mut_lamports()? = 0;
        **rent_exempt_recepient_account_info.try_borrow_mut_lamports()? = (rent_exempt_recepient_account_balance)
            .checked_add(user_account_balance)
            .ok_or::<ProgramError>(LotteryError::Overflow.into())?;

        // clear data field and write "CLOSED_USER_ACCOUNT" discriminator
        //  clear data
        sol_memset(
            &mut user_account_info
                .data
                .try_borrow_mut()
                .unwrap(),
            0,
            Self::LEN
        );

        //  write new discriminator
        sol_memcpy(
            &mut user_account_info
                .data
                .try_borrow_mut()
                .unwrap(),
            &Self::get_closed_account_discriminator(),
            DISCRIMINATOR_LENTGH
        );

        Ok(())
    }

    pub fn get_discriminator() -> [u8; DISCRIMINATOR_LENTGH] {
        hash(USER_ACCOUNT_DISCRIMINATOR.as_bytes())
            .to_bytes()
            .get(..8)
            .and_then(|slice| slice.try_into().ok())
            .map(|dis: [u8; 8]| dis)
            .unwrap()
    }

    pub fn get_closed_account_discriminator() -> [u8; DISCRIMINATOR_LENTGH] {
        hash(CLOSED_USER_ACCOUNT_DISCRIMINATOR.as_bytes())
            .to_bytes()
            .get(..8)
            .and_then(|slice| slice.try_into().ok())
            .map(|dis: [u8; 8]| dis)
            .unwrap()
    }
}

#[cfg(test)]
mod test_config {
    use {
        borsh::BorshSerialize,
        solana_program::clock::Epoch,
        std::{
            rc::Rc,
            cell::RefCell,
            str::FromStr
        },
        crate::error::LotteryError
    };
    use super::{
        Config,
        AccountInfo,
        CONFIG_ACCOUNT_SEED,
        Pubkey,
        ProgramError,
        PYTH_PULL_ORACLE_RECEIVER_PROGRAM_ID
    };

    #[test]
    fn test_succeed_check_is_pause() {
        let config_account = Config::default();
        config_account.check_is_pause().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_fail_check_is_pause() {
        let mut config_account = Config::default();
        config_account.is_pause = true;

        config_account.check_is_pause().unwrap();
    }

    #[test]
    fn test_succeed_check_is_pause_raw() {
        let config_account = Config::default();

        let mut data: [u8; Config::LEN] = [0; Config::LEN];
        config_account.serialize(
            &mut &mut data.as_mut_slice()
        ).unwrap();

        Config::check_is_pause_raw(
            &AccountInfo {
                key: &Pubkey::new_unique(),
                lamports: Rc::new(RefCell::new(&mut u64::default())),
                data: Rc::new(RefCell::new(&mut data)),
                owner: &Pubkey::new_unique(),
                rent_epoch: Epoch::default(),
                is_signer: false,
                is_writable: false,
                executable: false
            }
        ).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_fail_check_is_pause_raw() {
        let mut config_account = Config::default();
        config_account.is_pause = true;

        let mut data: [u8; Config::LEN] = [0; Config::LEN];
        config_account.serialize(
            &mut &mut data.as_mut_slice()
        ).unwrap();

        Config::check_is_pause_raw(
            &AccountInfo {
                key: &Pubkey::new_unique(),
                lamports: Rc::new(RefCell::new(&mut u64::default())),
                data: Rc::new(RefCell::new(&mut data)),
                owner: &Pubkey::new_unique(),
                rent_epoch: Epoch::default(),
                is_signer: false,
                is_writable: false,
                executable: false
            }
        ).unwrap();
    }

    #[test]
    fn test_succeed_validate_config_account() {
        let program_id = Pubkey::from_str("EGxRBwjoC99LtLznAyLFcSxaiCrzPiXW3gHmemq4pump").unwrap();
        let config_account_addr: Pubkey;

        let mut config_account = Config::default();
        
        let (
            pda_addr,
            pda_bump
        ) = Pubkey::try_find_program_address(
            &[
                CONFIG_ACCOUNT_SEED.as_bytes()
            ],
            &program_id
        ).unwrap();
        
        config_account.canonical_bump = pda_bump;
        config_account.discriminator = Config::get_discriminator();
        config_account_addr = pda_addr;

        let mut data: [u8; Config::LEN] = [0; Config::LEN];
        config_account.serialize(
            &mut &mut data.as_mut_slice()
        ).unwrap();

        Config::validate_config_account(
            &AccountInfo {
                key: &config_account_addr,
                lamports: Rc::new(RefCell::new(&mut u64::default())),
                data: Rc::new(RefCell::new(&mut data)),
                owner: &program_id,
                rent_epoch: Epoch::default(),
                is_signer: false,
                is_writable: false,
                executable: false
            },
            &program_id
        ).unwrap();
    }

    #[test]
    fn test_validate_config_account_error_invalid_programid() {
        let program_id = Pubkey::from_str("EGxRBwjoC99LtLznAyLFcSxaiCrzPiXW3gHmemq4pump").unwrap();
        let config_account_addr: Pubkey;

        let mut config_account = Config::default();
        
        let (
            pda_addr,
            pda_bump
        ) = Pubkey::try_find_program_address(
            &[
                CONFIG_ACCOUNT_SEED.as_bytes()
            ],
            &program_id
        ).unwrap();
        
        config_account.canonical_bump = pda_bump;
        config_account.discriminator = Config::get_discriminator();
        config_account_addr = pda_addr;

        let mut data: [u8; Config::LEN] = [0; Config::LEN];
        config_account.serialize(
            &mut &mut data.as_mut_slice()
        ).unwrap();

        let result = Config::validate_config_account(
            &AccountInfo {
                key: &config_account_addr,
                lamports: Rc::new(RefCell::new(&mut u64::default())),
                data: Rc::new(RefCell::new(&mut data)),
                owner: &Pubkey::default(),
                rent_epoch: Epoch::default(),
                is_signer: false,
                is_writable: false,
                executable: false
            },
            &program_id
        );

        if let Err(error) = result {
            assert_eq!(
                error,
                ProgramError::IncorrectProgramId
            );
        } else {
            panic!("Test must gives us an erorr!");
        };
    }

    #[test]
    fn test_validate_config_account_error_invalid_discriminator() {
        let program_id = Pubkey::from_str("EGxRBwjoC99LtLznAyLFcSxaiCrzPiXW3gHmemq4pump").unwrap();
        let config_account_addr: Pubkey;

        let mut config_account = Config::default();
        
        let (
            pda_addr,
            pda_bump
        ) = Pubkey::try_find_program_address(
            &[
                CONFIG_ACCOUNT_SEED.as_bytes()
            ],
            &program_id
        ).unwrap();
        
        config_account.canonical_bump = pda_bump;
        config_account.discriminator = [0; 8];
        config_account_addr = pda_addr;

        let mut data: [u8; Config::LEN] = [0; Config::LEN];
        config_account.serialize(
            &mut &mut data.as_mut_slice()
        ).unwrap();

        let result = Config::validate_config_account(
            &AccountInfo {
                key: &config_account_addr,
                lamports: Rc::new(RefCell::new(&mut u64::default())),
                data: Rc::new(RefCell::new(&mut data)),
                owner: &program_id,
                rent_epoch: Epoch::default(),
                is_signer: false,
                is_writable: false,
                executable: false
            },
            &program_id
        );

        if let Err(error) = result {
            assert_eq!(
                error,
                ProgramError::Custom(
                    LotteryError::InvalidDiscriminator as u32
                )
            );
        } else {
            panic!("Test must gives us an erorr!");
        };
    }

    #[test]
    fn test_validate_config_account_error_invalid_account_address_invalid_seeds() {
        let program_id = Pubkey::from_str("EGxRBwjoC99LtLznAyLFcSxaiCrzPiXW3gHmemq4pump").unwrap();
        let config_account_addr: Pubkey;

        let mut config_account = Config::default();
        
        let (
            pda_addr,
            pda_bump
        ) = Pubkey::try_find_program_address(
            &[
                CONFIG_ACCOUNT_SEED.as_bytes(),
                b"invalid_seed"
            ],
            &program_id
        ).unwrap();
        
        config_account.canonical_bump = pda_bump;
        config_account.discriminator = Config::get_discriminator();
        config_account_addr = pda_addr;

        let mut data: [u8; Config::LEN] = [0; Config::LEN];
        config_account.serialize(
            &mut &mut data.as_mut_slice()
        ).unwrap();

        let result = Config::validate_config_account(
            &AccountInfo {
                key: &config_account_addr,
                lamports: Rc::new(RefCell::new(&mut u64::default())),
                data: Rc::new(RefCell::new(&mut data)),
                owner: &program_id,
                rent_epoch: Epoch::default(),
                is_signer: false,
                is_writable: false,
                executable: false
            },
            &program_id
        );

        if let Err(error) = result {
            assert_eq!(
                error,
                ProgramError::Custom(
                    LotteryError::InvalidConfigAccount as u32
                )
            );
        } else {
            panic!("Test must gives us an erorr!");
        };
    }

    #[test]
    fn test_validate_price_feed_accounts() {
        let mut config_account = Config::default();
        config_account.pyth_price_feed_accounts = [
            Pubkey::new_from_array([1; 32]), // Sol
            Pubkey::new_from_array([2; 32]), // Btc
            Pubkey::new_from_array([3; 32]) // Eth
        ];

        let mut data: [u8; Config::LEN] = [0; Config::LEN];
        config_account.serialize(
            &mut data.as_mut_slice()
        ).unwrap();

        // success
        {
            config_account.validate_price_feed_accounts(
                // Sol price feed account
                &AccountInfo {
                    key: &config_account.pyth_price_feed_accounts[0],
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut [])),
                    owner: &PYTH_PULL_ORACLE_RECEIVER_PROGRAM_ID,
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                },
                // Btc price feed account
                &AccountInfo {
                    key: &config_account.pyth_price_feed_accounts[1],
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut [])),
                    owner: &PYTH_PULL_ORACLE_RECEIVER_PROGRAM_ID,
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                },
                // Eth price feed account
                &AccountInfo {
                    key: &config_account.pyth_price_feed_accounts[2],
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut [])),
                    owner: &PYTH_PULL_ORACLE_RECEIVER_PROGRAM_ID,
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                }
            ).unwrap();
        }

        // fail(all) - invalid_price_feed_accounts_owner
        {
            let result = config_account.validate_price_feed_accounts(
                // Sol price feed account
                &AccountInfo {
                    key: &config_account.pyth_price_feed_accounts[0],
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut [])),
                    owner: &Pubkey::new_from_array([5; 32]),
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                },
                // Btc price feed account
                &AccountInfo {
                    key: &config_account.pyth_price_feed_accounts[1],
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut [])),
                    owner: &Pubkey::new_from_array([5; 32]),
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                },
                // Eth price feed account
                &AccountInfo {
                    key: &config_account.pyth_price_feed_accounts[2],
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut [])),
                    owner: &Pubkey::new_from_array([5; 32]),
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                }
            );
            
            if let Err(error) = result {
                assert_eq!(
                    error,
                    ProgramError::Custom(
                        LotteryError::InvalidPriceFeedAccountsOwner as u32
                    )
                );
            } else {
                panic!("We must have an invalid_owner error!");
            };
        }

        // fail(any) - invalid_price_feed_accounts_owner
        {
            let result = config_account.validate_price_feed_accounts(
                // Sol price feed account
                &AccountInfo {
                    key: &config_account.pyth_price_feed_accounts[0],
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut [])),
                    owner: &PYTH_PULL_ORACLE_RECEIVER_PROGRAM_ID,
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                },
                // Btc price feed account
                &AccountInfo {
                    key: &config_account.pyth_price_feed_accounts[1],
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut [])),
                    owner: &Pubkey::new_from_array([5; 32]),
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                },
                // Eth price feed account
                &AccountInfo {
                    key: &config_account.pyth_price_feed_accounts[2],
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut [])),
                    owner: &PYTH_PULL_ORACLE_RECEIVER_PROGRAM_ID,
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                }
            );
            
            if let Err(error) = result {
                assert_eq!(
                    error,
                    ProgramError::Custom(
                        LotteryError::InvalidPriceFeedAccountsOwner as u32
                    )
                );
            } else {
                panic!("We must have an invalid_owner error!");
            };
        }

        // fail - invalid_sol_price_feed_account
        {
            let result = config_account.validate_price_feed_accounts(
                // Sol price feed account
                &AccountInfo {
                    key: &Pubkey::new_from_array([5; 32]),
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut [])),
                    owner: &PYTH_PULL_ORACLE_RECEIVER_PROGRAM_ID,
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                },
                // Btc price feed account
                &AccountInfo {
                    key: &config_account.pyth_price_feed_accounts[1],
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut [])),
                    owner: &PYTH_PULL_ORACLE_RECEIVER_PROGRAM_ID,
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                },
                // Eth price feed account
                &AccountInfo {
                    key: &config_account.pyth_price_feed_accounts[2],
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut [])),
                    owner: &PYTH_PULL_ORACLE_RECEIVER_PROGRAM_ID,
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                }
            );

            if let Err(error) = result {
                assert_eq!(
                    error,
                    ProgramError::Custom(
                        LotteryError::InvalidSolPriceFeedAccount as u32
                    )
                );
            } else {
                panic!("We must have an invalid_sol_price_feed_account error!");
            };
        }

        // fail - invalid_btc_price_feed_account
        {
            let result = config_account.validate_price_feed_accounts(
                // Sol price feed account
                &AccountInfo {
                    key: &config_account.pyth_price_feed_accounts[0],
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut [])),
                    owner: &PYTH_PULL_ORACLE_RECEIVER_PROGRAM_ID,
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                },
                // Btc price feed account
                &AccountInfo {
                    key: &Pubkey::new_from_array([6; 32]),
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut [])),
                    owner: &PYTH_PULL_ORACLE_RECEIVER_PROGRAM_ID,
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                },
                // Eth price feed account
                &AccountInfo {
                    key: &config_account.pyth_price_feed_accounts[2],
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut [])),
                    owner: &PYTH_PULL_ORACLE_RECEIVER_PROGRAM_ID,
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                }
            );

            if let Err(error) = result {
                assert_eq!(
                    error,
                    ProgramError::Custom(
                        LotteryError::InvalidBtcPriceFeedAccount as u32
                    )
                );
            } else {
                panic!("We must have an invalid_btc_price_feed_account error!");
            };
        }

        // fail - invalid_eth_price_feed_account
        {
            let result = config_account.validate_price_feed_accounts(
                // Sol price feed account
                &AccountInfo {
                    key: &config_account.pyth_price_feed_accounts[0],
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut [])),
                    owner: &PYTH_PULL_ORACLE_RECEIVER_PROGRAM_ID,
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                },
                // Btc price feed account
                &AccountInfo {
                    key: &config_account.pyth_price_feed_accounts[1],
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut [])),
                    owner: &PYTH_PULL_ORACLE_RECEIVER_PROGRAM_ID,
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                },
                // Eth price feed account
                &AccountInfo {
                    key: &Pubkey::new_from_array([6; 32]),
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut [])),
                    owner: &PYTH_PULL_ORACLE_RECEIVER_PROGRAM_ID,
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                }
            );

            if let Err(error) = result {
                assert_eq!(
                    error,
                    ProgramError::Custom(
                        LotteryError::InvalidEthPriceFeedAccount as u32
                    )
                );
            } else {
                panic!("We must have an invalid_eth_price_feed_account error!");
            };
        }
    }

    #[test]
    fn test_validate_fee_per_ticket() {
        // success
        Config::validate_fee_per_ticket(&45.6).unwrap();

        // fail
        if let Ok(_) = Config::validate_fee_per_ticket(&101.0) {
            panic!("This must fail!");
        };
    } 
}

#[cfg(test)]
mod test_lottery {
    use borsh::{BorshDeserialize, BorshSerialize};
    use solana_program::{
        pubkey::Pubkey,
        account_info::AccountInfo,
        clock::Epoch,
        program_error::ProgramError,
        hash::hash
    };
    use super::{
        LotteryError,
        Lottery,
        LotteryState
    };
    use std::{
        rc::Rc,
        cell::RefCell
    };

    #[test]
    fn test_validate_lottery_account() {
        let program_id = Pubkey::new_from_array([1; 32]);
        let mut lottery_account = Lottery::default();
        lottery_account.discriminator = Lottery::get_discriminator();

        let mut data: [u8; 500] = [0; 500];
        lottery_account.serialize(
            &mut data.as_mut_slice()
        ).unwrap();

        // success
        {
            Lottery::validate_lottery_account(
                &AccountInfo {
                    key: &Pubkey::new_unique(),
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut data)),
                    owner: &program_id,
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                },
                &program_id
            ).unwrap();
        }

        // fail - invalid program id
        {
            let result = Lottery::validate_lottery_account(
                &AccountInfo {
                    key: &Pubkey::new_unique(),
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut data)),
                    owner: &Pubkey::new_unique(),
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                },
                &program_id
            );

            if let Err(error) = result {
                assert_eq!(
                    error,
                    ProgramError::IncorrectProgramId
                );
            } else {
                panic!("It must panic.");
            };
        }

        // fail - invalid discriminator
        {
            let mut new_data = data.get_mut(8..).unwrap();

            let result = Lottery::validate_lottery_account(
                &AccountInfo {
                    key: &Pubkey::new_unique(),
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut new_data)),
                    owner: &program_id,
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                },
                &program_id
            );

            if let Err(error) = result {
                assert_eq!(
                    error,
                    ProgramError::Custom(
                        LotteryError::InvalidDiscriminator as u32
                    )
                );
            } else {
                panic!("It must panic.");
            };
        }
    }

    #[test]
    fn test_lottery_timestamps() {
        let mut lottery_account = Lottery::default();
        let mut current_time: i64;

        // not_started state
        {
            // true
            {
                lottery_account.starting_time = 1000;
                current_time = 100;

                assert_eq!(
                    lottery_account.is_not_started(current_time),
                    true
                );
            }

            // false 
            {
                lottery_account.starting_time = 1000;
                current_time = 1000;

                assert_eq!(
                    lottery_account.is_not_started(current_time),
                    false
                );
            }
        }

        // started_and_not_ended state
        {
            // true
            {
                lottery_account.starting_time = 1000;
                lottery_account.ending_time = 2000;
                current_time = 1200;

                assert_eq!(
                    lottery_account.is_started_and_not_ended(current_time),
                    true
                );
            }

            // false
            {
                lottery_account.starting_time = 1000;
                lottery_account.ending_time = 2000;
                current_time = 2200;

                assert_eq!(
                    lottery_account.is_started_and_not_ended(current_time),
                    false
                );
            }
        }

        // ended state 
        {
            // true
            {
                lottery_account.ending_time = 2000;
                current_time = 2000;

                assert_eq!(
                    lottery_account.is_ended(current_time),
                    true
                );
            }

            // false
            {
                lottery_account.ending_time = 2000;
                current_time = 1800;

                assert_eq!(
                    lottery_account.is_ended(current_time),
                    false
                );
            }
        }
    }

    #[test]
    fn test_add_tickets_and_get_ticket() {
        let mut lottery_account = Lottery::default();
        lottery_account.initial_bytes = lottery_account.try_to_vec().unwrap().len() as u64;

        let mut lottery_account_data: Vec<u8> = vec![
            lottery_account.try_to_vec().unwrap(), vec![0; size_of::<Pubkey>() * 5]
        ].concat();
        let mut lottery_balance = solana_program::native_token::sol_to_lamports(1.0);
        let lottery_account_info = &AccountInfo {
            key: &Pubkey::new_unique(),
            lamports: Rc::new(RefCell::new(&mut lottery_balance)),
            data: Rc::new(RefCell::new(&mut lottery_account_data)),
            owner: &Pubkey::new_unique(),
            rent_epoch: Epoch::default(),
            is_signer: false,
            is_writable: false,
            executable: false
        };

        assert_eq!(
            lottery_account.tickets_total_amount,
            0
        );

        assert_eq!(
            lottery_account_info.data_len(),
            lottery_account.initial_bytes as usize + size_of::<Pubkey>() * 5
        );

        Lottery::add_ticket(
            lottery_account_info,
            lottery_account.initial_bytes,
            0,
            1,
            Pubkey::new_from_array([5; 32])
        );

        Lottery::add_ticket(
            lottery_account_info,
            lottery_account.initial_bytes,
            1,
            1,
            Pubkey::new_from_array([23; 32])
        );

        Lottery::add_ticket(
            lottery_account_info,
            lottery_account.initial_bytes,
            2,
            2,
            Pubkey::new_from_array([55; 32])
        );

        Lottery::add_ticket(
            lottery_account_info,
            lottery_account.initial_bytes,
            4,
            1,
            Pubkey::new_from_array([9; 32])
        );

        let updated_lottery_account = Lottery::deserialize(
            &mut &lottery_account_info.data.try_borrow().unwrap()[..]
        ).unwrap();

        assert_eq!(
            updated_lottery_account.tickets_total_amount,
            5
        );

        // println!("\n>>>>> {:?}", lottery_account_info.data.try_borrow().unwrap().get(207..).unwrap().get(..32));
        // println!(">>>>> {:?}", lottery_account_info.data.try_borrow().unwrap().get(207..).unwrap().get(32..64));
        // println!(">>>>> {:?}", lottery_account_info.data.try_borrow().unwrap().get(207..).unwrap().get(64..96));

        // println!("\n>>> {:?}", lottery_account_info.data.try_borrow().unwrap());

        assert_eq!(
            Lottery::get_ticket(lottery_account_info, 0).unwrap(),
            Pubkey::new_from_array([5; 32])
        );

        assert_eq!(
            Lottery::get_ticket(lottery_account_info, 1).unwrap(),
            Pubkey::new_from_array([23; 32])
        );

        assert_eq!(
            Lottery::get_ticket(lottery_account_info, 2).unwrap(),
            Pubkey::new_from_array([55; 32])
        );

        assert_eq!(
            Lottery::get_ticket(lottery_account_info, 3).unwrap(),
            Pubkey::new_from_array([55; 32])
        );

        assert_eq!(
            Lottery::get_ticket(lottery_account_info, 4).unwrap(),
            Pubkey::new_from_array([9; 32])
        );
    }

    #[test]
    fn test_get_lottery_state() {
        // LotteryState::Unknown
        {
            let mut lottery_account = Lottery::default();
            lottery_account.ending_time = 200;
            lottery_account.starting_time = 100;
            
            let current_time = 50;
            
            if lottery_account.get_lottery_state(current_time) != LotteryState::Unknown {
                panic!("Invalid lottery state (1)");
            };
        }

        // LotteryState::Unknown
        {
            let mut lottery_account = Lottery::default();
            lottery_account.ending_time = 200;
            lottery_account.starting_time = 100;
            
            let current_time = 150;
            
            if lottery_account.get_lottery_state(current_time) != LotteryState::Unknown {
                panic!("Invalid lottery state (2)");
            };
        }

        // LotteryState::Successful
        {
            let mut lottery_account = Lottery::default();
            lottery_account.ending_time = 200;
            lottery_account.starting_time = 100;
            lottery_account.minimum_tickets_amount_required_to_be_sold = 100;
            lottery_account.tickets_total_amount = 150;

            let current_time = 250;

            if lottery_account.get_lottery_state(current_time) != LotteryState::Successful {
                panic!("Invalid lottery state (3)");
            };
        }

        // LotteryState::Failed
        {
            let mut lottery_account = Lottery::default();
            lottery_account.ending_time = 200;
            lottery_account.starting_time = 100;
            lottery_account.minimum_tickets_amount_required_to_be_sold = 100;
            lottery_account.tickets_total_amount = 90;

            let current_time = 250;

            if lottery_account.get_lottery_state(current_time) != LotteryState::Failed {
                panic!("Invalid lottery state (4)");
            };
        }
    }

    #[test]
    fn test_pick_winners() {
        let hashes: [[u8; 32]; 10] = [
            self::get_sha256_hash(110),
            self::get_sha256_hash(111),
            self::get_sha256_hash(112),
            self::get_sha256_hash(113),
            self::get_sha256_hash(114),
            self::get_sha256_hash(115),
            self::get_sha256_hash(116),
            self::get_sha256_hash(117),
            self::get_sha256_hash(118),
            self::get_sha256_hash(119)
        ];

        for index in 0..10 {
            let mut lottery_account = Lottery::default();
            lottery_account.winners_count = 10;
            lottery_account.initial_bytes = lottery_account.try_to_vec().unwrap().len() as u64;

            let mut lottery_account_data: Vec<u8> = vec![
                lottery_account.try_to_vec().unwrap(), vec![0; size_of::<Pubkey>() * 100]
            ].concat();
            let mut lottery_balance = solana_program::native_token::sol_to_lamports(1.0);
            let lottery_account_info = &AccountInfo {
                key: &Pubkey::new_unique(),
                lamports: Rc::new(RefCell::new(&mut lottery_balance)),
                data: Rc::new(RefCell::new(&mut lottery_account_data)),
                owner: &Pubkey::new_unique(),
                rent_epoch: Epoch::default(),
                is_signer: false,
                is_writable: false,
                executable: false
            };

            Lottery::add_ticket(
                lottery_account_info,
                lottery_account.initial_bytes, 
                0, 
                30, 
                Pubkey::new_from_array([99; 32])
            );
            Lottery::add_ticket(
                lottery_account_info,
                lottery_account.initial_bytes, 
                30, 
                30, 
                Pubkey::new_from_array([88; 32])
            );
            Lottery::add_ticket(
                lottery_account_info,
                lottery_account.initial_bytes, 
                60, 
                40, 
                Pubkey::new_from_array([77; 32])
            );

            let mut updated_lottery_account = Lottery::deserialize(
                &mut &lottery_account_info.data.try_borrow_mut().unwrap()[..]
            ).unwrap();

            updated_lottery_account.pick_winners(
                &hashes[index],
                lottery_account_info
            ).unwrap();
        };
    }

    fn get_sha256_hash(price: i64) -> [u8; 32] {
        hash(price.to_le_bytes().as_slice()).to_bytes()
    }

    #[test]
    fn test_get_winner_info() {
        let mut lottery_account = Lottery::default();
        lottery_account.winners = vec![
            (Pubkey::new_from_array([1; 32]), false),
            (Pubkey::new_from_array([1; 32]), false),
            (Pubkey::new_from_array([1; 32]), false)
        ];

        // get winner's info for the first time
        {
            let winning_count = lottery_account
                .get_winner_info(&Pubkey::new_from_array([1; 32]))
                .unwrap();

            assert_eq!(winning_count, 3);
            assert_eq!(lottery_account.winners[0].1, true);
            assert_eq!(lottery_account.winners[1].1, true);
            assert_eq!(lottery_account.winners[2].1, true);
        }

        // try to get winner's info for the second time
        {
            let result = lottery_account.get_winner_info(
                &Pubkey::new_from_array([1; 32])
            );

            assert_eq!(
                result,
                Err(
                    ProgramError::Custom(
                        LotteryError::WinnerNotFound as u32
                    )
                )
            );
        }

        // try to get winner's info which does not exists
        {
            let result = lottery_account.get_winner_info(
                &Pubkey::new_from_array([5; 32])
            );

            assert_eq!(
                result,
                Err(
                    ProgramError::Custom(
                        LotteryError::WinnerNotFound as u32
                    )
                )
            );
        }
    }
}

#[cfg(test)]
mod test_user {
    use borsh::BorshSerialize;
    use super::{
        User,
        LotteryError,
        Pubkey,
        USER_ACCOUNT_SEED
    };
    use solana_program::{
        account_info::AccountInfo,
        clock::Epoch,
        program_error::ProgramError
    };
    use std::{
        rc::Rc,
        cell::RefCell
    };

    #[test]
    fn test_validate_user_account() {
        let program_id = Pubkey::new_from_array([5; 32]);
        let user_account_authority = Pubkey::new_from_array([7; 32]);
        let lottery_account = Pubkey::new_from_array([8; 32]);

        let mut user_account = User::default();
        user_account.authority = Pubkey::new_from_array([7; 32]);
        user_account.lottery = Pubkey::new_from_array([8; 32]);
        user_account.discriminator = User::get_discriminator();

        let user_pda = Pubkey::find_program_address(
            &[
                USER_ACCOUNT_SEED.as_bytes(),
                &user_account.authority.to_bytes(),
                &user_account.lottery.to_bytes()
            ],
            &program_id
        );
        user_account.canonical_bump = user_pda.1;

        let mut data: [u8; User::LEN] = [0; User::LEN];
        user_account.serialize(
            &mut data.as_mut_slice()
        ).unwrap();

        // success
        {
            User::validate_user_account(
                &AccountInfo {
                    key: &user_pda.0,
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut data)),
                    owner: &program_id,
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                },
                &program_id,
                &lottery_account,
                &user_account_authority
            ).unwrap();
        }

        // fail - invalid program id
        {
            let result = User::validate_user_account(
                &AccountInfo {
                    key: &user_pda.0,
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut data)),
                    owner: &Pubkey::new_unique(),
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                },
                &program_id,
                &lottery_account,
                &user_account_authority
            );

            assert_eq!(
                result,
                Err(
                    ProgramError::IncorrectProgramId
                )
            );
        }

        // fail - invalid discriminator
        {
            user_account.discriminator = [0; 8];

            let mut new_data: [u8; User::LEN] = [0; User::LEN];
            user_account.serialize(
                &mut new_data.as_mut_slice()
            ).unwrap();

            let result = User::validate_user_account(
                &AccountInfo {
                    key: &user_pda.0,
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut new_data)),
                    owner: &program_id,
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                },
                &program_id,
                &lottery_account,
                &user_account_authority
            );

            assert_eq!(
                result,
                Err(
                    ProgramError::Custom(
                        LotteryError::InvalidDiscriminator as u32
                    )
                )
            );
        }

        // fail - invalid seeds (invalid user_account_authority)
        {
            let result = User::validate_user_account(
                &AccountInfo {
                    key: &Pubkey::new_unique(),
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut data)),
                    owner: &program_id,
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                },
                &program_id,
                &lottery_account,
                &Pubkey::default()
            );

            if !(result == Err(ProgramError::InvalidSeeds) || result == Err(ProgramError::Custom(LotteryError::FailedToFindProgramAddress as u32))) {
                panic!("1. invalid error.");
            };
        }

        // fail - invalid seeds (invalid lottery_account)
        {
            let result = User::validate_user_account(
                &AccountInfo {
                    key: &Pubkey::new_unique(),
                    lamports: Rc::new(RefCell::new(&mut u64::default())),
                    data: Rc::new(RefCell::new(&mut data)),
                    owner: &program_id,
                    rent_epoch: Epoch::default(),
                    is_signer: false,
                    is_writable: false,
                    executable: false
                },
                &program_id,
                &Pubkey::default(),
                &user_account_authority
            );

            if !(result == Err(ProgramError::InvalidSeeds) || result == Err(ProgramError::Custom(LotteryError::FailedToFindProgramAddress as u32))) {
                panic!("2. invalid error.");
            };
        }
    }

    #[test]
    fn test_validate_user_holding_tickets_amount() {
        let mut user_account = User::default();
        
        // success
        {
            // 1.
            user_account.total_tickets_acquired = 25;
            user_account.validate_user_holding_tickets_amount(&Some(30), 4).unwrap();

            // 2.
            user_account.validate_user_holding_tickets_amount(&None, 40).unwrap();
        }

        // fail
        {
            user_account.total_tickets_acquired = 45;
            let result = user_account.validate_user_holding_tickets_amount(&Some(50), 10);

            assert_eq!(
                result,
                Err(
                    ProgramError::Custom(
                        LotteryError::MaxTicketsAmountViolated as u32
                    )
                )
            );
        }
    }
}