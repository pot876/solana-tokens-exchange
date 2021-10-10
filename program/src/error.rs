use thiserror::Error;

use solana_program::program_error::ProgramError;

#[derive(Error, Debug, Copy, Clone)]
pub enum StoreError {
    #[error("Account Price Mismatch")]
    AccountPriceMismatch,
}

impl From<StoreError> for ProgramError {
    fn from(e: StoreError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
