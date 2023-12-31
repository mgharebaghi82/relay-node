mod handlers;
pub mod rpc;
use std::env::consts::OS;
use std::fs::{File, OpenOptions, self};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::TcpStream;
use std::time::Duration;

use handlers::handle_streams;
use handlers::structures::Channels;
use handlers::structures::CustomBehav;
use handlers::structures::Req;
use handlers::structures::Res;

use libp2p::{
    gossipsub::IdentTopic,
    identity::Keypair,
    request_response::{cbor, ProtocolSupport},
    Multiaddr, PeerId, StreamProtocol, SwarmBuilder,
};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
struct Addresses {
    addr: Vec<String>,
}

pub async fn run() {
    let mut wallet = String::new();
    let mut wallet_path = "";
    if OS == "linux" {
        wallet_path = "/etc/wallet.dat"
    } else if OS == "windows" {
        wallet_path = "wallet.dat"
    };
    let wallet_file = File::open(wallet_path).unwrap();
    let reader = BufReader::new(wallet_file);
    for addr in reader.lines() {
        let wallet_addr = addr.unwrap();
        if wallet_addr.trim().len() > 0 {
            wallet.push_str(&wallet_addr);
        }
    }

    let relay_topic = IdentTopic::new("relay");
    let clients_topic = IdentTopic::new("client");
    let tx_topic = IdentTopic::new("transaction");

    //generate peer keys and peer id for network
    let keypair = Keypair::generate_ecdsa();
    let local_peer_id = PeerId::from(keypair.public());

    //gossip protocol config
    let privacy = libp2p::gossipsub::MessageAuthenticity::Signed(keypair.clone());
    let gossip_cfg_builder = libp2p::gossipsub::ConfigBuilder::default();
    let gossip_cfg = libp2p::gossipsub::ConfigBuilder::build(&gossip_cfg_builder).unwrap();
    let gossipsub = libp2p::gossipsub::Behaviour::new(privacy, gossip_cfg).unwrap();

    //request and response protocol config
    let req_res = cbor::Behaviour::<Req, Res>::new(
        [(StreamProtocol::new("/mg/1.0"), ProtocolSupport::Full)],
        libp2p::request_response::Config::default(),
    );

    //Definition of behavior
    let mut behaviour = CustomBehav { gossipsub, req_res };

    behaviour.gossipsub.subscribe(&relay_topic.clone()).unwrap();
    behaviour.gossipsub.subscribe(&tx_topic.clone()).unwrap();
    behaviour
        .gossipsub
        .subscribe(&clients_topic.clone())
        .unwrap();

    //config swarm
    let swarm_config = libp2p::swarm::Config::with_tokio_executor()
        .with_idle_connection_timeout(Duration::from_secs(u64::MAX));

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
        .with_behaviour(|_key| behaviour)
        .unwrap()
        .with_swarm_config(|_conf| swarm_config)
        .build();
    let listener: Multiaddr = "/ip4/0.0.0.0/tcp/3369".parse().unwrap();
    swarm.listen_on(listener).unwrap();

    let mut connections: Vec<PeerId> = Vec::new();
    let mut relay_topic_subscribers: Vec<PeerId> = Vec::new();
    let mut client_topic_subscribers: Vec<PeerId> = Vec::new();
    let mut wallet_topic_subscribers: Vec<PeerId> = Vec::new();
    let mut clients: Vec<PeerId> = Vec::new();
    let mut relays: Vec<PeerId> = Vec::new();

    let mut channels: Vec<Channels> = Vec::new();
    let mut my_addresses = Vec::new();

    let server_address = "www.centichain.org:80";
    let site_connection = TcpStream::connect(server_address);

    match site_connection {
        Ok(_) => {
            let mut relays_path = "";
            if OS == "windows" {
                relays_path = "relays.dat"
            } else if OS == "linux" {
                relays_path = "/etc/relays.dat"
            }
            let addr = reqwest::get("https://centichain.org/api/relays")
                .await
                .unwrap()
                .text()
                .await
                .unwrap();
            let addresses: Addresses = serde_json::from_str(&addr).unwrap();

            let mut prev_addresses = Vec::new();
            let path_exist = fs::metadata(relays_path).is_ok();
            if path_exist {
                let relays_file = File::open(relays_path).unwrap();
                let reader = BufReader::new(relays_file);
                for i in reader.lines() {
                    let relay_addr = i.unwrap();
                    prev_addresses.push(relay_addr);
                }

                let write_file = OpenOptions::new()
                    .write(true)
                    .append(true)
                    .open(relays_path)
                    .unwrap();
                let mut writer = BufWriter::new(write_file);
                for addr in addresses.addr {
                    if !prev_addresses.contains(&addr) {
                        writeln!(writer, "{}", addr).unwrap();
                    }
                }
            } else {
                let relays_file = OpenOptions::new()
                    .write(true)
                    .append(true)
                    .create(true)
                    .open(relays_path)
                    .unwrap();
                let mut writer = BufWriter::new(relays_file);

                for addr in addresses.addr {
                    writeln!(writer, "{}", addr).unwrap();
                }
                
            }
        }
        Err(_) => {
            
        }
    }

    handle_streams(
        local_peer_id,
        &mut swarm,
        clients_topic,
        &mut my_addresses,
        &mut channels,
        &mut relays,
        &mut clients,
        relay_topic,
        &mut connections,
        &mut relay_topic_subscribers,
        &mut client_topic_subscribers,
        &mut wallet,
        &mut wallet_topic_subscribers,
    )
    .await;
}
