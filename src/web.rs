#![feature(proc_macro_hygiene, decl_macro)]

use std::path::Path;
use std::str::FromStr;

use bitcoin::secp256k1::Scalar;
use chrono::{Days, Utc};
use libp2p::PeerId;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::form::Form;
use rocket::http::{Header, Status};
use rocket::serde::json::Json;
use rocket::{Request, Response, State};
use rocket_dyn_templates::{context, handlebars, Template};
use tokio::runtime::Handle;

use crate::blockchain::{Chain, ChainToReturn, GossipsubEvent, GossipsubEventId, OperationCode};
use crate::constants::{BILLS_FOLDER_PATH, BILL_VALIDITY_PERIOD, IDENTITY_FILE_PATH, USEDNET};
use crate::dht::network::Client;
use crate::work_with_mint::mint;
use crate::{
    accept_bill, add_in_contacts_map, api, blockchain, change_contact_data_from_dht,
    change_contact_name_from_contacts_map, create_whole_identity, delete_from_contacts_map,
    endorse_bitcredit_bill, get_bills, get_bills_for_list, get_contact_from_map, get_contacts_vec,
    get_whole_identity, issue_new_bill, issue_new_bill_drawer_is_drawee,
    issue_new_bill_drawer_is_payee, mint_bitcredit_bill, read_bill_from_file, read_contacts_map,
    read_identity_from_file, read_peer_id_from_file, request_acceptance, request_pay,
    AcceptBitcreditBillForm, BitcreditBill, BitcreditBillForList, BitcreditBillForm,
    BitcreditBillToReturn, Contact, DeleteContactForm, EditContactForm, EndorseBitcreditBillForm,
    Identity, IdentityForm, IdentityPublicData, IdentityWithAll, NewContactForm, NodeId,
    RequestToAcceptBitcreditBillForm, RequestToPayBitcreditBillForm,
};

use self::handlebars::{Handlebars, JsonRender};

#[get("/")]
pub async fn start() -> Template {
    if !Path::new(IDENTITY_FILE_PATH).exists() {
        Template::render("hbs/create_identity", context! {})
    } else {
        let bills = get_bills();
        let identity: IdentityWithAll = get_whole_identity();

        Template::render(
            "hbs/home",
            context! {
                identity: Some(identity.identity),
                bills: bills,
            },
        )
    }
}

#[get("/")]
pub async fn exit() {
    std::process::exit(0x0100);
}

#[get("/")]
pub async fn info() -> Template {
    Template::render("hbs/info", context! {})
}

#[get("/")]
pub async fn get_identity() -> Template {
    if !Path::new(IDENTITY_FILE_PATH).exists() {
        Template::render("hbs/create_identity", context! {})
    } else {
        let identity: IdentityWithAll = get_whole_identity();
        let peer_id = identity.peer_id.to_string();
        let usednet = USEDNET.to_string();

        Template::render(
            "hbs/identity",
            context! {
                peer_id: peer_id,
                identity: Some(identity.identity),
                usednet: usednet,
            },
        )
    }
}

#[get("/return")]
pub async fn return_identity() -> Json<Identity> {
    let my_identity;
    if !Path::new(IDENTITY_FILE_PATH).exists() {
        let identity = Identity::new_empty();
        my_identity = identity;
    } else {
        let identity: IdentityWithAll = get_whole_identity();
        my_identity = identity.identity;
    }
    Json(my_identity)
}

#[get("/peer_id/return")]
pub async fn return_peer_id() -> Json<NodeId> {
    let peer_id: PeerId = read_peer_id_from_file();
    let node_id = NodeId::new(peer_id.to_string());
    Json(node_id)
}

#[get("/return")]
pub async fn return_contacts() -> Json<Vec<Contact>> {
    let contacts: Vec<Contact> = get_contacts_vec();
    Json(contacts)
}

#[get("/return")]
pub async fn return_bills_list() -> Json<Vec<BitcreditBillToReturn>> {
    let bills: Vec<BitcreditBillToReturn> = get_bills_for_list().await;
    Json(bills)
}

#[post("/create", data = "<identity_form>")]
pub async fn create_identity(identity_form: Form<IdentityForm>, state: &State<Client>) -> Status {
    println!("Create identity");
    let identity: IdentityForm = identity_form.into_inner();
    create_whole_identity(
        identity.name,
        identity.company,
        identity.date_of_birth,
        identity.city_of_birth,
        identity.country_of_birth,
        identity.email,
        identity.postal_address,
    );

    let mut client = state.inner().clone();
    let identity: IdentityWithAll = get_whole_identity();
    let bills = get_bills();
    client.put_identity_public_data_in_dht().await;

    Status::Ok
}

#[get("/")]
pub async fn bills_list() -> Template {
    if !Path::new(IDENTITY_FILE_PATH).exists() {
        Template::render("hbs/create_identity", context! {})
    } else {
        let bills = get_bills();

        Template::render(
            "hbs/bills_list",
            context! {
                bills: bills,
            },
        )
    }
}

#[get("/history/<id>")]
pub async fn get_bill_history(id: String) -> Template {
    if !Path::new(IDENTITY_FILE_PATH).exists() {
        Template::render("hbs/create_identity", context! {})
    } else if Path::new((BILLS_FOLDER_PATH.to_string() + "/" + &id + ".json").as_str()).exists() {
        let mut bill: BitcreditBill = read_bill_from_file(&id);
        let chain = Chain::read_chain_from_file(&bill.name);
        let history = chain.get_bill_history();

        let address_to_pay = get_address_to_pay(bill.clone());
        let info_about_address =
            api::AddressInfo::get_testnet_address_info(address_to_pay.clone()).await;
        let chain_received_summ = info_about_address.chain_stats.funded_txo_sum;
        let chain_spent_summ = info_about_address.chain_stats.spent_txo_sum;
        let chain_summ = chain_received_summ + chain_spent_summ;
        let mempool_received_summ = info_about_address.mempool_stats.funded_txo_sum;
        let mempool_spent_summ = info_about_address.mempool_stats.spent_txo_sum;
        let mempool_summ = mempool_received_summ + mempool_spent_summ;

        Template::render(
            "hbs/bill_history",
            context! {
                bill: Some(bill),
                history: history,
                chain_summ: chain_summ,
                mempool_summ: mempool_summ,
                address_to_pay: address_to_pay,
            },
        )
    } else {
        let bills = get_bills();
        let identity: IdentityWithAll = get_whole_identity();
        let peer_id = read_peer_id_from_file().to_string();

        Template::render(
            "hbs/home",
            context! {
                peer_id: Some(peer_id),
                identity: Some(identity.identity),
                bills: bills,
            },
        )
    }
}

#[get("/blockchain/<id>")]
pub async fn get_bill_chain(id: String) -> Template {
    if !Path::new(IDENTITY_FILE_PATH).exists() {
        Template::render("hbs/create_identity", context! {})
    } else if Path::new((BILLS_FOLDER_PATH.to_string() + "/" + &id + ".json").as_str()).exists() {
        let mut bill: BitcreditBill = read_bill_from_file(&id);
        let chain = Chain::read_chain_from_file(&bill.name);
        Template::render(
            "hbs/bill_chain",
            context! {
                bill: Some(bill),
                chain: chain,
            },
        )
    } else {
        let bills = get_bills();
        let identity: IdentityWithAll = get_whole_identity();
        let peer_id = read_peer_id_from_file().to_string();

        Template::render(
            "hbs/home",
            context! {
                peer_id: Some(peer_id),
                identity: Some(identity.identity),
                bills: bills,
            },
        )
    }
}

#[get("/<id>/block/<block_id>")]
pub async fn get_block(id: String, block_id: u64) -> Template {
    if !Path::new(IDENTITY_FILE_PATH).exists() {
        Template::render("hbs/create_identity", context! {})
    } else if Path::new((BILLS_FOLDER_PATH.to_string() + "/" + &id + ".json").as_str()).exists() {
        let mut bill: BitcreditBill = read_bill_from_file(&id);
        let chain = Chain::read_chain_from_file(&bill.name);
        let block = chain.get_block_by_id(block_id);
        Template::render(
            "hbs/block",
            context! {
                bill: Some(bill),
                block: block,
            },
        )
    } else {
        let bills = get_bills();
        let identity: IdentityWithAll = get_whole_identity();
        let peer_id = read_peer_id_from_file().to_string();

        Template::render(
            "hbs/home",
            context! {
                peer_id: Some(peer_id),
                identity: Some(identity.identity),
                bills: bills,
            },
        )
    }
}

#[get("/return/basic/<id>")]
pub async fn return_basic_bill(id: String) -> Json<BitcreditBill> {
    let bill: BitcreditBill = read_bill_from_file(&id);
    Json(bill)
}

#[get("/chain/return/<id>")]
pub async fn return_chain_of_blocks(id: String) -> Json<Chain> {
    let chain = Chain::read_chain_from_file(&id);
    Json(chain)
}

#[get("/return")]
pub async fn return_operation_codes() -> Json<Vec<OperationCode>> {
    Json(OperationCode::get_all_operation_codes())
}

//PUT
#[get("/mint/<id>")]
pub async fn mint_bill(state: &State<Client>, id: String) -> Status {
    if !Path::new(IDENTITY_FILE_PATH).exists() {
        Status::NotAcceptable
    } else {
        let mut client = state.inner().clone();

        let timestamp = api::TimeApi::get_atomic_time().await.timestamp;

        let correct = mint_bitcredit_bill(&id, timestamp).await;

        if correct {
            let chain: Chain = Chain::read_chain_from_file(&id);
            let block = chain.get_latest_block();

            let block_bytes = serde_json::to_vec(block).expect("Error serializing block");
            let event = GossipsubEvent::new(GossipsubEventId::Block, block_bytes);
            let message = event.to_byte_array();

            client.add_message_to_topic(message, id.clone()).await;

            //TODO: add for mint or just sent it directly to mint?
            // client
            //     .add_bill_to_dht_for_node(
            //         &endorse_bill_form.bill_name,
            //         &public_data_endorsee.peer_id.to_string().clone(),
            //     )
            //     .await;
        } else {
            println!("Can't mint");
        }

        Status::Ok
    }
}

#[get("/return/<id>")]
pub async fn return_bill(id: String) -> Json<BitcreditBillToReturn> {
    let identity: IdentityWithAll = get_whole_identity();
    let bill: BitcreditBill = read_bill_from_file(&id);
    let chain = Chain::read_chain_from_file(&bill.name);
    let drawer = chain.get_drawer();
    let chain_to_return = ChainToReturn::new(chain.clone());
    let endorsed = chain.exist_block_with_operation_code(blockchain::OperationCode::Endorse);
    let accepted = chain.exist_block_with_operation_code(blockchain::OperationCode::Accept);
    let mut requested_to_pay =
        chain.exist_block_with_operation_code(blockchain::OperationCode::RequestToPay);
    let mut requested_to_accept =
        chain.exist_block_with_operation_code(blockchain::OperationCode::RequestToAccept);
    let address_to_pay = get_address_to_pay(bill.clone());
    let check_if_already_paid =
        check_if_paid(address_to_pay.clone(), bill.amount_numbers.clone()).await;
    let payed = check_if_already_paid.0;
    let mut number_of_confirmations: u64 = 0;
    let mut pending = false;
    if payed && check_if_already_paid.1.eq(&0) {
        pending = true;
    } else if payed && !check_if_already_paid.1.eq(&0) {
        let transaction = api::get_transactions_testet(address_to_pay.clone()).await;
        let txid = api::Txid::get_first_transaction(transaction.clone()).await;
        let height = api::get_testnet_last_block_height().await;
        number_of_confirmations = height - txid.status.block_height + 1;
    }
    let address_to_pay = get_address_to_pay(bill.clone());
    let message: String = format!("Payment in relation to a bill {}", bill.name.clone());
    let link_to_pay =
        generate_link_to_pay(address_to_pay.clone(), bill.amount_numbers.clone(), message).await;
    let mut pr_key_bill = String::new();
    if !endorsed.clone()
        && bill
            .payee
            .bitcoin_public_key
            .clone()
            .eq(&identity.identity.bitcoin_public_key)
    {
        pr_key_bill = get_current_payee_private_key(identity.identity.clone(), bill.clone());
    } else if endorsed
        && bill
            .endorsee
            .bitcoin_public_key
            .eq(&identity.identity.bitcoin_public_key)
    {
        pr_key_bill = get_current_payee_private_key(identity.identity.clone(), bill.clone());
    }

    let full_bill = BitcreditBillToReturn {
        name: bill.name,
        to_payee: bill.to_payee,
        bill_jurisdiction: bill.bill_jurisdiction,
        timestamp_at_drawing: bill.timestamp_at_drawing,
        drawee: bill.drawee,
        drawer: drawer,
        payee: bill.payee,
        endorsee: bill.endorsee,
        place_of_drawing: bill.place_of_drawing,
        currency_code: bill.currency_code,
        amount_numbers: bill.amount_numbers,
        amounts_letters: bill.amounts_letters,
        maturity_date: bill.maturity_date,
        date_of_issue: bill.date_of_issue,
        compounding_interest_rate: bill.compounding_interest_rate,
        type_of_interest_calculation: bill.type_of_interest_calculation,
        place_of_payment: bill.place_of_payment,
        public_key: bill.public_key,
        private_key: bill.private_key,
        language: bill.language,
        accepted,
        endorsed,
        requested_to_pay,
        requested_to_accept,
        payed,
        link_to_pay,
        address_to_pay,
        pr_key_bill,
        number_of_confirmations,
        pending,
        chain_of_blocks: chain_to_return,
    };
    Json(full_bill)
}

#[get("/<id>")]
pub async fn get_bill(id: String) -> Template {
    if !Path::new(IDENTITY_FILE_PATH).exists() {
        Template::render("hbs/create_identity", context! {})
    } else if Path::new((BILLS_FOLDER_PATH.to_string() + "/" + &id + ".json").as_str()).exists() {
        let mut bill: BitcreditBill = read_bill_from_file(&id);
        let chain = Chain::read_chain_from_file(&bill.name);
        let endorsed = chain.exist_block_with_operation_code(blockchain::OperationCode::Endorse);
        let last_block = chain.get_latest_block().clone();
        let operation_code = last_block.operation_code;
        let identity: IdentityWithAll = get_whole_identity();
        let accepted = chain.exist_block_with_operation_code(blockchain::OperationCode::Accept);
        let payee = bill.payee.clone();
        let local_peer_id = identity.peer_id.to_string().clone();
        let drawer_from_bill = bill.drawer.clone();
        let drawee_from_bill = bill.drawee.clone();
        let amount = bill.amount_numbers.clone();
        let payee_public_key = bill.payee.bitcoin_public_key.clone();
        let mut address_to_pay = String::new();
        let mut link_to_pay = String::new();
        let mut pr_key_bill = String::new();
        let mut payed: bool = false;
        let mut number_of_confirmations: u64 = 0;
        let usednet = USEDNET.to_string();
        let mut pending = String::new();
        let mut requested_to_pay =
            chain.exist_block_with_operation_code(blockchain::OperationCode::RequestToPay);
        let mut requested_to_accept =
            chain.exist_block_with_operation_code(blockchain::OperationCode::RequestToAccept);

        address_to_pay = get_address_to_pay(bill.clone());
        let message: String = format!("Payment in relation to a bill {}", bill.name.clone());
        link_to_pay = generate_link_to_pay(address_to_pay.clone(), amount, message).await;
        let check_if_already_paid = check_if_paid(address_to_pay.clone(), amount).await;
        payed = check_if_already_paid.0;
        if payed && check_if_already_paid.1.eq(&0) {
            pending = "Pending".to_string();
        } else if payed && !check_if_already_paid.1.eq(&0) {
            let transaction = api::get_transactions_testet(address_to_pay.clone()).await;
            let txid = api::Txid::get_first_transaction(transaction.clone()).await;
            let height = api::get_testnet_last_block_height().await;
            number_of_confirmations = height - txid.status.block_height + 1;
        }
        if !endorsed.clone() && payee_public_key.eq(&identity.identity.bitcoin_public_key)
        // && !payee.peer_id.eq(&drawee_from_bill.peer_id)
        {
            pr_key_bill = get_current_payee_private_key(identity.identity.clone(), bill.clone());
        } else if endorsed
            && bill
                .endorsee
                .bitcoin_public_key
                .eq(&identity.identity.bitcoin_public_key)
        {
            pr_key_bill = get_current_payee_private_key(identity.identity.clone(), bill.clone());
        }

        // if payed {
        //     bill.payee = bill.drawee.clone();
        // }

        Template::render(
            "hbs/bill",
            context! {
                codes: blockchain::OperationCode::get_all_operation_codes(),
                operation_code: operation_code,
                peer_id: local_peer_id,
                bill: Some(bill),
                identity: Some(identity.identity),
                accepted: accepted,
                payed: payed,
                requested_to_pay: requested_to_pay,
                requested_to_accept: requested_to_accept,
                address_to_pay: address_to_pay,
                pr_key_bill: pr_key_bill,
                usednet: usednet,
                endorsed: endorsed,
                pending: pending,
                number_of_confirmations: number_of_confirmations,
                link_to_pay: link_to_pay,
            },
        )
    } else {
        //todo: add for this block of code some function
        let bills = get_bills();
        let identity: IdentityWithAll = get_whole_identity();
        let peer_id = read_peer_id_from_file().to_string();

        Template::render(
            "hbs/home",
            context! {
                peer_id: Some(peer_id),
                identity: Some(identity.identity),
                bills: bills,
            },
        )
    }
}

async fn generate_link_to_pay(address: String, amount: u64, message: String) -> String {
    //todo check what net we used
    let link = format!("bitcoin:{}?amount={}&message={}", address, amount, message);
    link
}

pub async fn check_if_paid(address: String, amount: u64) -> (bool, u64) {
    //todo check what net we used
    let info_about_address = api::AddressInfo::get_testnet_address_info(address.clone()).await;
    let received_summ = info_about_address.chain_stats.funded_txo_sum;
    let spent_summ = info_about_address.chain_stats.spent_txo_sum;
    let received_summ_mempool = info_about_address.mempool_stats.funded_txo_sum;
    let spent_summ_mempool = info_about_address.mempool_stats.spent_txo_sum;
    return if amount.eq(&(received_summ + spent_summ + received_summ_mempool + spent_summ_mempool))
    {
        (true, received_summ.clone())
    } else {
        (false, 0)
    };
}

pub fn get_address_to_pay(bill: BitcreditBill) -> String {
    let public_key_bill = bitcoin::PublicKey::from_str(&bill.public_key).unwrap();

    let mut person_to_pay = bill.payee.clone();

    if !bill.endorsee.name.is_empty() {
        person_to_pay = bill.endorsee.clone();
    }

    let public_key_holder = person_to_pay.bitcoin_public_key;
    let public_key_bill_holder = bitcoin::PublicKey::from_str(&public_key_holder).unwrap();

    let public_key_bill = public_key_bill
        .inner
        .combine(&public_key_bill_holder.inner)
        .unwrap();
    let pub_key_bill = bitcoin::PublicKey::new(public_key_bill);
    let address_to_pay = bitcoin::Address::p2pkh(&pub_key_bill, USEDNET).to_string();

    address_to_pay
}

fn get_current_payee_private_key(identity: Identity, bill: BitcreditBill) -> String {
    let private_key_bill = bitcoin::PrivateKey::from_str(&bill.private_key).unwrap();

    let private_key_bill_holder =
        bitcoin::PrivateKey::from_str(&identity.bitcoin_private_key).unwrap();

    let privat_key_bill = private_key_bill
        .inner
        .add_tweak(&Scalar::from(private_key_bill_holder.inner.clone()))
        .unwrap();
    let pr_key_bill = bitcoin::PrivateKey::new(privat_key_bill, USEDNET).to_string();

    pr_key_bill
}

#[get("/dht")]
pub async fn search_bill(state: &State<Client>) -> Status {
    if !Path::new(IDENTITY_FILE_PATH).exists() {
        Status::NotAcceptable
    } else {
        let mut client = state.inner().clone();
        let local_peer_id = read_peer_id_from_file();
        client.check_new_bills(local_peer_id.to_string()).await;

        Status::Ok
    }
}

#[post("/issue", data = "<bill_form>")]
pub async fn issue_bill(state: &State<Client>, bill_form: Form<BitcreditBillForm>) -> Status {
    if !Path::new(IDENTITY_FILE_PATH).exists() {
        Status::NotAcceptable
    } else {
        let mut status: Status = Status::Ok;

        let form_bill = bill_form.into_inner();
        let drawer = get_whole_identity();
        let mut client = state.inner().clone();
        let timestamp = api::TimeApi::get_atomic_time().await.timestamp;
        let mut bill = BitcreditBill::new_empty();

        if form_bill.drawer_is_payee {
            let public_data_drawee =
                get_identity_public_data(form_bill.drawee_name, client.clone()).await;

            if !public_data_drawee.name.is_empty() {
                bill = issue_new_bill_drawer_is_payee(
                    form_bill.bill_jurisdiction,
                    form_bill.place_of_drawing,
                    form_bill.amount_numbers,
                    form_bill.place_of_payment,
                    form_bill.maturity_date,
                    drawer.clone(),
                    form_bill.language,
                    public_data_drawee,
                    timestamp,
                );
            } else {
                status = Status::NotAcceptable
            }
        } else if form_bill.drawer_is_drawee {
            let public_data_payee =
                get_identity_public_data(form_bill.payee_name, client.clone()).await;

            if !public_data_payee.name.is_empty() {
                bill = issue_new_bill_drawer_is_drawee(
                    form_bill.bill_jurisdiction,
                    form_bill.place_of_drawing,
                    form_bill.amount_numbers,
                    form_bill.place_of_payment,
                    form_bill.maturity_date,
                    drawer.clone(),
                    form_bill.language,
                    public_data_payee,
                    timestamp,
                );
            } else {
                status = Status::NotAcceptable
            }
        } else {
            let public_data_drawee =
                get_identity_public_data(form_bill.drawee_name, client.clone()).await;

            let public_data_payee =
                get_identity_public_data(form_bill.payee_name, client.clone()).await;

            if !public_data_payee.name.is_empty() && !public_data_drawee.name.is_empty() {
                bill = issue_new_bill(
                    form_bill.bill_jurisdiction,
                    form_bill.place_of_drawing,
                    form_bill.amount_numbers,
                    form_bill.place_of_payment,
                    form_bill.maturity_date,
                    drawer.clone(),
                    form_bill.language,
                    public_data_drawee,
                    public_data_payee,
                    timestamp,
                );
            } else {
                status = Status::NotAcceptable
            }
        }

        if status.eq(&Status::Ok) {
            let mut nodes: Vec<String> = Vec::new();
            let my_peer_id = drawer.peer_id.to_string().clone();
            nodes.push(my_peer_id.to_string());
            nodes.push(bill.drawee.peer_id.clone());
            nodes.push(bill.payee.peer_id.clone());

            for node in nodes {
                if !node.is_empty() {
                    println!("Add {} for node {}", &bill.name, &node);
                    client.add_bill_to_dht_for_node(&bill.name, &node).await;
                }
            }

            client.subscribe_to_topic(bill.name.clone()).await;

            client.put(&bill.name).await;

            if form_bill.drawer_is_drawee {
                let timestamp = api::TimeApi::get_atomic_time().await.timestamp;

                let correct = accept_bill(&bill.name, timestamp);

                if correct {
                    let chain: Chain = Chain::read_chain_from_file(&bill.name);
                    let block = chain.get_latest_block();

                    let block_bytes = serde_json::to_vec(block).expect("Error serializing block");
                    let event = GossipsubEvent::new(GossipsubEventId::Block, block_bytes);
                    let message = event.to_byte_array();

                    client
                        .add_message_to_topic(message, bill.name.clone())
                        .await;
                } else {
                    println!("Can't accept");
                }
            }
        }

        return status;
    }
}

#[get("/")]
pub async fn new_two_party_bill_drawer_is_payee() -> Template {
    if !Path::new(IDENTITY_FILE_PATH).exists() {
        Template::render("hbs/create_identity", context! {})
    } else {
        let identity: IdentityWithAll = get_whole_identity();
        let utc = Utc::now();
        let date_of_issue = utc.naive_local().date().to_string();
        let maturity_date = utc
            .checked_add_days(Days::new(BILL_VALIDITY_PERIOD))
            .unwrap()
            .naive_local()
            .date()
            .to_string();

        Template::render(
            "hbs/new_two_party_bill_drawer_is_payee",
            context! {
                identity: Some(identity.identity),
                date_of_issue: date_of_issue,
                maturity_date: maturity_date,
            },
        )
    }
}

#[get("/")]
pub async fn new_two_party_bill_drawer_is_drawee() -> Template {
    if !Path::new(IDENTITY_FILE_PATH).exists() {
        Template::render("hbs/create_identity", context! {})
    } else {
        let identity: IdentityWithAll = get_whole_identity();
        let utc = Utc::now();
        let date_of_issue = utc.naive_local().date().to_string();
        let maturity_date = utc
            .checked_add_days(Days::new(BILL_VALIDITY_PERIOD))
            .unwrap()
            .naive_local()
            .date()
            .to_string();

        Template::render(
            "hbs/new_two_party_bill_drawer_is_drawee",
            context! {
                identity: Some(identity.identity),
                date_of_issue: date_of_issue,
                maturity_date: maturity_date,
            },
        )
    }
}

pub async fn get_identity_public_data(
    identity_real_name: String,
    mut client: Client,
) -> IdentityPublicData {
    let mut identity = get_contact_from_map(&identity_real_name);

    let mut identity_public_data = client
        .get_identity_public_data_from_dht(identity.peer_id.clone())
        .await;

    if !identity_public_data.name.is_empty() {
        change_contact_data_from_dht(
            identity_real_name,
            identity_public_data.clone(),
            identity.clone(),
        );
        identity = identity_public_data;
    }

    identity
}

//PUT
#[post("/endorse", data = "<endorse_bill_form>")]
pub async fn endorse_bill(
    state: &State<Client>,
    endorse_bill_form: Form<EndorseBitcreditBillForm>,
) -> Status {
    if !Path::new(IDENTITY_FILE_PATH).exists() {
        Status::NotAcceptable
    } else {
        let mut client = state.inner().clone();

        let public_data_endorsee =
            get_identity_public_data(endorse_bill_form.endorsee.clone(), client.clone()).await;

        if !public_data_endorsee.name.is_empty() {
            let timestamp = api::TimeApi::get_atomic_time().await.timestamp;

            let correct = endorse_bitcredit_bill(
                &endorse_bill_form.bill_name,
                public_data_endorsee.clone(),
                timestamp,
            );

            if correct {
                let chain: Chain = Chain::read_chain_from_file(&endorse_bill_form.bill_name);
                let block = chain.get_latest_block();

                let block_bytes = serde_json::to_vec(block).expect("Error serializing block");
                let event = GossipsubEvent::new(GossipsubEventId::Block, block_bytes);
                let message = event.to_byte_array();

                client
                    .add_message_to_topic(message, endorse_bill_form.bill_name.clone())
                    .await;

                client
                    .add_bill_to_dht_for_node(
                        &endorse_bill_form.bill_name,
                        &public_data_endorsee.peer_id.to_string().clone(),
                    )
                    .await;
            } else {
                println!("Can't endorse");
            }

            let bills = get_bills();
            let identity: Identity = read_identity_from_file();

            Status::Ok
        } else {
            Status::NotAcceptable
        }
    }
}

#[post("/request_to_pay", data = "<request_to_pay_bill_form>")]
pub async fn request_to_pay_bill(
    state: &State<Client>,
    request_to_pay_bill_form: Form<RequestToPayBitcreditBillForm>,
) -> Status {
    if !Path::new(IDENTITY_FILE_PATH).exists() {
        Status::NotAcceptable
    } else {
        let mut client = state.inner().clone();

        let timestamp = api::TimeApi::get_atomic_time().await.timestamp;

        let correct = request_pay(&request_to_pay_bill_form.bill_name, timestamp);

        if correct {
            let chain: Chain = Chain::read_chain_from_file(&request_to_pay_bill_form.bill_name);
            let block = chain.get_latest_block();

            let block_bytes = serde_json::to_vec(block).expect("Error serializing block");
            let event = GossipsubEvent::new(GossipsubEventId::Block, block_bytes);
            let message = event.to_byte_array();

            client
                .add_message_to_topic(message, request_to_pay_bill_form.bill_name.clone())
                .await;
        } else {
            println!("Can't request to pay");
        }
        Status::Ok
    }
}

#[post("/request_to_accept", data = "<request_to_accept_bill_form>")]
pub async fn request_to_accept_bill(
    state: &State<Client>,
    request_to_accept_bill_form: Form<RequestToAcceptBitcreditBillForm>,
) -> Status {
    if !Path::new(IDENTITY_FILE_PATH).exists() {
        Status::NotAcceptable
    } else {
        let mut client = state.inner().clone();

        let timestamp = api::TimeApi::get_atomic_time().await.timestamp;

        let correct = request_acceptance(&request_to_accept_bill_form.bill_name, timestamp);

        if correct {
            let chain: Chain = Chain::read_chain_from_file(&request_to_accept_bill_form.bill_name);
            let block = chain.get_latest_block();

            let block_bytes = serde_json::to_vec(block).expect("Error serializing block");
            let event = GossipsubEvent::new(GossipsubEventId::Block, block_bytes);
            let message = event.to_byte_array();

            client
                .add_message_to_topic(message, request_to_accept_bill_form.bill_name.clone())
                .await;
        } else {
            println!("Can't request to accept");
        }
        Status::Ok
    }
}

#[post("/accept", data = "<accept_bill_form>")]
pub async fn accept_bill_form(
    state: &State<Client>,
    accept_bill_form: Form<AcceptBitcreditBillForm>,
) -> Status {
    if !Path::new(IDENTITY_FILE_PATH).exists() {
        Status::NotAcceptable
    } else {
        let mut client = state.inner().clone();

        let timestamp = api::TimeApi::get_atomic_time().await.timestamp;

        let correct = accept_bill(&accept_bill_form.bill_name, timestamp);

        if correct {
            let chain: Chain = Chain::read_chain_from_file(&accept_bill_form.bill_name);
            let block = chain.get_latest_block();

            let block_bytes = serde_json::to_vec(block).expect("Error serializing block");
            let event = GossipsubEvent::new(GossipsubEventId::Block, block_bytes);
            let message = event.to_byte_array();

            client
                .add_message_to_topic(message, accept_bill_form.bill_name.clone())
                .await;
        } else {
            println!("Can't accept");
        }
        Status::Ok
    }
}

#[get("/add")]
pub async fn add_contact() -> Template {
    if !Path::new(IDENTITY_FILE_PATH).exists() {
        Template::render("hbs/create_identity", context! {})
    } else {
        Template::render("hbs/new_contact", context! {})
    }
}

#[post("/remove", data = "<remove_contact_form>")]
pub async fn remove_contact(remove_contact_form: Form<DeleteContactForm>) -> Status {
    if !Path::new(IDENTITY_FILE_PATH).exists() {
        Status::NotAcceptable
    } else {
        delete_from_contacts_map(remove_contact_form.name.clone());

        Status::Ok
    }
}

#[post("/new", data = "<new_contact_form>")]
pub async fn new_contact(
    state: &State<Client>,
    new_contact_form: Form<NewContactForm>,
) -> Result<Json<Vec<Contact>>, Status> {
    if !Path::new(IDENTITY_FILE_PATH).exists() {
        Err(Status::NotAcceptable)
    } else {
        add_in_contacts_map(
            new_contact_form.name.clone(),
            new_contact_form.node_id.clone(),
            state.inner().clone(),
        )
        .await;

        Ok(Json(get_contacts_vec()))
    }
}

#[post("/edit", data = "<edit_contact_form>")]
pub async fn edit_contact(
    edit_contact_form: Form<EditContactForm>,
) -> Result<Json<Vec<Contact>>, Status> {
    if !Path::new(IDENTITY_FILE_PATH).exists() {
        Err(Status::NotAcceptable)
    } else {
        change_contact_name_from_contacts_map(
            edit_contact_form.old_name.clone(),
            edit_contact_form.name.clone(),
        );

        Ok(Json(get_contacts_vec()))
    }
}

#[get("/")]
pub async fn contacts() -> Template {
    if !Path::new(IDENTITY_FILE_PATH).exists() {
        Template::render("hbs/create_identity", context! {})
    } else {
        let contacts = read_contacts_map();
        Template::render(
            "hbs/contacts",
            context! {
                contacts: contacts,
            },
        )
    }
}

#[catch(404)]
pub fn not_found(req: &Request) -> String {
    format!("We couldn't find the requested path '{}'", req.uri())
}

pub fn customize(hbs: &mut Handlebars) {
    hbs.register_helper("wow", Box::new(wow_helper));
    hbs.register_template_string(
        "hbs/about.html",
        r#"
        {{#*inline "page"}}
        <section id="about">
        <h1>About - Here's another page!</h1>
        </section>
        {{/inline}}
        {{> hbs/layout}}
    "#,
    )
    .expect("valid HBS template");
}

fn wow_helper(
    h: &handlebars::Helper<'_, '_>,
    _: &handlebars::Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext<'_, '_>,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    if let Some(param) = h.param(0) {
        out.write("<b><i>")?;
        out.write(&param.value().render())?;
        out.write("</b></i>")?;
    }

    Ok(())
}

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "POST, GET, PATCH, OPTIONS",
        ));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}
