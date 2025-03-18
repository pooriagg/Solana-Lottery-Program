use solana_program::pubkey::Pubkey;

pub type Time = i64;
pub type PriceFeedAccount = Pubkey;
pub type PricePublishTime = i64;
pub type Price = i64;
pub type UserAccount = Pubkey;
pub type IsWithdrawed = bool;
pub type WinnerStatus = (UserAccount, IsWithdrawed);
pub type RandomNumberInfo = (PriceFeedAccount, PricePublishTime, Price);