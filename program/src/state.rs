use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Store {
    pub is_initialized: bool,

    /// amount native tokens per store token
    pub price: u64,
    pub owner_pubkey: Pubkey,

    /// account to take tokens when sell
    pub native_tokens_to_auto_sell_pubkey: Pubkey,
    /// account to take tokens when buy
    pub store_tokens_to_auto_buy_pubkey: Pubkey,
}

impl Sealed for Store {}

impl IsInitialized for Store {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for Store {
    const LEN: usize = 1 + 8 + 32 + 32 + 32;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, Store::LEN];
        let (is_initialized, price, initializer_pubkey, native_tokens_pubkey, store_tokens_pubkey) =
            array_refs![src, 1, 8, 32, 32, 32];
        let is_initialized = match is_initialized {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };

        Ok(Store {
            is_initialized,
            price: u64::from_le_bytes(*price),
            owner_pubkey: Pubkey::new_from_array(*initializer_pubkey),
            native_tokens_to_auto_sell_pubkey: Pubkey::new_from_array(*native_tokens_pubkey),
            store_tokens_to_auto_buy_pubkey: Pubkey::new_from_array(*store_tokens_pubkey),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, Store::LEN];
        let (
            is_initialized_dst,
            price_dst,
            initializer_pubkey_dst,
            native_tokens_pubkey_dst,
            store_tokens_pubkey_dst,
        ) = mut_array_refs![dst, 1, 8, 32, 32, 32];

        let Store {
            is_initialized,
            price,
            owner_pubkey,
            native_tokens_to_auto_sell_pubkey,
            store_tokens_to_auto_buy_pubkey,
        } = self;

        is_initialized_dst[0] = *is_initialized as u8;
        *price_dst = price.to_le_bytes();
        initializer_pubkey_dst.copy_from_slice(owner_pubkey.as_ref());
        native_tokens_pubkey_dst.copy_from_slice(native_tokens_to_auto_sell_pubkey.as_ref());
        store_tokens_pubkey_dst.copy_from_slice(store_tokens_to_auto_buy_pubkey.as_ref());
    }
}
