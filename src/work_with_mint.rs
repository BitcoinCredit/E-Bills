use crate::{read_identity_from_file, read_keys_from_bill_file, Identity};
use moksha_core::amount::Amount;
use moksha_core::primitives::{
    CurrencyUnit, PaymentMethod, PostMintQuoteBitcreditResponse, PostRequestToMintBitcreditResponse,
};
use moksha_wallet::http::CrossPlatformHttpClient;
use moksha_wallet::localstore::sqlite::SqliteLocalStore;
use moksha_wallet::wallet::Wallet;
use std::path::PathBuf;
use url::Url;

// pub fn mint_with_handle(handle: Handle, amount: u64, bill_id: String) {
//     block_on(async {
//         handle
//             .spawn(async {
//                 let dir = PathBuf::from("./data/wallet".to_string());
//                 fs::create_dir_all(dir.clone()).unwrap();
//                 let db_path = dir.join("wallet.db").to_str().unwrap().to_string();
//
//                 let localstore = SqliteLocalStore::with_path(db_path.clone())
//                     .await
//                     .expect("Cannot parse local store");
//                 let client = HttpClient::default();
//                 let mint_url = Url::parse("http://127.0.0.1:3338").expect("Invalid url");
//
//                 // block_on(async {
//                 let wallet = WalletBuilder::default()
//                     .with_client(client)
//                     .with_localstore(localstore)
//                     .with_mint_url(mint_url)
//                     .build()
//                     .await
//                     .expect("Could not create wallet");
//
//                 loop {
//                     tokio::time::sleep_until(
//                         tokio::time::Instant::now() + std::time::Duration::from_millis(1_000),
//                     )
//                     .await;
//
//                     let req = wallet
//                         .get_mint_payment_request(50)
//                         .await
//                         .expect("Cannot get mint payment request");
//
//                     let mint_result = wallet.mint_tokens(50.into(), req.hash.clone()).await;
//
//                     match mint_result {
//                         Ok(_) => {
//                             println!(
//                                 "Tokens minted successfully.\nNew balance {} sats",
//                                 wallet.get_balance().await.unwrap()
//                             );
//                             break;
//                         }
//                         Err(moksha_wallet::error::MokshaWalletError::InvoiceNotPaidYet(_, _)) => {
//                             continue;
//                         }
//                         Err(e) => {
//                             println!("General Error: {}", e);
//                             break;
//                         }
//                     }
//                 }
//             })
//             .await
//             .expect("Task mint spawned in Tokio executor panicked")
//     })
// }

#[tokio::main]
pub async fn mint(
    amount: u64,
    bill_id: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let dir = PathBuf::from("./data/wallet".to_string());
    let db_path = dir.join("wallet.db").to_str().unwrap().to_string();
    let localstore = SqliteLocalStore::with_path(db_path.clone())
        .await
        .expect("Cannot parse local store");

    //TODO change to some conf in settings
    let mint_url = Url::parse("http://127.0.0.1:3338").expect("Invalid url");

    let identity: Identity = read_identity_from_file();
    let bitcoin_key = identity.bitcoin_public_key.clone();

    let wallet: Wallet<_, CrossPlatformHttpClient> = Wallet::builder()
        .with_localstore(localstore)
        .build()
        .await
        .expect("Could not create wallet");

    loop {
        tokio::time::sleep_until(
            tokio::time::Instant::now() + std::time::Duration::from_millis(1_000),
        )
        .await;

        let mint_url = Url::parse("http://127.0.0.1:3338").expect("Invalid url");

        let req = wallet
            .get_mint_quote(&mint_url, Amount::from(amount), CurrencyUnit::Sat)
            .await
            .expect("Cannot get mint payment request");

        let wallet_keysets = wallet
            .add_mint_keysets(&Url::parse("https://mint.mutinynet.moksha.cash")?)
            .await?;
        let wallet_keyset = wallet_keysets.first().unwrap();

        let result = wallet
            .mint_tokens(
                wallet_keyset,
                &PaymentMethod::Bolt11,
                amount.into(),
                req.quote,
            )
            .await;

        match result {
            Ok(_) => {
                println!(
                    "Tokens minted successfully.\nNew balance {} sats",
                    wallet.get_balance().await.unwrap()
                );
                break;
            }
            Err(moksha_wallet::error::MokshaWalletError::InvoiceNotPaidYet(_, _)) => {
                continue;
            }
            Err(e) => {
                println!("General Error: {}", e);
                break;
            }
        }
    }

    Ok(())
}

#[tokio::main]
pub async fn mint_bitcredit(amount: u64, bill_id: String) -> PostMintQuoteBitcreditResponse {
    let dir = PathBuf::from("./data/wallet".to_string());
    let db_path = dir.join("wallet.db").to_str().unwrap().to_string();
    let localstore = SqliteLocalStore::with_path(db_path.clone())
        .await
        .expect("Cannot parse local store");

    let mint_url = Url::parse("http://127.0.0.1:3338").expect("Invalid url");

    let identity: Identity = read_identity_from_file();
    let bitcoin_key = identity.bitcoin_public_key.clone();

    let wallet: Wallet<_, CrossPlatformHttpClient> = Wallet::builder()
        .with_localstore(localstore)
        .build()
        .await
        .expect("Could not create wallet");

    let req = wallet.create_quote_bitcredit(&mint_url, amount, bill_id);

    // let wallet_keysets = wallet
    //     .add_mint_keysets(&Url::parse("https://mint.mutinynet.moksha.cash")?)
    //     .await?;
    // let wallet_keyset = wallet_keysets.first().unwrap();

    // let result = wallet
    //     .mint_tokens(wallet_keyset, &PaymentMethod::Bolt11, amount.into(), req.quote)
    //     .await;

    // let quote = req.quote;

    req.await.unwrap()
}

#[tokio::main]
pub async fn request_to_mint_bitcredit(bill_id: String) -> PostRequestToMintBitcreditResponse {
    let dir = PathBuf::from("./data/wallet".to_string());
    let db_path = dir.join("wallet.db").to_str().unwrap().to_string();
    let localstore = SqliteLocalStore::with_path(db_path.clone())
        .await
        .expect("Cannot parse local store");

    let mint_url = Url::parse("http://127.0.0.1:3338").expect("Invalid url");

    let wallet: Wallet<_, CrossPlatformHttpClient> = Wallet::builder()
        .with_localstore(localstore)
        .build()
        .await
        .expect("Could not create wallet");

    let bill_keys = read_keys_from_bill_file(&bill_id);
    let bill_private_key = bill_keys.private_key_pem;

    let req = wallet.send_request_to_mint_bitcredit(&mint_url, bill_id, bill_private_key);

    req.await.unwrap()
}
