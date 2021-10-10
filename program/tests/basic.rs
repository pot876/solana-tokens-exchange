use solana_program::{clock::Epoch, program_pack::Pack, pubkey::Pubkey, system_instruction};
use solana_program_test::*;
use solana_sdk::{
    account::{Account, WritableAccount},
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};
use solana_test::{instruction, processor::Processor, state};
use spl_token::state::{Account as SplAccount, AccountState as SplAccountState};

#[tokio::test]
async fn test_one() {
    let program_id = Pubkey::new_unique();
    let (pda, _nonce) = Pubkey::find_program_address(&[b"store"], &program_id);

    let store_owner_keypair = Keypair::new();
    let store_payment_tokens_account_pubkey = Pubkey::new_unique();
    let store_store_tokens_account_pubkey = Pubkey::new_unique();
    let pay_to_store_payment_tokens_account_pubkey = Pubkey::new_unique();
    let pay_to_store_store_tokens_account_pubkey = Pubkey::new_unique();

    let user_keypair = Keypair::new();
    let user_payment_tokens_account_pubkey = Pubkey::new_unique();
    let user_store_tokens_account_pubkey = Pubkey::new_unique();

    let store_account_keypair = Keypair::new();
    let store_token_mint_pubkey = Pubkey::new_unique();
    let payment_token_mint_pubkey = Pubkey::new_unique();

    let mut program_test =
        ProgramTest::new("store_test", program_id, processor!(Processor::process));

    program_test.add_account(
        store_owner_keypair.pubkey(),
        Account {
            lamports: 1_000_000_000,
            ..Account::default()
        },
    );

    const INITIAL_TOKENS_AMOUNT: u64 = 1_000_000;
    {
        program_test.add_account(
            store_store_tokens_account_pubkey,
            create_token_account(
                store_owner_keypair.pubkey(),
                INITIAL_TOKENS_AMOUNT,
                store_token_mint_pubkey,
            ),
        );
        program_test.add_account(
            store_payment_tokens_account_pubkey,
            create_token_account(
                store_owner_keypair.pubkey(),
                INITIAL_TOKENS_AMOUNT,
                payment_token_mint_pubkey,
            ),
        );
        program_test.add_account(
            pay_to_store_store_tokens_account_pubkey,
            create_token_account(
                store_owner_keypair.pubkey(),
                INITIAL_TOKENS_AMOUNT,
                store_token_mint_pubkey,
            ),
        );
        program_test.add_account(
            pay_to_store_payment_tokens_account_pubkey,
            create_token_account(
                store_owner_keypair.pubkey(),
                INITIAL_TOKENS_AMOUNT,
                payment_token_mint_pubkey,
            ),
        );
        program_test.add_account(
            user_store_tokens_account_pubkey,
            create_token_account(
                user_keypair.pubkey(),
                INITIAL_TOKENS_AMOUNT,
                store_token_mint_pubkey,
            ),
        );
        program_test.add_account(
            user_payment_tokens_account_pubkey,
            create_token_account(
                user_keypair.pubkey(),
                INITIAL_TOKENS_AMOUNT,
                payment_token_mint_pubkey,
            ),
        );
    }

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    let rent = banks_client.get_rent().await.unwrap();

    {
        const INITIAL_PRICE: u64 = 123;
        let mut transaction = Transaction::new_with_payer(
            &[
                system_instruction::create_account(
                    &store_owner_keypair.pubkey(),
                    &store_account_keypair.pubkey(),
                    rent.minimum_balance(state::Store::LEN),
                    state::Store::LEN as u64,
                    &program_id,
                ),
                instruction::initialyze_account_instruction(
                    INITIAL_PRICE,
                    &program_id,
                    &store_owner_keypair.pubkey(),
                    &store_account_keypair.pubkey(),
                    &store_payment_tokens_account_pubkey,
                    &store_store_tokens_account_pubkey,
                    &spl_token::id(),
                )
                .unwrap(),
            ],
            Some(&payer.pubkey()),
        );

        transaction.sign(
            &[&payer, &store_account_keypair, &store_owner_keypair],
            recent_blockhash,
        );
        banks_client.process_transaction(transaction).await.unwrap();
        {
            assert_store_account(
                &mut banks_client,
                &store_account_keypair.pubkey(),
                Some(INITIAL_PRICE),
                Some(store_owner_keypair.pubkey()),
                &program_id,
            )
            .await;
            assert_spl_token_account(
                &mut banks_client,
                &store_payment_tokens_account_pubkey,
                Some(pda),
                None,
            )
            .await;
            assert_spl_token_account(
                &mut banks_client,
                &store_store_tokens_account_pubkey,
                Some(pda),
                None,
            )
            .await;
        }
    }
    const UPDATED_PRICE: u64 = 321;
    {
        let mut transaction = Transaction::new_with_payer(
            &[instruction::update_price_instruction(
                UPDATED_PRICE,
                &program_id,
                &store_owner_keypair.pubkey(),
                &store_account_keypair.pubkey(),
            )
            .unwrap()],
            Some(&payer.pubkey()),
        );

        transaction.sign(&[&payer, &store_owner_keypair], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();
        {
            assert_store_account(
                &mut banks_client,
                &store_account_keypair.pubkey(),
                Some(UPDATED_PRICE),
                Some(store_owner_keypair.pubkey()),
                &program_id,
            )
            .await;
        }
    }

    const BUY_AMOUNT: u64 = 3;
    {
        let mut transaction = Transaction::new_with_payer(
            &[instruction::buy_instruction(
                BUY_AMOUNT,
                UPDATED_PRICE,
                &program_id,
                &user_keypair.pubkey(),
                &store_account_keypair.pubkey(),
                &pay_to_store_payment_tokens_account_pubkey,
                &store_store_tokens_account_pubkey,
                &user_payment_tokens_account_pubkey,
                &user_store_tokens_account_pubkey,
                &pda,
                &spl_token::id(),
            )
            .unwrap()],
            Some(&payer.pubkey()),
        );

        transaction.sign(&[&payer, &user_keypair], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();
        {
            assert_spl_token_account(
                &mut banks_client,
                &user_payment_tokens_account_pubkey,
                Some(user_keypair.pubkey()),
                Some(INITIAL_TOKENS_AMOUNT - UPDATED_PRICE * BUY_AMOUNT),
            )
            .await;
            assert_spl_token_account(
                &mut banks_client,
                &user_store_tokens_account_pubkey,
                Some(user_keypair.pubkey()),
                Some(INITIAL_TOKENS_AMOUNT + BUY_AMOUNT),
            )
            .await;
            assert_spl_token_account(
                &mut banks_client,
                &pay_to_store_payment_tokens_account_pubkey,
                Some(store_owner_keypair.pubkey()),
                Some(INITIAL_TOKENS_AMOUNT + UPDATED_PRICE * BUY_AMOUNT),
            )
            .await;
            assert_spl_token_account(
                &mut banks_client,
                &store_store_tokens_account_pubkey,
                Some(pda),
                Some(INITIAL_TOKENS_AMOUNT - BUY_AMOUNT),
            )
            .await;
        }
    }
    const SELL_AMOUNT: u64 = 6;
    {
        let mut transaction = Transaction::new_with_payer(
            &[instruction::sell_instruction(
                SELL_AMOUNT,
                UPDATED_PRICE,
                &program_id,
                &user_keypair.pubkey(),
                &store_account_keypair.pubkey(),
                &store_payment_tokens_account_pubkey,
                &pay_to_store_store_tokens_account_pubkey,
                &user_payment_tokens_account_pubkey,
                &user_store_tokens_account_pubkey,
                &pda,
                &spl_token::id(),
            )
            .unwrap()],
            Some(&payer.pubkey()),
        );

        transaction.sign(&[&payer, &user_keypair], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();
        {
            assert_spl_token_account(
                &mut banks_client,
                &user_payment_tokens_account_pubkey,
                Some(user_keypair.pubkey()),
                Some(
                    INITIAL_TOKENS_AMOUNT - UPDATED_PRICE * BUY_AMOUNT
                        + UPDATED_PRICE * SELL_AMOUNT,
                ),
            )
            .await;
            assert_spl_token_account(
                &mut banks_client,
                &user_store_tokens_account_pubkey,
                Some(user_keypair.pubkey()),
                Some(INITIAL_TOKENS_AMOUNT + BUY_AMOUNT - SELL_AMOUNT),
            )
            .await;
            assert_spl_token_account(
                &mut banks_client,
                &store_payment_tokens_account_pubkey,
                Some(pda),
                Some(INITIAL_TOKENS_AMOUNT - UPDATED_PRICE * SELL_AMOUNT),
            )
            .await;
            assert_spl_token_account(
                &mut banks_client,
                &pay_to_store_store_tokens_account_pubkey,
                Some(store_owner_keypair.pubkey()),
                Some(INITIAL_TOKENS_AMOUNT + SELL_AMOUNT),
            )
            .await;
        }
    }
}

async fn assert_spl_token_account(
    banks_client: &mut BanksClient,
    account_pubkey: &Pubkey,
    owner: Option<Pubkey>,
    amount: Option<u64>,
) {
    let a = banks_client
        .get_account(*account_pubkey)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(a.owner, spl_token::ID);

    let sa = SplAccount::unpack_unchecked(&a.data).unwrap();
    if let Some(owner) = owner {
        assert_eq!(sa.owner, owner);
    }
    if let Some(amount) = amount {
        assert_eq!(sa.amount, amount);
    }
}
async fn assert_store_account(
    banks_client: &mut BanksClient,
    account_pubkey: &Pubkey,
    price: Option<u64>,
    owner: Option<Pubkey>,
    store_program_id: &Pubkey,
) {
    let a = banks_client
        .get_account(*account_pubkey)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(a.owner, *store_program_id);

    let sa = state::Store::unpack_unchecked(&a.data).unwrap();
    if let Some(price) = price {
        assert_eq!(sa.price, price);
    }
    if let Some(owner) = owner {
        assert_eq!(sa.owner_pubkey, owner);
    }
}

fn create_token_account(owner: Pubkey, amount: u64, mint: Pubkey) -> Account {
    const DEFAULT_LAMPORTS_AMOUNT: u64 = 10000000000;

    let mut store_tokens_account_vec = vec![0u8; SplAccount::LEN];

    let store_tokens_account_data = SplAccount {
        mint: mint,
        owner: owner,
        amount: amount,
        state: SplAccountState::Initialized,
        ..SplAccount::default()
    };
    Pack::pack(store_tokens_account_data, &mut store_tokens_account_vec).unwrap();

    let store_tokens_account = Account::create(
        DEFAULT_LAMPORTS_AMOUNT,
        store_tokens_account_vec,
        spl_token::id(),
        false,
        Epoch::default(),
    );
    store_tokens_account
}

#[allow(dead_code)]
async fn print_acc(banks_client: &mut BanksClient, pubkey: Pubkey, store_program_id: Pubkey) {
    let a = banks_client.get_account(pubkey).await.unwrap().unwrap();
    println!("{:?}", a);

    if a.owner == spl_token::id() {
        let sa = SplAccount::unpack_unchecked(&a.data).unwrap();
        println!("{:?}", sa);
    }
    if a.owner == store_program_id {
        let sa = state::Store::unpack_unchecked(&a.data).unwrap();
        println!("{:?}", sa);
    }
}
