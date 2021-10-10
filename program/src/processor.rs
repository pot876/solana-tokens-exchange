use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::IsInitialized,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};

use crate::{error::StoreError, instruction::StoreInstruction, state::Store};

pub struct Processor;
impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = StoreInstruction::unpack(instruction_data)?;
        match instruction {
            StoreInstruction::InitializeAccount { price } => {
                Self::process_init_store(accounts, price, program_id)
            }
            StoreInstruction::UpdatePrice { price } => {
                Self::process_update_price(accounts, price, program_id)
            }
            StoreInstruction::Buy { amount, price } => {
                Self::process_buy(accounts, amount, price, program_id)
            }
            StoreInstruction::Sell { amount, price } => {
                Self::process_sell(accounts, amount, price, program_id)
            }
        }
    }

    fn process_init_store(
        accounts: &[AccountInfo],
        price: u64,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let owner = next_account_info(account_info_iter)?;

        if !owner.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let store_account = next_account_info(account_info_iter)?;

        let native_tokens_account = next_account_info(account_info_iter)?;
        let store_tokens_account = next_account_info(account_info_iter)?;
        let token_program = next_account_info(account_info_iter)?;
        {
            if *store_tokens_account.owner != spl_token::id() {
                return Err(ProgramError::IncorrectProgramId);
            }
            if *native_tokens_account.owner != spl_token::id() {
                return Err(ProgramError::IncorrectProgramId);
            }

            let (pda, _nonce) = Pubkey::find_program_address(&[b"store"], program_id);
            {
                let owner_change_ix = spl_token::instruction::set_authority(
                    token_program.key,
                    store_tokens_account.key,
                    Some(&pda),
                    spl_token::instruction::AuthorityType::AccountOwner,
                    owner.key,
                    &[&owner.key],
                )?;

                msg!("Calling the token program to transfer token account ownership...");
                invoke(
                    &owner_change_ix,
                    &[
                        store_tokens_account.clone(),
                        owner.clone(),
                        token_program.clone(),
                    ],
                )?;
            }
            {
                let owner_change_ix = spl_token::instruction::set_authority(
                    token_program.key,
                    native_tokens_account.key,
                    Some(&pda),
                    spl_token::instruction::AuthorityType::AccountOwner,
                    owner.key,
                    &[&owner.key],
                )?;

                msg!("Calling the token program to transfer token account ownership...");
                invoke(
                    &owner_change_ix,
                    &[
                        native_tokens_account.clone(),
                        owner.clone(),
                        token_program.clone(),
                    ],
                )?;
            }
        }
        {
            let rent = &Rent::from_account_info(next_account_info(account_info_iter)?)?;
            if !rent.is_exempt(store_account.lamports(), store_account.data_len()) {
                return Err(ProgramError::AccountNotRentExempt);
            }
            if store_account.owner != program_id {
                return Err(ProgramError::IncorrectProgramId);
            }
        }
        {
            let mut store_info = Store::unpack_unchecked(&store_account.data.borrow())?;
            if store_info.is_initialized() {
                return Err(ProgramError::AccountAlreadyInitialized);
            }

            store_info.is_initialized = true;
            store_info.price = price;
            store_info.owner_pubkey = *owner.key;
            store_info.native_tokens_to_auto_sell_pubkey = *native_tokens_account.key;
            store_info.store_tokens_to_auto_buy_pubkey = *store_tokens_account.key;

            Store::pack(store_info, &mut store_account.data.borrow_mut())?;
        }
        Ok(())
    }

    fn process_update_price(
        accounts: &[AccountInfo],
        price: u64,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let owner = next_account_info(account_info_iter)?;
        if !owner.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let store_account = next_account_info(account_info_iter)?;
        if store_account.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }

        {
            let mut store_info = Store::unpack_unchecked(&store_account.data.borrow())?;
            if !store_info.is_initialized() {
                return Err(ProgramError::UninitializedAccount);
            }
            if store_info.owner_pubkey != *owner.key {
                return Err(ProgramError::InvalidAccountData);
            }
            store_info.price = price;
            Store::pack(store_info, &mut store_account.data.borrow_mut())?;
        }

        Ok(())
    }

    fn process_buy(
        accounts: &[AccountInfo],
        amount: u64,
        price: u64,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let buyer = next_account_info(account_info_iter)?;
        if !buyer.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let store_account = next_account_info(account_info_iter)?;
        if store_account.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }
        let store_info = Store::unpack_unchecked(&store_account.data.borrow())?;
        if !store_info.is_initialized() {
            return Err(ProgramError::UninitializedAccount);
        }
        if price != store_info.price {
            return Err(StoreError::AccountPriceMismatch.into());
        }

        // store accounts
        let store_account_payment_tokens = next_account_info(account_info_iter)?;
        let store_account_store_tokens = next_account_info(account_info_iter)?;
        {
            if *store_account_payment_tokens.owner != spl_token::id() {
                return Err(ProgramError::IncorrectProgramId);
            }
            let test_info = spl_token::state::Account::unpack_unchecked(
                &store_account_payment_tokens.data.borrow(),
            )?;
            if test_info.owner != store_info.owner_pubkey {
                return Err(ProgramError::InvalidAccountData);
            }
        }

        // user accounts
        let user_account_payment_tokens = next_account_info(account_info_iter)?;
        let user_account_store_tokens = next_account_info(account_info_iter)?;

        let pda_account = next_account_info(account_info_iter)?;
        let token_program = next_account_info(account_info_iter)?;
        {
            // transfer payment tokens
            let transfer_to_initializer_ix = spl_token::instruction::transfer(
                token_program.key,
                user_account_payment_tokens.key,
                store_account_payment_tokens.key,
                buyer.key,
                &[&buyer.key],
                amount * price,
            )?;
            msg!("Calling the token program to transfer tokens to the store's owner...");
            invoke(
                &transfer_to_initializer_ix,
                &[
                    user_account_payment_tokens.clone(),
                    store_account_payment_tokens.clone(),
                    buyer.clone(),
                    token_program.clone(),
                ],
            )?;
        }
        {
            // transfer store tokens
            let (pda, nonce) = Pubkey::find_program_address(&[b"store"], program_id);
            let transfer_to_initializer_ix = spl_token::instruction::transfer(
                token_program.key,
                store_account_store_tokens.key,
                user_account_store_tokens.key,
                &pda,
                &[&pda],
                amount,
            )?;
            msg!("Calling the token program to transfer tokens to the user...");
            invoke_signed(
                &transfer_to_initializer_ix,
                &[
                    store_account_store_tokens.clone(),
                    user_account_store_tokens.clone(),
                    buyer.clone(),
                    pda_account.clone(),
                    token_program.clone(),
                ],
                &[&[&b"store"[..], &[nonce]]],
            )?;
        }

        Ok(())
    }

    fn process_sell(
        accounts: &[AccountInfo],
        amount: u64,
        price: u64,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let seller = next_account_info(account_info_iter)?;
        if !seller.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let store_account = next_account_info(account_info_iter)?;
        if store_account.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }

        let store_info = Store::unpack_unchecked(&store_account.data.borrow())?;
        if !store_info.is_initialized() {
            return Err(ProgramError::UninitializedAccount);
        }
        if price != store_info.price {
            return Err(StoreError::AccountPriceMismatch.into());
        }

        // store accounts
        let store_account_payment_tokens = next_account_info(account_info_iter)?;
        let store_account_store_tokens = next_account_info(account_info_iter)?;
        {
            if *store_account_store_tokens.owner != spl_token::id() {
                return Err(ProgramError::IncorrectProgramId);
            }
            let test_info = spl_token::state::Account::unpack_unchecked(
                &store_account_store_tokens.data.borrow(),
            )?;
            if test_info.owner != store_info.owner_pubkey {
                return Err(ProgramError::InvalidAccountData);
            }
        }

        // user accounts
        let user_account_payment_tokens = next_account_info(account_info_iter)?;
        let user_account_store_tokens = next_account_info(account_info_iter)?;

        let pda_account = next_account_info(account_info_iter)?;
        let token_program = next_account_info(account_info_iter)?;
        {
            // transfer store tokens
            let transfer_to_initializer_ix = spl_token::instruction::transfer(
                token_program.key,
                user_account_store_tokens.key,
                store_account_store_tokens.key,
                seller.key,
                &[&seller.key],
                amount,
            )?;
            msg!("Calling the token program to transfer tokens to the store owner...");
            invoke(
                &transfer_to_initializer_ix,
                &[
                    user_account_store_tokens.clone(),
                    store_account_store_tokens.clone(),
                    seller.clone(),
                    token_program.clone(),
                ],
            )?;
        }
        {
            // transfer payment tokens
            let (pda, nonce) = Pubkey::find_program_address(&[b"store"], program_id);
            let transfer_to_initializer_ix = spl_token::instruction::transfer(
                token_program.key,
                store_account_payment_tokens.key,
                user_account_payment_tokens.key,
                &pda,
                &[&pda],
                amount * price,
            )?;
            msg!("Calling the token program to transfer tokens to the user...");
            invoke_signed(
                &transfer_to_initializer_ix,
                &[
                    store_account_payment_tokens.clone(),
                    user_account_payment_tokens.clone(),
                    seller.clone(),
                    pda_account.clone(),
                    token_program.clone(),
                ],
                &[&[&b"store"[..], &[nonce]]],
            )?;
        }

        Ok(())
    }
}
