use std::{convert::TryInto, mem::size_of};

use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar,
};

pub enum StoreInstruction {
    ///   0. `[signer]` The initializer's account, which will be set as owner of store account
    ///   0. `[writable]` The store account
    ///   0. `[writable]` account with payment tokens, to take tokens when sell, (owner will be updated to program)
    ///   0. `[writable]` account with store tokens, to take tokens when buy, (owner will be updated to program)
    ///   0. `[]` The token program
    ///   0. `[]` Rent sysvar
    InitializeAccount { price: u64 },

    ///   0. `[signer]` The owner of store account
    ///   0. `[writable]` The store account
    UpdatePrice { price: u64 },

    ///   0. `[signer]` owner of token accounts to transfer
    ///   0. `[]` The store account
    ///   0. `[writable]` store account with payment tokens (owner must be same as store owner)
    ///   0. `[writable]` store account with store tokens (same as in store info account)
    ///   0. `[writable]` user account to transfer payment tokens from (owner is signer)
    ///   0. `[writable]` user account for store tokens
    ///   0. `[]` The PDA account
    ///   0. `[]` The token program
    Buy {
        amount: u64,
        /// price same as in store account
        price: u64,
    },

    ///   0. `[signer]` owner of store tokens account to sell
    ///   0. `[]` The store account
    ///   0. `[writable]` store account with payment tokens for sell payment (same as in store info account)
    ///   0. `[writable]` account to transfer store tokens to (owner must be same as store owner)
    ///   0. `[writable]` user account to transfer payment tokens to
    ///   0. `[writable]` user account with store tokens to sell (owner is signer)
    ///   0. `[]` The PDA account
    ///   0. `[]` The token program
    Sell {
        amount: u64,
        /// price same as in store account
        price: u64,
    },
    // ReleaseAccounts (close or get back accounts owned by program)
    // CreateBuyOffer
    // CreateSellOffer
    // AcceptBuyOffer
    // AcceptSellOffer
}

impl StoreInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (tag, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        Ok(match tag {
            0 => Self::InitializeAccount {
                price: Self::unpack_u64(0, rest)?,
            },
            1 => Self::UpdatePrice {
                price: Self::unpack_u64(0, rest)?,
            },
            2 => Self::Buy {
                amount: Self::unpack_u64(0, rest)?,
                price: Self::unpack_u64(8, rest)?,
            },
            3 => Self::Sell {
                amount: Self::unpack_u64(0, rest)?,
                price: Self::unpack_u64(8, rest)?,
            },
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }

    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        match self {
            &Self::InitializeAccount { price } => {
                buf.push(0);
                buf.extend_from_slice(&price.to_le_bytes());
            }
            &Self::UpdatePrice { price } => {
                buf.push(1);
                buf.extend_from_slice(&price.to_le_bytes());
            }
            &Self::Buy { amount, price } => {
                buf.push(2);
                buf.extend_from_slice(&amount.to_le_bytes());
                buf.extend_from_slice(&price.to_le_bytes());
            }
            &Self::Sell { amount, price } => {
                buf.push(3);
                buf.extend_from_slice(&amount.to_le_bytes());
                buf.extend_from_slice(&price.to_le_bytes());
            }
        }
        buf
    }

    fn unpack_u64(offset: usize, input: &[u8]) -> Result<u64, ProgramError> {
        let price = input
            .get(offset..offset + 8)
            .and_then(|slice| slice.try_into().ok())
            .map(u64::from_le_bytes)
            .ok_or(ProgramError::InvalidInstructionData)?;
        Ok(price)
    }
}

pub fn initialyze_account_instruction(
    price: u64,
    store_program_id: &Pubkey,
    owner_pubkey: &Pubkey,
    store_account_pubkey: &Pubkey,
    account_with_payment_tokens: &Pubkey,
    account_with_store_tokens: &Pubkey,
    token_program_id: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = StoreInstruction::InitializeAccount { price }.pack();

    let accounts = vec![
        AccountMeta::new(*owner_pubkey, true),
        AccountMeta::new(*store_account_pubkey, false),
        AccountMeta::new(*account_with_payment_tokens, false),
        AccountMeta::new(*account_with_store_tokens, false),
        AccountMeta::new_readonly(*token_program_id, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    Ok(Instruction {
        program_id: *store_program_id,
        accounts,
        data,
    })
}

pub fn update_price_instruction(
    price: u64,
    store_program_id: &Pubkey,
    owner_pubkey: &Pubkey,
    store_account_pubkey: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = StoreInstruction::UpdatePrice { price }.pack();

    let accounts = vec![
        AccountMeta::new(*owner_pubkey, true),
        AccountMeta::new(*store_account_pubkey, false),
    ];

    Ok(Instruction {
        program_id: *store_program_id,
        accounts,
        data,
    })
}

pub fn buy_instruction(
    amount: u64,
    price: u64,
    store_program_id: &Pubkey,
    buyer_pubkey: &Pubkey,
    store_account_pubkey: &Pubkey,
    store_account_with_payment_tokens: &Pubkey,
    store_account_with_store_tokens: &Pubkey,
    user_account_with_payment_tokens: &Pubkey,
    user_account_with_store_tokens: &Pubkey,
    pda: &Pubkey,
    token_program_id: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = StoreInstruction::Buy { amount, price }.pack();

    let accounts = vec![
        AccountMeta::new(*buyer_pubkey, true),
        AccountMeta::new(*store_account_pubkey, false),
        AccountMeta::new(*store_account_with_payment_tokens, false),
        AccountMeta::new(*store_account_with_store_tokens, false),
        AccountMeta::new(*user_account_with_payment_tokens, false),
        AccountMeta::new(*user_account_with_store_tokens, false),
        AccountMeta::new_readonly(*pda, false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];

    Ok(Instruction {
        program_id: *store_program_id,
        accounts,
        data,
    })
}
pub fn sell_instruction(
    amount: u64,
    price: u64,
    store_program_id: &Pubkey,
    buyer_pubkey: &Pubkey,
    store_account_pubkey: &Pubkey,
    store_account_with_payment_tokens: &Pubkey,
    store_account_with_store_tokens: &Pubkey,
    user_account_with_payment_tokens: &Pubkey,
    user_account_with_store_tokens: &Pubkey,
    pda: &Pubkey,
    token_program_id: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = StoreInstruction::Sell { amount, price }.pack();

    let accounts = vec![
        AccountMeta::new(*buyer_pubkey, true),
        AccountMeta::new(*store_account_pubkey, false),
        AccountMeta::new(*store_account_with_payment_tokens, false),
        AccountMeta::new(*store_account_with_store_tokens, false),
        AccountMeta::new(*user_account_with_payment_tokens, false),
        AccountMeta::new(*user_account_with_store_tokens, false),
        AccountMeta::new_readonly(*pda, false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];

    Ok(Instruction {
        program_id: *store_program_id,
        accounts,
        data,
    })
}
