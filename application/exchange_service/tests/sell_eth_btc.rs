#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate bitcoin_rpc_client;
extern crate bitcoin_support;
extern crate common_types;
extern crate env_logger;
extern crate ethereum_support;
extern crate ethereum_wallet;
extern crate event_store;
extern crate exchange_service;
extern crate hex;
extern crate rocket;
extern crate rocket_contrib;
extern crate secp256k1_support;
extern crate serde;
extern crate serde_json;

use bitcoin_rpc_client::BlockHeight;
use bitcoin_support::Network;
use common_types::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
    secret::Secret,
    TradingSymbol,
};
use ethereum_support::{web3, Bytes, H256};
use ethereum_wallet::fake::StaticFakeWallet;
use event_store::{EventStore, InMemoryEventStore};
use exchange_service::{
    bitcoin_fee_service::StaticBitcoinFeeService,
    ethereum_service::{self, BlockingEthereumApi},
    gas_price_service::StaticGasPriceService,
    rocket_factory::create_rocket_instance,
    swaps::{
        common::TradeId,
        events::{OfferCreated, OrderTaken, TradeFunded},
    },
    treasury_api_client::FakeApiClient,
};
use hex::FromHex;
use rocket::{
    http::{ContentType, Status},
    local::{Client, LocalResponse},
};
use secp256k1_support::KeyPair;
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc, time::Duration};

trait DeserializeAsJson {
    fn body_json<T>(&mut self) -> T
    where
        for<'de> T: Deserialize<'de>;
}

impl<'r> DeserializeAsJson for LocalResponse<'r> {
    fn body_json<T>(&mut self) -> T
    where
        for<'de> T: Deserialize<'de>,
    {
        let body = self.body().unwrap().into_inner();

        serde_json::from_reader(body).unwrap()
    }
}

struct StaticEthereumApi;

impl BlockingEthereumApi for StaticEthereumApi {
    fn send_raw_transaction(&self, _rlp: Bytes) -> Result<H256, web3::Error> {
        Ok(H256::new())
    }
}

fn create_rocket_client(event_store: InMemoryEventStore<TradeId>) -> Client {
    let rocket = create_rocket_instance(
        Arc::new(FakeApiClient),
        event_store,
        Arc::new(ethereum_service::EthereumService::new(
            Arc::new(StaticFakeWallet::account0()),
            Arc::new(StaticGasPriceService::default()),
            Arc::new(StaticEthereumApi),
            0,
        )),
        Arc::new(bitcoin_rpc_client::BitcoinStubClient::new()),
        "e7b6bfabddfaeb2c016b334a5322e4327dc5e499".into(),
        bitcoin_support::PrivateKey::from_str(
            "cR6U4gNiCQsPo5gLNP2w6QsLTZkvCGEijhYVPZVhnePQKjMwmas8",
        ).unwrap()
            .secret_key()
            .clone()
            .into(),
        bitcoin_support::Address::from_str("2NBNQWga7p2yEZmk1m5WuMxK5SyXM5cBZSL").unwrap(),
        Network::Regtest,
        Arc::new(StaticBitcoinFeeService::new(50.0)),
    );
    rocket::local::Client::new(rocket).unwrap()
}

fn mock_offer_created(event_store: &InMemoryEventStore<TradeId>, trade_id: TradeId) {
    let offer_created: OfferCreated<Bitcoin, Ethereum> = OfferCreated::new(
        0.1,
        bitcoin_support::BitcoinQuantity::from_bitcoin(1.0),
        ethereum_support::EthereumQuantity::from_eth(10.0),
        TradingSymbol::ETH_BTC,
    );
    event_store
        .add_event(trade_id.clone(), offer_created)
        .unwrap();
}

fn mock_order_taken(event_store: &InMemoryEventStore<TradeId>, trade_id: TradeId) {
    let bytes = b"hello world, you are beautiful!!";
    let secret = Secret::from(*bytes);

    let secret_key_data = <[u8; 32]>::from_hex(
        "e8aafba2be13ee611059bc756878933bee789cc1aec7c35e23054a44d071c80b",
    ).unwrap();
    let keypair = KeyPair::from_secret_key_slice(&secret_key_data).unwrap();

    let order_taken: OrderTaken<Bitcoin, Ethereum> = OrderTaken {
        uid: trade_id,
        contract_secret_lock: secret.hash(),
        client_contract_time_lock: Duration::new(60 * 60 * 12, 0),
        exchange_contract_time_lock: BlockHeight::new(24u32),
        client_refund_address: ethereum_support::Address::from_str(
            "1111111111111111111111111111111111111111",
        ).unwrap(),
        client_success_address: bitcoin_support::Address::from_str(
            "2NBNQWga7p2yEZmk1m5WuMxK5SyXM5cBZSL",
        ).unwrap(),
        exchange_refund_address: bitcoin_support::Address::from_str(
            "bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap",
        ).unwrap(),
        exchange_success_address: ethereum_support::Address::from_str(
            "2222222222222222222222222222222222222222",
        ).unwrap(),
        exchange_success_keypair: keypair,
    };
    event_store.add_event(trade_id, order_taken).unwrap();
}

fn mock_trade_funded(event_store: &InMemoryEventStore<TradeId>, trade_id: TradeId) {
    let trade_funded: TradeFunded<Bitcoin, Ethereum> = TradeFunded::new(
        trade_id,
        ethereum_support::Address::from_str("2222222222222222222222222222222222222222").unwrap(),
    );
    event_store.add_event(trade_id, trade_funded).unwrap();
}
//series of events is as follows:
// OfferCreated buy ETH for BTC -> OrderTaken ETH for BTC-> TradeFunded BTC from trader -> ContractDeployed ETH from exchange

#[test]
fn given_an_accepted_trade_when_provided_with_funding_tx_should_deploy_htlc() {
    let _ = env_logger::try_init();
    let event_store = InMemoryEventStore::new();

    let trade_id = TradeId::new();

    mock_offer_created(&event_store, trade_id);
    mock_order_taken(&event_store, trade_id);

    let client = create_rocket_client(event_store);

    let response = {
        let request = client
            .post(format!("/trades/ETH-BTC/{}/sell-order-htlc-funded", trade_id).to_string())
            .header(ContentType::JSON)
            .body(r#" "0x3333333333333333333333333333333333333333" "#);
        request.dispatch()
    };

    assert_eq!(response.status(), Status::Ok);
    //contract should be deployed now
    //TODO Finish this test and implement bitcoin service
}

#[derive(Serialize)]
pub struct RedeemETHNotificationBody {
    pub secret: Secret,
}

#[test]
fn given_an_deployed_htlc_and_secret_should_redeem_htlc() {
    let _ = env_logger::try_init();
    let event_store = InMemoryEventStore::new();

    let trade_id = TradeId::new();

    mock_offer_created(&event_store, trade_id);
    mock_order_taken(&event_store, trade_id);
    mock_trade_funded(&event_store, trade_id);

    let client = create_rocket_client(event_store);

    let bytes = b"hello world, you are beautiful!!";
    let secret = Secret::from(*bytes);
    let redeem_body = RedeemETHNotificationBody { secret };
    let response = {
        let request = client
            .post(format!("/trades/ETH-BTC/{}/sell-order-secret-revealed", trade_id).to_string())
            .header(ContentType::JSON)
            .body(serde_json::to_string(&redeem_body).unwrap());
        request.dispatch()
    };
    assert_eq!(response.status(), Status::Ok);
}