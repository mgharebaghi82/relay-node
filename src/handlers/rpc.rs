use libp2p::{
    futures::StreamExt,
    gossipsub::{Behaviour, IdentTopic},
    identity::Keypair,
    swarm::SwarmEvent,
    Multiaddr, SwarmBuilder,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use sp_core::ecdsa::{Public, Signature};
use std::{
    fs::File,
    io::{BufRead, BufReader},
};
use std::{net::SocketAddr, time::Duration};

use axum::{extract, http::Method, routing::post, Router};
use tower_http::cors::{AllowHeaders, Any, CorsLayer};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum TransactionScript {
    SingleSig,
    MultiSig,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Transaction {
    pub tx_hash: String,
    pub input: TxInput,
    pub output: TxOutput,
    #[serde_as(as = "DisplayFromStr")]
    pub value: Decimal,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct TxInput {
    pub input_hash: String,
    pub input_data: InputData,
    pub signatures: Vec<Signature>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct InputData {
    pub number: u8,
    pub utxos: Vec<UtxoData>,
    pub script: TransactionScript,
}

#[serde_as]
//a UTXO structure model
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct UtxoData {
    pub transaction_hash: String,
    #[serde_as(as = "DisplayFromStr")]
    pub unspent: Decimal,
    pub output_hash: String,
    pub block_number: i64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct TxOutput {
    pub output_hash: String,
    pub output_data: OutputData,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct OutputData {
    pub number: u8,
    pub utxos: Vec<OutputUtxo>,
    pub sigenr_public_keys: Vec<Public>,
    #[serde_as(as = "DisplayFromStr")]
    pub fee: Decimal,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct OutputUtxo {
    pub hash: String,
    pub output_unspent: OutputUnspent,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct OutputUnspent {
    pub public_key: String,
    #[serde_as(as = "DisplayFromStr")]
    pub unspent: Decimal,
    pub rnum: u32,
}

async fn handle_transaction(extract::Json(transaction): extract::Json<Transaction>) -> String {
    let client_topic = IdentTopic::new("client");
    //generate peer keys and peer id for network
    let keypair = Keypair::generate_ecdsa();
    // let local_peer_id = PeerId::from(keypair.public());

    //gossip protocol config
    let privacy = libp2p::gossipsub::MessageAuthenticity::Signed(keypair.clone());
    let gossip_cfg_builder = libp2p::gossipsub::ConfigBuilder::default();
    let gossip_cfg = libp2p::gossipsub::ConfigBuilder::build(&gossip_cfg_builder).unwrap();
    let gossipsub: Behaviour = libp2p::gossipsub::Behaviour::new(privacy, gossip_cfg).unwrap();
    // gossipsub.subscribe(&relay_topic.clone()).unwrap();

    //config swarm
    let swarm_config = libp2p::swarm::Config::with_tokio_executor()
        .with_idle_connection_timeout(Duration::from_secs(15));

    let mut swarm = SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            Default::default(),
            (libp2p::tls::Config::new, libp2p::noise::Config::new),
            libp2p::yamux::Config::default,
        )
        .unwrap()
        .with_quic()
        .with_dns()
        .unwrap()
        .with_websocket(
            (libp2p::tls::Config::new, libp2p::noise::Config::new),
            libp2p::yamux::Config::default,
        )
        .await
        .unwrap()
        .with_behaviour(|_key| gossipsub)
        .unwrap()
        .with_swarm_config(|_conf| swarm_config)
        .build();

    let listener: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse().unwrap();
    swarm.listen_on(listener).unwrap();

    let mut dial_addr = String::new();

    let address_file = File::open("/etc/myaddress.dat").unwrap();
    let reader = BufReader::new(address_file);
    for i in reader.lines() {
        let addr = i.unwrap();
        if addr.trim().len() > 0 {
            dial_addr.push_str(&addr);
            break;
        }
    }

    let dial_multiaddr: Multiaddr = dial_addr.parse().unwrap();
    swarm.dial(dial_multiaddr).unwrap();

    let str_transaction = serde_json::to_string(&transaction).unwrap();

    loop {
        match swarm.next().await.unwrap() {
            SwarmEvent::ConnectionEstablished { .. } => {
                match swarm
                    .behaviour_mut()
                    .publish(client_topic, str_transaction.as_bytes())
                {
                    Ok(_) => {
                        return "Your transaction sent.".to_string();
                    }
                    Err(_) => {
                        return "Sending failed!".to_string();
                    }
                }
            }
            _ => {}
        }
    }
}

pub async fn handle_requests() {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any)
        .allow_headers(AllowHeaders::any());
    let app: Router = Router::new()
        .route("/rpc", post(handle_transaction))
        .layer(cors);
    let addr = SocketAddr::from(([0, 0, 0, 0], 3390));

    if let Some(ip) = public_ip::addr().await {
        let full_address = format!("http://{}:3390", ip);
        let client = reqwest::Client::new();
        let res = client
            .post("https://centichain.org/api/rpc")
            .body(full_address)
            .send()
            .await;
        match res {
            Ok(_) => println!("Your address sent."),
            Err(_) => println!("problem to send address!"),
        }
        println!("your public ip: {}", ip);
    } else {
        println!("You dont have public ip, listener: {}", addr);
    }

    axum_server::bind(addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
