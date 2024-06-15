use std::str::FromStr;
use std::thread;
use std::thread::sleep;
use std::time::Duration;

use borsh::{BorshDeserialize, BorshSerialize};
use chrono::prelude::*;
use chrono::Days;
use futures::executor::block_on;
use log::{info, warn};
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::pkey::Private;
use openssl::rsa::Rsa;
use openssl::sha::Sha256;
use openssl::sign::{Signer, Verifier};
use rocket::form::validate::Contains;
use rocket::route::Cloneable;
use serde::{Deserialize, Serialize};

use crate::blockchain::OperationCode::{Endorse, Mint, Sell};
use crate::constants::{BILLS_FOLDER_PATH, USEDNET};
use crate::{
    api, bill_from_byte_array, decrypt_bytes, encrypt_bytes, private_key_from_pem_u8,
    public_key_from_pem_u8, read_bill_from_file, read_bill_with_chain_from_file,
    read_keys_from_bill_file, BitcreditBill, IdentityPublicData, IdentityWithAll,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChainToReturn {
    pub blocks: Vec<BlockToReturn>,
}

impl ChainToReturn {
    pub fn new(chain: Chain) -> Self {
        let mut blocks: Vec<BlockToReturn> = Vec::new();
        let bill = chain.get_first_version_bill();
        for block in chain.blocks {
            blocks.push(BlockToReturn::new(block, bill.clone()));
        }
        Self { blocks }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Chain {
    pub blocks: Vec<Block>,
}

impl Chain {
    pub fn new(first_block: Block) -> Self {
        let mut blocks = Vec::new();
        blocks.push(first_block);

        Self { blocks }
    }

    pub fn read_chain_from_file(bill_name: &String) -> Self {
        let input_path = BILLS_FOLDER_PATH.to_string() + "/" + bill_name.as_str() + ".json";
        let blockchain_from_file = std::fs::read(input_path.clone()).expect("file not found");
        serde_json::from_slice(blockchain_from_file.as_slice()).unwrap()
    }

    pub fn write_chain_to_file(&self, bill_name: &String) {
        let output_path = BILLS_FOLDER_PATH.to_string() + "/" + bill_name.as_str() + ".json";
        std::fs::write(
            output_path.clone(),
            serde_json::to_string_pretty(&self).unwrap(),
        )
        .unwrap();
    }

    pub fn is_chain_valid(&self) -> bool {
        for i in 0..self.blocks.len() {
            if i == 0 {
                continue;
            }
            let first: &Block = &self.blocks[i - 1];
            let second: &Block = &self.blocks[i];
            if !is_block_valid(second, first) {
                return false;
            }
        }
        true
    }

    pub fn try_add_block(&mut self, block: Block) -> bool {
        let latest_block = self.blocks.last().expect("there is at least one block");
        if is_block_valid(&block, latest_block) {
            self.blocks.push(block);
            return true;
        } else {
            error!("could not add block - invalid");
            return false;
        }
    }

    pub fn get_latest_block(&self) -> &Block {
        self.blocks.last().expect("there is at least one block")
    }

    pub fn get_first_block(&self) -> &Block {
        self.blocks.first().expect("there is at least one block")
    }

    pub fn get_last_version_block_with_operation_code(
        &self,
        operation_code: OperationCode,
    ) -> &Block {
        let mut last_version_block: &Block = &self.get_first_block();
        for block in &self.blocks {
            if block.operation_code == operation_code {
                last_version_block = block;
            }
        }
        last_version_block
    }

    pub fn exist_block_with_operation_code(&self, operation_code: OperationCode) -> bool {
        let mut exist_block_with_operation_code = false;
        for block in &self.blocks {
            if block.operation_code == operation_code {
                exist_block_with_operation_code = true;
            }
        }
        exist_block_with_operation_code
    }

    pub fn get_last_version_bill(&self) -> BitcreditBill {
        let first_block = self.get_first_block();

        let bill_keys = read_keys_from_bill_file(&first_block.bill_name);
        let key: Rsa<Private> =
            Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
        let bytes = hex::decode(first_block.data.clone()).unwrap();
        let decrypted_bytes = decrypt_bytes(&bytes, &key);
        let bill_first_version: BitcreditBill = bill_from_byte_array(&decrypted_bytes);

        let mut last_endorsee = IdentityPublicData {
            peer_id: "".to_string(),
            name: "".to_string(),
            company: "".to_string(),
            bitcoin_public_key: "".to_string(),
            postal_address: "".to_string(),
            email: "".to_string(),
            rsa_public_key_pem: "".to_string(),
        };

        if self.blocks.len() > 1
            && (self.exist_block_with_operation_code(Endorse.clone())
                || self.exist_block_with_operation_code(Sell.clone())
                || self.exist_block_with_operation_code(Mint.clone()))
        {
            let last_version_block_endorse =
                self.get_last_version_block_with_operation_code(Endorse);
            let last_version_block_mint = self.get_last_version_block_with_operation_code(Mint);
            let last_version_block_sell = self.get_last_version_block_with_operation_code(Sell);
            let last_block = self.get_latest_block();

            let paid = Self::check_if_last_sell_block_is_paid(self);

            if (last_version_block_endorse.id < last_version_block_sell.id)
                && (last_version_block_mint.id < last_version_block_sell.id)
                && ((last_block.id > last_version_block_sell.id) || paid)
            {
                let bill_keys = read_keys_from_bill_file(&last_version_block_sell.bill_name);
                let key: Rsa<Private> =
                    Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
                let bytes = hex::decode(last_version_block_sell.data.clone()).unwrap();
                let decrypted_bytes = decrypt_bytes(&bytes, &key);
                let block_data_decrypted = String::from_utf8(decrypted_bytes).unwrap();

                let mut part_without_sold_to = block_data_decrypted
                    .split("Sold to ")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .unwrap()
                    .to_string();

                let mut part_with_buyer = part_without_sold_to
                    .split(" sold by ")
                    .collect::<Vec<&str>>()
                    .get(0)
                    .unwrap()
                    .to_string();

                let mut part_with_seller_and_amount = part_without_sold_to
                    .clone()
                    .split(" sold by ")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .unwrap()
                    .to_string();

                let mut amount: u64 = part_with_seller_and_amount
                    .clone()
                    .split(" amount: ")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .unwrap()
                    .to_string()
                    .parse()
                    .unwrap();

                let mut part_with_seller = part_with_seller_and_amount
                    .clone()
                    .split(" amount: ")
                    .collect::<Vec<&str>>()
                    .get(0)
                    .unwrap()
                    .to_string();

                let buyer_bill_u8 = hex::decode(part_with_buyer).unwrap();
                let buyer_bill: IdentityPublicData =
                    serde_json::from_slice(&buyer_bill_u8).unwrap();

                let seller_bill_u8 = hex::decode(part_with_seller).unwrap();
                let seller_bill: IdentityPublicData =
                    serde_json::from_slice(&seller_bill_u8).unwrap();

                last_endorsee = buyer_bill.clone();
            } else if self.exist_block_with_operation_code(Endorse.clone())
                && (last_version_block_endorse.id > last_version_block_mint.id)
            {
                let bill_keys = read_keys_from_bill_file(&last_version_block_endorse.bill_name);
                let key: Rsa<Private> =
                    Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
                let bytes = hex::decode(last_version_block_endorse.data.clone()).unwrap();
                let decrypted_bytes = decrypt_bytes(&bytes, &key);
                let block_data_decrypted = String::from_utf8(decrypted_bytes).unwrap();

                let mut part_with_endorsee = block_data_decrypted
                    .split("Endorsed to ")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .unwrap()
                    .to_string();

                part_with_endorsee = part_with_endorsee
                    .split(" endorsed by ")
                    .collect::<Vec<&str>>()
                    .get(0)
                    .unwrap()
                    .to_string();

                let endorsee = hex::decode(part_with_endorsee).unwrap();
                last_endorsee = serde_json::from_slice(&endorsee).unwrap();
            } else if self.exist_block_with_operation_code(Mint.clone())
                && (last_version_block_mint.id > last_version_block_endorse.id)
            {
                let bill_keys = read_keys_from_bill_file(&last_version_block_mint.bill_name);
                let key: Rsa<Private> =
                    Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
                let bytes = hex::decode(last_version_block_mint.data.clone()).unwrap();
                let decrypted_bytes = decrypt_bytes(&bytes, &key);
                let block_data_decrypted = String::from_utf8(decrypted_bytes).unwrap();

                let mut part_with_mint = block_data_decrypted
                    .split("Endorsed to ")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .unwrap()
                    .to_string();

                part_with_mint = part_with_mint
                    .split(" endorsed by ")
                    .collect::<Vec<&str>>()
                    .get(0)
                    .unwrap()
                    .to_string();

                let mint = hex::decode(part_with_mint).unwrap();
                last_endorsee = serde_json::from_slice(&mint).unwrap();
            }
        }

        let mut payee = bill_first_version.payee.clone();

        if !last_endorsee.peer_id.is_empty() {
            payee = last_endorsee.clone();
        }

        let bill = BitcreditBill {
            name: bill_first_version.name,
            to_payee: bill_first_version.to_payee,
            bill_jurisdiction: bill_first_version.bill_jurisdiction,
            timestamp_at_drawing: bill_first_version.timestamp_at_drawing,
            drawee: bill_first_version.drawee,
            drawer: bill_first_version.drawer,
            payee: payee.clone(),
            endorsee: last_endorsee.clone(),
            place_of_drawing: bill_first_version.place_of_drawing,
            currency_code: bill_first_version.currency_code,
            amount_numbers: bill_first_version.amount_numbers,
            amounts_letters: bill_first_version.amounts_letters,
            maturity_date: bill_first_version.maturity_date,
            date_of_issue: bill_first_version.date_of_issue,
            compounding_interest_rate: bill_first_version.compounding_interest_rate,
            type_of_interest_calculation: bill_first_version.type_of_interest_calculation,
            place_of_payment: bill_first_version.place_of_payment,
            public_key: bill_first_version.public_key,
            private_key: bill_first_version.private_key,
            language: bill_first_version.language,
        };

        bill
    }

    pub fn waiting_for_payment(
        &self,
    ) -> (bool, IdentityPublicData, IdentityPublicData, String, u64) {
        let last_block = self.get_latest_block();
        let last_version_block_sell = self.get_last_version_block_with_operation_code(Sell);
        let identity_buyer = IdentityPublicData::new_empty();
        let identity_seller = IdentityPublicData::new_empty();

        if self.exist_block_with_operation_code(Sell.clone())
            && last_block.id == last_version_block_sell.id
        {
            let bill_keys = read_keys_from_bill_file(&last_version_block_sell.bill_name);
            let key: Rsa<Private> =
                Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
            let bytes = hex::decode(last_version_block_sell.data.clone()).unwrap();
            let decrypted_bytes = decrypt_bytes(&bytes, &key);
            let block_data_decrypted = String::from_utf8(decrypted_bytes).unwrap();

            let mut part_without_sold_to = block_data_decrypted
                .split("Sold to ")
                .collect::<Vec<&str>>()
                .get(1)
                .unwrap()
                .to_string();

            let mut part_with_buyer = part_without_sold_to
                .split(" sold by ")
                .collect::<Vec<&str>>()
                .get(0)
                .unwrap()
                .to_string();

            let mut part_with_seller_and_amount = part_without_sold_to
                .clone()
                .split(" sold by ")
                .collect::<Vec<&str>>()
                .get(1)
                .unwrap()
                .to_string();

            let mut amount: u64 = part_with_seller_and_amount
                .clone()
                .split(" amount: ")
                .collect::<Vec<&str>>()
                .get(1)
                .unwrap()
                .to_string()
                .parse()
                .unwrap();

            let mut part_with_seller = part_with_seller_and_amount
                .clone()
                .split(" amount: ")
                .collect::<Vec<&str>>()
                .get(0)
                .unwrap()
                .to_string();

            let buyer_bill_u8 = hex::decode(part_with_buyer).unwrap();
            let buyer_bill: IdentityPublicData = serde_json::from_slice(&buyer_bill_u8).unwrap();
            let identity_buyer = buyer_bill;

            let seller_bill_u8 = hex::decode(part_with_seller).unwrap();
            let seller_bill: IdentityPublicData = serde_json::from_slice(&seller_bill_u8).unwrap();
            let identity_seller = seller_bill;

            let bill = self.get_first_version_bill();

            let address_to_pay =
                Self::get_address_to_pay_for_block_sell(last_version_block_sell.clone(), bill);

            let address_to_pay_for_async = address_to_pay.clone();

            let paid = thread::spawn(move || Self::check_if_paid(address_to_pay_for_async, amount))
                .join()
                .expect("Thread panicked");

            (
                !paid,
                identity_buyer,
                identity_seller,
                address_to_pay,
                amount,
            )
        } else {
            (false, identity_buyer, identity_seller, String::new(), 0)
        }
    }

    pub async fn check_if_payment_deadline_has_passed(&self) -> bool {
        if self.exist_block_with_operation_code(crate::blockchain::OperationCode::Sell) {
            let last_version_block_sell = self
                .get_last_version_block_with_operation_code(crate::blockchain::OperationCode::Sell);

            let timestamp = last_version_block_sell.timestamp;

            let payment_deadline_has_passed = Self::payment_deadline_has_passed(timestamp, 2).await;

            payment_deadline_has_passed
        } else {
            false
        }
    }

    async fn payment_deadline_has_passed(timestamp: i64, day: i32) -> bool {
        let period: i64 = (86400 * day) as i64;
        let current_timestamp = api::TimeApi::get_atomic_time().await.timestamp;
        let diference = current_timestamp - timestamp;
        if diference > period {
            true
        } else {
            false
        }
    }

    fn check_if_last_sell_block_is_paid(&self) -> bool {
        if self.exist_block_with_operation_code(Sell) {
            let last_version_block_sell = self.get_last_version_block_with_operation_code(Sell);

            let bill_keys = read_keys_from_bill_file(&last_version_block_sell.bill_name);
            let key: Rsa<Private> =
                Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
            let bytes = hex::decode(last_version_block_sell.data.clone()).unwrap();
            let decrypted_bytes = decrypt_bytes(&bytes, &key);
            let block_data_decrypted = String::from_utf8(decrypted_bytes).unwrap();

            let mut part_without_sold_to = block_data_decrypted
                .split("Sold to ")
                .collect::<Vec<&str>>()
                .get(1)
                .unwrap()
                .to_string();

            let mut part_with_buyer = part_without_sold_to
                .split(" sold by ")
                .collect::<Vec<&str>>()
                .get(0)
                .unwrap()
                .to_string();

            let mut part_with_seller_and_amount = part_without_sold_to
                .clone()
                .split(" sold by ")
                .collect::<Vec<&str>>()
                .get(1)
                .unwrap()
                .to_string();

            let mut amount: u64 = part_with_seller_and_amount
                .clone()
                .split(" amount: ")
                .collect::<Vec<&str>>()
                .get(1)
                .unwrap()
                .to_string()
                .parse()
                .unwrap();

            let mut part_with_seller = part_with_seller_and_amount
                .clone()
                .split(" amount: ")
                .collect::<Vec<&str>>()
                .get(0)
                .unwrap()
                .to_string();

            let buyer_bill_u8 = hex::decode(part_with_buyer).unwrap();
            let buyer_bill: IdentityPublicData = serde_json::from_slice(&buyer_bill_u8).unwrap();

            let seller_bill_u8 = hex::decode(part_with_seller).unwrap();
            let seller_bill: IdentityPublicData = serde_json::from_slice(&seller_bill_u8).unwrap();

            let bill = self.get_first_version_bill();

            let address_to_pay =
                Self::get_address_to_pay_for_block_sell(last_version_block_sell.clone(), bill);

            let paid = thread::spawn(move || Self::check_if_paid(address_to_pay, amount))
                .join()
                .expect("Thread panicked");

            paid
        } else {
            false
        }
    }

    fn get_address_to_pay_for_block_sell(
        last_version_block_sell: Block,
        bill: BitcreditBill,
    ) -> String {
        let public_key_bill = bitcoin::PublicKey::from_str(&bill.public_key).unwrap();

        let bill_keys = read_keys_from_bill_file(&last_version_block_sell.bill_name);
        let key: Rsa<Private> =
            Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
        let bytes = hex::decode(last_version_block_sell.data.clone()).unwrap();
        let decrypted_bytes = decrypt_bytes(&bytes, &key);
        let block_data_decrypted = String::from_utf8(decrypted_bytes).unwrap();

        let mut part_without_sold_to = block_data_decrypted
            .split("Sold to ")
            .collect::<Vec<&str>>()
            .get(1)
            .unwrap()
            .to_string();

        let mut part_with_buyer = part_without_sold_to
            .split(" sold by ")
            .collect::<Vec<&str>>()
            .get(0)
            .unwrap()
            .to_string();

        let mut part_with_seller_and_amount = part_without_sold_to
            .clone()
            .split(" sold by ")
            .collect::<Vec<&str>>()
            .get(1)
            .unwrap()
            .to_string();

        let mut amount: u64 = part_with_seller_and_amount
            .clone()
            .split(" amount: ")
            .collect::<Vec<&str>>()
            .get(1)
            .unwrap()
            .to_string()
            .parse()
            .unwrap();

        let part_with_seller = part_with_seller_and_amount
            .clone()
            .split(" amount: ")
            .collect::<Vec<&str>>()
            .get(0)
            .unwrap()
            .to_string();

        let seller_bill_u8 = hex::decode(part_with_seller).unwrap();
        let seller_bill: IdentityPublicData = serde_json::from_slice(&seller_bill_u8).unwrap();

        let public_key_seller = seller_bill.bitcoin_public_key;
        let public_key_bill_seller = bitcoin::PublicKey::from_str(&public_key_seller).unwrap();

        let public_key_bill = public_key_bill
            .inner
            .combine(&public_key_bill_seller.inner)
            .unwrap();
        let pub_key_bill = bitcoin::PublicKey::new(public_key_bill);
        let address_to_pay = bitcoin::Address::p2pkh(&pub_key_bill, USEDNET).to_string();

        address_to_pay
    }

    #[tokio::main]
    async fn check_if_paid(address: String, amount: u64) -> bool {
        let info_about_address = api::AddressInfo::get_testnet_address_info(address.clone()).await;
        let received_summ = info_about_address.chain_stats.funded_txo_sum;
        let spent_summ = info_about_address.chain_stats.spent_txo_sum;
        // let received_summ_mempool = info_about_address.mempool_stats.funded_txo_sum;
        let spent_summ_mempool = info_about_address.mempool_stats.spent_txo_sum;
        return if amount.eq(&(received_summ + spent_summ
                // + received_summ_mempool
                + spent_summ_mempool))
        {
            true
        } else {
            false
        };
    }

    fn get_first_version_bill(&self) -> BitcreditBill {
        let first_block_data = &self.get_first_block();
        let bill_keys = read_keys_from_bill_file(&first_block_data.bill_name);
        let key: Rsa<Private> =
            Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
        let bytes = hex::decode(first_block_data.data.clone()).unwrap();
        let decrypted_bytes = decrypt_bytes(&bytes, &key);
        let bill_first_version: BitcreditBill = bill_from_byte_array(&decrypted_bytes);
        bill_first_version
    }

    pub fn get_block_by_id(&self, id: u64) -> Block {
        let mut block = self.get_first_block().clone();
        for b in &self.blocks {
            if b.id == id {
                block = b.clone();
            }
        }
        block
    }

    pub fn compare_chain(&mut self, other_chain: Chain, bill_name: &String) {
        let local_chain_last_id = self.get_latest_block().id.clone();
        let other_chain_last_id = other_chain.get_latest_block().id.clone();
        if local_chain_last_id.eq(&other_chain_last_id) {
            return;
        } else if local_chain_last_id > other_chain_last_id {
            return;
        } else {
            let difference_in_id = other_chain_last_id - local_chain_last_id;
            for block_id in 1..difference_in_id + 1 {
                let block = other_chain.get_block_by_id(local_chain_last_id.clone() + block_id);
                let try_add_block = self.try_add_block(block);
                if try_add_block && self.is_chain_valid() {
                    self.write_chain_to_file(&bill_name);
                } else {
                    return;
                }
            }
        }
    }

    pub fn get_all_nodes_from_bill(&self) -> Vec<String> {
        let mut nodes: Vec<String> = Vec::new();

        for block in &self.blocks {
            let bill = self.get_first_version_bill();
            let mut nodes_in_block = block.get_nodes_from_block(bill);
            for node in nodes_in_block {
                if !node.is_empty() && !nodes.contains(&node) {
                    nodes.push(node);
                }
            }
        }
        nodes
    }

    pub fn get_bill_history(&self) -> Vec<BlockForHistory> {
        let mut history: Vec<BlockForHistory> = Vec::new();

        for block in &self.blocks {
            let bill = self.get_first_version_bill();
            let mut line = block.get_history_label(bill);
            history.push(BlockForHistory {
                id: block.id.clone(),
                text: line,
                bill_name: block.bill_name.clone(),
            });
        }
        history
    }

    pub fn get_drawer(&self) -> IdentityPublicData {
        let drawer: IdentityPublicData;
        let bill = self.get_first_version_bill();
        if !bill.drawer.name.is_empty() {
            drawer = bill.drawer.clone();
        } else if bill.to_payee {
            drawer = bill.payee.clone();
        } else {
            drawer = bill.drawee.clone();
        }
        drawer
    }

    pub fn bill_contain_node(&self, request_node_id: String) -> bool {
        for block in &self.blocks {
            match block.operation_code {
                OperationCode::Issue => {
                    let bill = self.get_first_version_bill();
                    if bill.drawer.peer_id.eq(&request_node_id) {
                        return true;
                    } else if bill.drawee.peer_id.eq(&request_node_id) {
                        return true;
                    } else if bill.payee.peer_id.eq(&request_node_id) {
                        return true;
                    }
                }
                OperationCode::Endorse => {
                    let block = self.get_block_by_id(block.id.clone());

                    let bill_keys = read_keys_from_bill_file(&block.bill_name);
                    let key: Rsa<Private> =
                        Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
                    let bytes = hex::decode(block.data.clone()).unwrap();
                    let decrypted_bytes = decrypt_bytes(&bytes, &key);
                    let block_data_decrypted = String::from_utf8(decrypted_bytes).unwrap();

                    let mut part_with_endorsee = block_data_decrypted
                        .split("Endorsed to ")
                        .collect::<Vec<&str>>()
                        .get(1)
                        .unwrap()
                        .to_string();

                    let part_with_endorsed_by = part_with_endorsee
                        .clone()
                        .split(" endorsed by ")
                        .collect::<Vec<&str>>()
                        .get(1)
                        .unwrap()
                        .to_string();

                    part_with_endorsee = part_with_endorsee
                        .split(" endorsed by ")
                        .collect::<Vec<&str>>()
                        .get(0)
                        .unwrap()
                        .to_string();

                    let endorsee_bill_u8 = hex::decode(part_with_endorsee).unwrap();
                    let endorsee_bill: IdentityPublicData =
                        serde_json::from_slice(&endorsee_bill_u8).unwrap();

                    let endorser_bill_u8 = hex::decode(part_with_endorsed_by).unwrap();
                    let endorser_bill: IdentityPublicData =
                        serde_json::from_slice(&endorser_bill_u8).unwrap();

                    if endorsee_bill.peer_id.eq(&request_node_id) {
                        return true;
                    } else if endorser_bill.peer_id.eq(&request_node_id) {
                        return true;
                    }
                }
                OperationCode::Mint => {
                    let block = self.get_block_by_id(block.id.clone());

                    let bill_keys = read_keys_from_bill_file(&block.bill_name);
                    let key: Rsa<Private> =
                        Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
                    let bytes = hex::decode(block.data.clone()).unwrap();
                    let decrypted_bytes = decrypt_bytes(&bytes, &key);
                    let block_data_decrypted = String::from_utf8(decrypted_bytes).unwrap();

                    let mut part_with_mint = block_data_decrypted
                        .split("Endorsed to ")
                        .collect::<Vec<&str>>()
                        .get(1)
                        .unwrap()
                        .to_string();

                    let part_with_minter = part_with_mint
                        .clone()
                        .split(" endorsed by ")
                        .collect::<Vec<&str>>()
                        .get(1)
                        .unwrap()
                        .to_string();

                    part_with_mint = part_with_mint
                        .split(" endorsed by ")
                        .collect::<Vec<&str>>()
                        .get(0)
                        .unwrap()
                        .to_string();

                    let minter_bill_u8 = hex::decode(part_with_minter).unwrap();
                    let minter_bill: IdentityPublicData =
                        serde_json::from_slice(&minter_bill_u8).unwrap();

                    let mint_bill_u8 = hex::decode(part_with_mint).unwrap();
                    let mint_bill: IdentityPublicData =
                        serde_json::from_slice(&mint_bill_u8).unwrap();

                    if minter_bill.peer_id.eq(&request_node_id) {
                        return true;
                    } else if mint_bill.peer_id.eq(&request_node_id) {
                        return true;
                    }
                }
                OperationCode::RequestToAccept => {
                    let block = self.get_block_by_id(block.id.clone());

                    let bill_keys = read_keys_from_bill_file(&block.bill_name);
                    let key: Rsa<Private> =
                        Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
                    let bytes = hex::decode(block.data.clone()).unwrap();
                    let decrypted_bytes = decrypt_bytes(&bytes, &key);
                    let block_data_decrypted = String::from_utf8(decrypted_bytes).unwrap();

                    let part_with_identity = block_data_decrypted
                        .split("Requested to accept by ")
                        .collect::<Vec<&str>>()
                        .get(1)
                        .unwrap()
                        .to_string();
                    let requester_to_accept_bill_u8 = hex::decode(part_with_identity).unwrap();
                    let requester_to_accept_bill: IdentityPublicData =
                        serde_json::from_slice(&requester_to_accept_bill_u8).unwrap();

                    if requester_to_accept_bill.peer_id.eq(&request_node_id) {
                        return true;
                    }
                }
                OperationCode::Accept => {
                    let block = self.get_block_by_id(block.id.clone());

                    let bill_keys = read_keys_from_bill_file(&block.bill_name);
                    let key: Rsa<Private> =
                        Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
                    let bytes = hex::decode(block.data.clone()).unwrap();
                    let decrypted_bytes = decrypt_bytes(&bytes, &key);
                    let block_data_decrypted = String::from_utf8(decrypted_bytes).unwrap();

                    let part_with_identity = block_data_decrypted
                        .split("Accepted by ")
                        .collect::<Vec<&str>>()
                        .get(1)
                        .unwrap()
                        .to_string();
                    let accepter_bill_u8 = hex::decode(part_with_identity).unwrap();
                    let accepter_bill: IdentityPublicData =
                        serde_json::from_slice(&accepter_bill_u8).unwrap();

                    if accepter_bill.peer_id.eq(&request_node_id) {
                        return true;
                    }
                }
                OperationCode::RequestToPay => {
                    let block = self.get_block_by_id(block.id.clone());

                    let bill_keys = read_keys_from_bill_file(&block.bill_name);
                    let key: Rsa<Private> =
                        Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
                    let bytes = hex::decode(block.data.clone()).unwrap();
                    let decrypted_bytes = decrypt_bytes(&bytes, &key);
                    let block_data_decrypted = String::from_utf8(decrypted_bytes).unwrap();

                    let part_with_identity = block_data_decrypted
                        .split("Requested to pay by ")
                        .collect::<Vec<&str>>()
                        .get(1)
                        .unwrap()
                        .to_string();
                    let requester_to_pay_bill_u8 = hex::decode(part_with_identity).unwrap();
                    let requester_to_pay_bill: IdentityPublicData =
                        serde_json::from_slice(&requester_to_pay_bill_u8).unwrap();

                    if requester_to_pay_bill.peer_id.eq(&request_node_id) {
                        return true;
                    }
                }
                OperationCode::Sell => {
                    let block = self.get_block_by_id(block.id.clone());

                    let bill_keys = read_keys_from_bill_file(&block.bill_name);
                    let key: Rsa<Private> =
                        Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
                    let bytes = hex::decode(block.data.clone()).unwrap();
                    let decrypted_bytes = decrypt_bytes(&bytes, &key);
                    let block_data_decrypted = String::from_utf8(decrypted_bytes).unwrap();

                    let mut part_without_sold_to = block_data_decrypted
                        .split("Sold to ")
                        .collect::<Vec<&str>>()
                        .get(1)
                        .unwrap()
                        .to_string();

                    let mut part_with_buyer = part_without_sold_to
                        .split(" sold by ")
                        .collect::<Vec<&str>>()
                        .get(0)
                        .unwrap()
                        .to_string();

                    let mut part_with_seller_and_amount = part_without_sold_to
                        .clone()
                        .split(" sold by ")
                        .collect::<Vec<&str>>()
                        .get(1)
                        .unwrap()
                        .to_string();

                    let mut amount: u64 = part_with_seller_and_amount
                        .clone()
                        .split(" amount: ")
                        .collect::<Vec<&str>>()
                        .get(1)
                        .unwrap()
                        .to_string()
                        .parse()
                        .unwrap();

                    let mut part_with_seller = part_with_seller_and_amount
                        .clone()
                        .split(" amount: ")
                        .collect::<Vec<&str>>()
                        .get(0)
                        .unwrap()
                        .to_string();

                    let buyer_bill_u8 = hex::decode(part_with_buyer).unwrap();
                    let buyer_bill: IdentityPublicData =
                        serde_json::from_slice(&buyer_bill_u8).unwrap();

                    let seller_bill_u8 = hex::decode(part_with_seller).unwrap();
                    let seller_bill: IdentityPublicData =
                        serde_json::from_slice(&seller_bill_u8).unwrap();

                    if buyer_bill.peer_id.eq(&request_node_id) {
                        return true;
                    } else if seller_bill.peer_id.eq(&request_node_id) {
                        return true;
                    }
                }
            }
        }
        return false;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, FromFormField)]
pub enum OperationCode {
    Issue,
    Accept,
    Endorse,
    RequestToAccept,
    RequestToPay,
    Sell,
    Mint,
}

impl OperationCode {
    pub fn get_all_operation_codes() -> Vec<OperationCode> {
        vec![
            OperationCode::Issue,
            OperationCode::Accept,
            OperationCode::Endorse,
            OperationCode::RequestToAccept,
            OperationCode::RequestToPay,
            OperationCode::Sell,
            OperationCode::Mint,
        ]
    }

    pub fn get_string_from_operation_code(self) -> String {
        match self {
            OperationCode::Issue => "Issue".to_string(),
            OperationCode::Accept => "Accept".to_string(),
            OperationCode::Endorse => "Endorse".to_string(),
            OperationCode::RequestToAccept => "RequestToAccept".to_string(),
            OperationCode::RequestToPay => "RequestToPay".to_string(),
            OperationCode::Sell => "Sell".to_string(),
            OperationCode::Mint => "Mint".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct BlockToReturn {
    pub id: u64,
    pub bill_name: String,
    pub hash: String,
    pub timestamp: i64,
    pub data: String,
    pub previous_hash: String,
    pub signature: String,
    pub public_key: String,
    pub operation_code: OperationCode,
    pub label: String,
}

impl BlockToReturn {
    pub fn new(block: Block, bill: BitcreditBill) -> Self {
        let label = block.get_history_label(bill);

        Self {
            id: block.id,
            bill_name: block.bill_name,
            hash: block.hash,
            timestamp: block.timestamp,
            data: block.data,
            previous_hash: block.previous_hash,
            signature: block.signature,
            public_key: block.public_key,
            operation_code: block.operation_code,
            label,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Block {
    pub id: u64,
    pub bill_name: String,
    pub hash: String,
    pub timestamp: i64,
    pub data: String,
    pub previous_hash: String,
    pub signature: String,
    pub public_key: String,
    pub operation_code: OperationCode,
}

impl Block {
    pub fn new(
        id: u64,
        previous_hash: String,
        data: String,
        bill_name: String,
        public_key: String,
        operation_code: OperationCode,
        private_key: String,
        timestamp: i64,
    ) -> Self {
        let hash: String = mine_block(
            &id,
            &bill_name,
            &previous_hash,
            &data,
            &timestamp,
            &public_key,
            &operation_code,
        );
        let signature = signature(hash.clone(), private_key.clone());

        Self {
            id,
            bill_name,
            hash,
            timestamp,
            previous_hash,
            signature,
            data,
            public_key,
            operation_code,
        }
    }

    pub fn get_nodes_from_block(&self, bill: BitcreditBill) -> Vec<String> {
        let mut nodes = Vec::new();
        match self.operation_code {
            OperationCode::Issue => {
                let drawer_name = bill.drawer.peer_id.clone();
                if !drawer_name.is_empty() && !nodes.contains(&drawer_name) {
                    nodes.push(drawer_name);
                }

                let payee_name = bill.payee.peer_id.clone();
                if !payee_name.is_empty() && !nodes.contains(&payee_name) {
                    nodes.push(payee_name);
                }

                let drawee_name = bill.drawee.peer_id.clone();
                if !drawee_name.is_empty() && !nodes.contains(&drawee_name) {
                    nodes.push(drawee_name);
                }
            }
            OperationCode::Endorse => {
                let bill_keys = read_keys_from_bill_file(&self.bill_name);
                let key: Rsa<Private> =
                    Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
                let bytes = hex::decode(self.data.clone()).unwrap();
                let decrypted_bytes = decrypt_bytes(&bytes, &key);
                let block_data_decrypted = String::from_utf8(decrypted_bytes).unwrap();

                let mut part_with_endorsee = block_data_decrypted
                    .split("Endorsed to ")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .unwrap()
                    .to_string();

                let part_with_endorsed_by = part_with_endorsee
                    .clone()
                    .split(" endorsed by ")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .unwrap()
                    .to_string();

                part_with_endorsee = part_with_endorsee
                    .split(" endorsed by ")
                    .collect::<Vec<&str>>()
                    .get(0)
                    .unwrap()
                    .to_string();

                let endorsee_bill_u8 = hex::decode(part_with_endorsee).unwrap();
                let endorsee_bill: IdentityPublicData =
                    serde_json::from_slice(&endorsee_bill_u8).unwrap();
                let endorsee_bill_name = endorsee_bill.peer_id.clone();
                if !endorsee_bill_name.is_empty() && !nodes.contains(&endorsee_bill_name) {
                    nodes.push(endorsee_bill_name);
                }

                let endorser_bill_u8 = hex::decode(part_with_endorsed_by).unwrap();
                let endorser_bill: IdentityPublicData =
                    serde_json::from_slice(&endorser_bill_u8).unwrap();
                let endorser_bill_name = endorser_bill.peer_id.clone();
                if !endorser_bill_name.is_empty() && !nodes.contains(&endorser_bill_name) {
                    nodes.push(endorser_bill_name);
                }
            }
            OperationCode::Mint => {
                let bill_keys = read_keys_from_bill_file(&self.bill_name);
                let key: Rsa<Private> =
                    Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
                let bytes = hex::decode(self.data.clone()).unwrap();
                let decrypted_bytes = decrypt_bytes(&bytes, &key);
                let block_data_decrypted = String::from_utf8(decrypted_bytes).unwrap();

                let mut part_with_mint = block_data_decrypted
                    .split("Endorsed to ")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .unwrap()
                    .to_string();

                let part_with_minter = part_with_mint
                    .clone()
                    .split(" endorsed by ")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .unwrap()
                    .to_string();

                part_with_mint = part_with_mint
                    .split(" endorsed by ")
                    .collect::<Vec<&str>>()
                    .get(0)
                    .unwrap()
                    .to_string();

                let mint_bill_u8 = hex::decode(part_with_mint).unwrap();
                let mint_bill: IdentityPublicData = serde_json::from_slice(&mint_bill_u8).unwrap();
                let mint_bill_name = mint_bill.peer_id.clone();
                if !mint_bill_name.is_empty() && !nodes.contains(&mint_bill_name) {
                    nodes.push(mint_bill_name);
                }

                let minter_bill_u8 = hex::decode(part_with_minter).unwrap();
                let minter_bill: IdentityPublicData =
                    serde_json::from_slice(&minter_bill_u8).unwrap();
                let minter_bill_name = minter_bill.peer_id.clone();
                if !minter_bill_name.is_empty() && !nodes.contains(&minter_bill_name) {
                    nodes.push(minter_bill_name);
                }
            }
            OperationCode::RequestToAccept => {
                let bill_keys = read_keys_from_bill_file(&self.bill_name);
                let key: Rsa<Private> =
                    Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
                let bytes = hex::decode(self.data.clone()).unwrap();
                let decrypted_bytes = decrypt_bytes(&bytes, &key);
                let block_data_decrypted = String::from_utf8(decrypted_bytes).unwrap();

                let part_with_identity = block_data_decrypted
                    .split("Requested to accept by ")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .unwrap()
                    .to_string();
                let requester_to_accept_bill_u8 = hex::decode(part_with_identity).unwrap();
                let requester_to_accept_bill: IdentityPublicData =
                    serde_json::from_slice(&requester_to_accept_bill_u8).unwrap();
                let requester_to_accept_bill_name = requester_to_accept_bill.peer_id.clone();
                if !requester_to_accept_bill_name.is_empty()
                    && !nodes.contains(&requester_to_accept_bill_name)
                {
                    nodes.push(requester_to_accept_bill_name);
                }
            }
            OperationCode::Accept => {
                let time_of_accept = Utc.timestamp_opt(self.timestamp.clone(), 0).unwrap();

                let bill_keys = read_keys_from_bill_file(&self.bill_name);
                let key: Rsa<Private> =
                    Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
                let bytes = hex::decode(self.data.clone()).unwrap();
                let decrypted_bytes = decrypt_bytes(&bytes, &key);
                let block_data_decrypted = String::from_utf8(decrypted_bytes).unwrap();

                let part_with_identity = block_data_decrypted
                    .split("Accepted by ")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .unwrap()
                    .to_string();
                let accepter_bill_u8 = hex::decode(part_with_identity).unwrap();
                let accepter_bill: IdentityPublicData =
                    serde_json::from_slice(&accepter_bill_u8).unwrap();
                let accepter_bill_name = accepter_bill.peer_id.clone();
                if !accepter_bill_name.is_empty() && !nodes.contains(&accepter_bill_name) {
                    nodes.push(accepter_bill_name);
                }
            }
            OperationCode::RequestToPay => {
                let time_of_request_to_pay = Utc.timestamp_opt(self.timestamp.clone(), 0).unwrap();

                let bill_keys = read_keys_from_bill_file(&self.bill_name);
                let key: Rsa<Private> =
                    Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
                let bytes = hex::decode(self.data.clone()).unwrap();
                let decrypted_bytes = decrypt_bytes(&bytes, &key);
                let block_data_decrypted = String::from_utf8(decrypted_bytes).unwrap();

                let part_with_identity = block_data_decrypted
                    .split("Requested to pay by ")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .unwrap()
                    .to_string();
                let requester_to_pay_bill_u8 = hex::decode(part_with_identity).unwrap();
                let requester_to_pay_bill: IdentityPublicData =
                    serde_json::from_slice(&requester_to_pay_bill_u8).unwrap();
                let requester_to_pay_bill_name = requester_to_pay_bill.peer_id.clone();
                if !requester_to_pay_bill_name.is_empty()
                    && !nodes.contains(&requester_to_pay_bill_name)
                {
                    nodes.push(requester_to_pay_bill_name);
                }
            }
            OperationCode::Sell => {
                let bill_keys = read_keys_from_bill_file(&self.bill_name);
                let key: Rsa<Private> =
                    Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
                let bytes = hex::decode(self.data.clone()).unwrap();
                let decrypted_bytes = decrypt_bytes(&bytes, &key);
                let block_data_decrypted = String::from_utf8(decrypted_bytes).unwrap();

                let mut part_without_sold_to = block_data_decrypted
                    .split("Sold to ")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .unwrap()
                    .to_string();

                let mut part_with_buyer = part_without_sold_to
                    .split(" sold by ")
                    .collect::<Vec<&str>>()
                    .get(0)
                    .unwrap()
                    .to_string();

                let mut part_with_seller_and_amount = part_without_sold_to
                    .clone()
                    .split(" sold by ")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .unwrap()
                    .to_string();

                let mut amount: u64 = part_with_seller_and_amount
                    .clone()
                    .split(" amount: ")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .unwrap()
                    .to_string()
                    .parse()
                    .unwrap();

                let mut part_with_seller = part_with_seller_and_amount
                    .clone()
                    .split(" amount: ")
                    .collect::<Vec<&str>>()
                    .get(0)
                    .unwrap()
                    .to_string();

                let buyer_bill_u8 = hex::decode(part_with_buyer).unwrap();
                let buyer_bill: IdentityPublicData =
                    serde_json::from_slice(&buyer_bill_u8).unwrap();
                let buyer_peer_id = buyer_bill.peer_id.clone();
                if !buyer_peer_id.is_empty() && !nodes.contains(&buyer_peer_id) {
                    nodes.push(buyer_peer_id);
                }

                let seller_bill_u8 = hex::decode(part_with_seller).unwrap();
                let seller_bill: IdentityPublicData =
                    serde_json::from_slice(&seller_bill_u8).unwrap();
                let seller_bill_peer_id = seller_bill.peer_id.clone();
                if !seller_bill_peer_id.is_empty() && !nodes.contains(&seller_bill_peer_id) {
                    nodes.push(seller_bill_peer_id);
                }
            }
        }
        nodes
    }

    pub fn get_history_label(&self, bill: BitcreditBill) -> String {
        let mut line = String::new();
        match self.operation_code {
            OperationCode::Issue => {
                let time_of_issue = Utc.timestamp_opt(self.timestamp.clone(), 0).unwrap();
                if !bill.drawer.name.is_empty() {
                    line = format!(
                        "Bill issued by {} at {} in {}",
                        bill.drawer.name, time_of_issue, bill.place_of_drawing
                    );
                } else if bill.to_payee {
                    line = format!(
                        "Bill issued by {} at {} in {}",
                        bill.payee.name, time_of_issue, bill.place_of_drawing
                    );
                } else {
                    line = format!(
                        "Bill issued by {} at {} in {}",
                        bill.drawee.name, time_of_issue, bill.place_of_drawing
                    );
                }
            }
            OperationCode::Endorse => {
                let time_of_endorse = Utc.timestamp_opt(self.timestamp.clone(), 0).unwrap();

                let bill_keys = read_keys_from_bill_file(&self.bill_name);
                let key: Rsa<Private> =
                    Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
                let bytes = hex::decode(self.data.clone()).unwrap();
                let decrypted_bytes = decrypt_bytes(&bytes, &key);
                let block_data_decrypted = String::from_utf8(decrypted_bytes).unwrap();

                let mut part_with_endorsee = block_data_decrypted
                    .split("Endorsed to ")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .unwrap()
                    .to_string();

                let part_with_endorsed_by = part_with_endorsee
                    .clone()
                    .split(" endorsed by ")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .unwrap()
                    .to_string();

                part_with_endorsee = part_with_endorsee
                    .split(" endorsed by ")
                    .collect::<Vec<&str>>()
                    .get(0)
                    .unwrap()
                    .to_string();

                let endorsee_bill_u8 = hex::decode(part_with_endorsee).unwrap();
                let endorsee_bill: IdentityPublicData =
                    serde_json::from_slice(&endorsee_bill_u8).unwrap();

                let endorser_bill_u8 = hex::decode(part_with_endorsed_by).unwrap();
                let endorser_bill: IdentityPublicData =
                    serde_json::from_slice(&endorser_bill_u8).unwrap();
                line = endorser_bill.name + ", " + &endorser_bill.postal_address;
            }
            OperationCode::Mint => {
                let time_of_mint = Utc.timestamp_opt(self.timestamp.clone(), 0).unwrap();

                let bill_keys = read_keys_from_bill_file(&self.bill_name);
                let key: Rsa<Private> =
                    Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
                let bytes = hex::decode(self.data.clone()).unwrap();
                let decrypted_bytes = decrypt_bytes(&bytes, &key);
                let block_data_decrypted = String::from_utf8(decrypted_bytes).unwrap();

                let mut part_with_mint = block_data_decrypted
                    .split("Endorsed to ")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .unwrap()
                    .to_string();

                let part_with_minter = part_with_mint
                    .clone()
                    .split(" endorsed by ")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .unwrap()
                    .to_string();

                part_with_mint = part_with_mint
                    .split(" endorsed by ")
                    .collect::<Vec<&str>>()
                    .get(0)
                    .unwrap()
                    .to_string();

                let mint_bill_u8 = hex::decode(part_with_mint).unwrap();
                let mint_bill: IdentityPublicData = serde_json::from_slice(&mint_bill_u8).unwrap();

                let minter_bill_u8 = hex::decode(part_with_minter).unwrap();
                let minter_bill: IdentityPublicData =
                    serde_json::from_slice(&minter_bill_u8).unwrap();
                line = minter_bill.name + ", " + &minter_bill.postal_address;
            }
            OperationCode::RequestToAccept => {
                let time_of_request_to_accept =
                    Utc.timestamp_opt(self.timestamp.clone(), 0).unwrap();

                let bill_keys = read_keys_from_bill_file(&self.bill_name);
                let key: Rsa<Private> =
                    Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
                let bytes = hex::decode(self.data.clone()).unwrap();
                let decrypted_bytes = decrypt_bytes(&bytes, &key);
                let block_data_decrypted = String::from_utf8(decrypted_bytes).unwrap();

                let part_with_identity = block_data_decrypted
                    .split("Requested to accept by ")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .unwrap()
                    .to_string();
                let requester_to_accept_bill_u8 = hex::decode(part_with_identity).unwrap();
                let requester_to_accept_bill: IdentityPublicData =
                    serde_json::from_slice(&requester_to_accept_bill_u8).unwrap();
                line = format!(
                    "Bill requested to accept by {} at {} in {}",
                    requester_to_accept_bill.name,
                    time_of_request_to_accept,
                    requester_to_accept_bill.postal_address
                );
            }
            OperationCode::Accept => {
                let time_of_accept = Utc.timestamp_opt(self.timestamp.clone(), 0).unwrap();

                let bill_keys = read_keys_from_bill_file(&self.bill_name);
                let key: Rsa<Private> =
                    Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
                let bytes = hex::decode(self.data.clone()).unwrap();
                let decrypted_bytes = decrypt_bytes(&bytes, &key);
                let block_data_decrypted = String::from_utf8(decrypted_bytes).unwrap();

                let part_with_identity = block_data_decrypted
                    .split("Accepted by ")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .unwrap()
                    .to_string();
                let accepter_bill_u8 = hex::decode(part_with_identity).unwrap();
                let accepter_bill: IdentityPublicData =
                    serde_json::from_slice(&accepter_bill_u8).unwrap();
                line = format!(
                    "Bill accepted by {} at {} in {}",
                    accepter_bill.name, time_of_accept, accepter_bill.postal_address
                );
            }
            OperationCode::RequestToPay => {
                let time_of_request_to_pay = Utc.timestamp_opt(self.timestamp.clone(), 0).unwrap();

                let bill_keys = read_keys_from_bill_file(&self.bill_name);
                let key: Rsa<Private> =
                    Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
                let bytes = hex::decode(self.data.clone()).unwrap();
                let decrypted_bytes = decrypt_bytes(&bytes, &key);
                let block_data_decrypted = String::from_utf8(decrypted_bytes).unwrap();

                let part_with_identity = block_data_decrypted
                    .split("Requested to pay by ")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .unwrap()
                    .to_string();
                let requester_to_pay_bill_u8 = hex::decode(part_with_identity).unwrap();
                let requester_to_pay_bill: IdentityPublicData =
                    serde_json::from_slice(&requester_to_pay_bill_u8).unwrap();
                line = format!(
                    "Bill requested to pay by {} at {} in {}",
                    requester_to_pay_bill.name,
                    time_of_request_to_pay,
                    requester_to_pay_bill.postal_address
                );
            }
            OperationCode::Sell => {
                let time_of_selling = Utc.timestamp_opt(self.timestamp.clone(), 0).unwrap();

                let bill_keys = read_keys_from_bill_file(&self.bill_name);
                let key: Rsa<Private> =
                    Rsa::private_key_from_pem(bill_keys.private_key_pem.as_bytes()).unwrap();
                let bytes = hex::decode(self.data.clone()).unwrap();
                let decrypted_bytes = decrypt_bytes(&bytes, &key);
                let block_data_decrypted = String::from_utf8(decrypted_bytes).unwrap();

                let mut part_without_sold_to = block_data_decrypted
                    .split("Sold to ")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .unwrap()
                    .to_string();

                let mut part_with_buyer = part_without_sold_to
                    .split(" sold by ")
                    .collect::<Vec<&str>>()
                    .get(0)
                    .unwrap()
                    .to_string();

                let mut part_with_seller_and_amount = part_without_sold_to
                    .clone()
                    .split(" sold by ")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .unwrap()
                    .to_string();

                let mut amount: u64 = part_with_seller_and_amount
                    .clone()
                    .split(" amount: ")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .unwrap()
                    .to_string()
                    .parse()
                    .unwrap();

                let part_with_seller = part_with_seller_and_amount
                    .clone()
                    .split(" amount: ")
                    .collect::<Vec<&str>>()
                    .get(0)
                    .unwrap()
                    .to_string();

                let buyer_bill_u8 = hex::decode(part_with_buyer).unwrap();
                let buyer_bill: IdentityPublicData =
                    serde_json::from_slice(&buyer_bill_u8).unwrap();

                let seller_bill_u8 = hex::decode(part_with_seller).unwrap();
                let seller_bill: IdentityPublicData =
                    serde_json::from_slice(&seller_bill_u8).unwrap();

                line = seller_bill.name + ", " + &seller_bill.postal_address;
            }
        }
        line
    }

    pub fn verifier(&self) -> bool {
        let public_key_bytes = self.public_key.as_bytes();
        let public_key_rsa = public_key_from_pem_u8(&public_key_bytes.to_vec());
        let verifier_key = PKey::from_rsa(public_key_rsa).unwrap();

        let mut verifier = Verifier::new(MessageDigest::sha256(), verifier_key.as_ref()).unwrap();

        let data_to_check = self.hash.as_bytes();
        verifier.update(data_to_check).unwrap();

        let signature_bytes = hex::decode(&self.signature).unwrap();
        verifier.verify(signature_bytes.as_slice()).unwrap()
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub struct GossipsubEvent {
    pub id: GossipsubEventId,
    pub message: Vec<u8>,
}

impl GossipsubEvent {
    pub fn new(id: GossipsubEventId, message: Vec<u8>) -> Self {
        Self { id, message }
    }

    pub fn to_byte_array(&self) -> Vec<u8> {
        let bytes = self.try_to_vec().expect("Failed to serialize event");
        bytes
    }

    pub fn from_byte_array(bytes: &Vec<u8>) -> Self {
        let event = Self::try_from_slice(bytes).expect("Failed to deserialize event");
        event
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub enum GossipsubEventId {
    Block,
    Chain,
    CommandGetChain,
}

#[derive(BorshSerialize, BorshDeserialize, FromForm, Debug, Serialize, Deserialize, Clone)]
pub struct BlockForHistory {
    id: u64,
    text: String,
    bill_name: String,
}

fn mine_block(
    id: &u64,
    bill_name: &str,
    previous_hash: &str,
    data: &str,
    timestamp: &i64,
    public_key: &str,
    operation_code: &OperationCode,
) -> String {
    let hash = calculate_hash(
        id,
        bill_name,
        previous_hash,
        data,
        timestamp,
        public_key,
        operation_code,
    );
    let binary_hash = hex::encode(&hash);
    info!(
        "mined! hash: {}, binary hash: {}",
        hex::encode(&hash),
        binary_hash
    );
    hex::encode(hash)
}

fn calculate_hash(
    id: &u64,
    bill_name: &str,
    previous_hash: &str,
    data: &str,
    timestamp: &i64,
    public_key: &str,
    operation_code: &OperationCode,
) -> Vec<u8> {
    let data = serde_json::json!({
        "id": id,
        "bill_name": bill_name,
        "previous_hash": previous_hash,
        "data": data,
        "timestamp": timestamp,
        "public_key": public_key,
        "operation_code": operation_code,
    });
    let mut hasher = Sha256::new();
    hasher.update(data.to_string().as_bytes());
    hasher.finish().try_to_vec().unwrap()
}

pub fn signature(hash: String, private_key_pem: String) -> String {
    let private_key_bytes = private_key_pem.as_bytes();
    let private_key_rsa = private_key_from_pem_u8(&private_key_bytes.to_vec());
    let signer_key = PKey::from_rsa(private_key_rsa).unwrap();

    let mut signer: Signer = Signer::new(MessageDigest::sha256(), signer_key.as_ref()).unwrap();

    let data_to_sign = hash.as_bytes();
    signer.update(data_to_sign).unwrap();

    let signature: Vec<u8> = signer.sign_to_vec().unwrap();
    let signature_readable = hex::encode(signature.as_slice());

    signature_readable
}

pub fn encrypted_hash_data_from_bill(bill: &BitcreditBill, private_key_pem: String) -> String {
    let bytes = bill.try_to_vec().unwrap();
    let key: Rsa<Private> = Rsa::private_key_from_pem(private_key_pem.as_bytes()).unwrap();
    let encrypted_bytes = encrypt_bytes(&bytes, &key);
    let data_from_bill_hash_readable = hex::encode(encrypted_bytes);
    data_from_bill_hash_readable
}

pub fn start_blockchain_for_new_bill(
    bill: &BitcreditBill,
    operation_code: OperationCode,
    drawer: IdentityPublicData,
    public_key: String,
    private_key: String,
    private_key_pem: String,
    timestamp: i64,
) {
    let data_for_new_block_in_bytes = serde_json::to_vec(&drawer).unwrap();
    let data_for_new_block = "Signed by ".to_string() + &hex::encode(data_for_new_block_in_bytes);

    let genesis_hash: String = hex::encode(data_for_new_block.as_bytes());

    let bill_data: String = encrypted_hash_data_from_bill(&bill, private_key_pem);

    let first_block = Block::new(
        1,
        genesis_hash,
        bill_data,
        bill.name.clone(),
        public_key,
        operation_code,
        private_key,
        timestamp,
    );

    let chain = Chain::new(first_block);
    let output_path = BILLS_FOLDER_PATH.to_string() + "/" + bill.name.clone().as_str() + ".json";
    std::fs::write(
        output_path.clone(),
        serde_json::to_string_pretty(&chain).unwrap(),
    )
    .unwrap();
}

pub fn is_block_valid(block: &Block, previous_block: &Block) -> bool {
    if block.previous_hash != previous_block.hash {
        warn!("block with id: {} has wrong previous hash", block.id);
        return false;
    } else if block.id != &previous_block.id + 1 {
        warn!(
            "block with id: {} is not the next block after the latest: {}",
            block.id, previous_block.id
        );
        return false;
    } else if hex::encode(calculate_hash(
        &block.id,
        &block.bill_name,
        &block.previous_hash,
        &block.data,
        &block.timestamp,
        &block.public_key,
        &block.operation_code,
    )) != block.hash
    {
        warn!("block with id: {} has invalid hash", block.id);
        return false;
    } else if !block.verifier() {
        warn!("block with id: {} has invalid signature", block.id);
        return false;
    }
    true
}
