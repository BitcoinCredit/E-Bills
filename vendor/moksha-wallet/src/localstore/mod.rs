use async_trait::async_trait;
use moksha_core::model::{Keysets, Proofs};
use serde::{Deserialize, Serialize};

use crate::error::MokshaWalletError;

#[cfg(not(target_arch = "wasm32"))]
pub mod sqlite;

pub mod memory;

#[cfg(target_arch = "wasm32")]
pub mod rexie;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WalletKeyset {
    pub id: String,
    pub mint_url: String,
}

impl WalletKeyset {
    pub fn new(id: String, mint_url: String) -> Self {
        Self {
            id,
            mint_url,
        }
    }
}


#[async_trait(?Send)]
pub trait LocalStore {
    async fn delete_proofs(&self, proofs: &Proofs) -> Result<(), MokshaWalletError>;
    async fn add_proofs(&self, proofs: &Proofs) -> Result<(), MokshaWalletError>;
    async fn get_proofs(&self) -> Result<Proofs, MokshaWalletError>;

    async fn get_keysets(&self) -> Result<Vec<WalletKeyset>, MokshaWalletError>;
    async fn add_keyset(&self, keyset: &WalletKeyset) -> Result<(), MokshaWalletError>;

    async fn migrate(&self);
}
