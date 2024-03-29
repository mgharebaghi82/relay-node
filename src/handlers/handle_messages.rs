use libp2p::{
    gossipsub::{IdentTopic, Message},
    PeerId, Swarm,
};

use super::{
    check_trx::handle_transactions,
    nodes_sync_announce::handle_sync_message,
    recieved_block::verifying_block,
    structures::{FullNodes, GossipMessage},
    CustomBehav
};

//check gossip messages and do its operations.....................................................................
pub async fn msg_check(
    message: Message,
    mut leader: &mut String,
    fullnodes: &mut Vec<FullNodes>,
    relays: &mut Vec<PeerId>,
    propagation_source: PeerId,
    swarm: &mut Swarm<CustomBehav>,
    connections: &mut Vec<PeerId>,
    local_peer_id: PeerId,
) {
    let str_msg = String::from_utf8(message.data.clone()).unwrap();

    handle_sync_message(fullnodes, &str_msg);

    handle_transactions(String::from_utf8(message.data).unwrap()).await;

    match verifying_block(&str_msg, &mut leader, fullnodes).await {
        Ok(_) => {
            //send true block to sse servers
            let sse_topic = IdentTopic::new("sse");
            match swarm
                .behaviour_mut()
                .gossipsub
                .publish(sse_topic, str_msg.clone().as_bytes())
            {
                Ok(_) => {}
                Err(_) => {}
            }

            //send true block to connected Validators
            let validators_topic = IdentTopic::new(local_peer_id.to_string());
            match swarm
                .behaviour_mut()
                .gossipsub
                .publish(validators_topic, str_msg.clone().as_bytes())
            {
                Ok(_) => {}
                Err(_) => {}
            }
        }
        Err(e) => {
            if e != "reject" {
                let gossipmsg: GossipMessage = serde_json::from_str(&str_msg).unwrap();
                let c_index = fullnodes.iter().position(|node| {
                    node.peer_id == gossipmsg.block.header.validator.parse().unwrap()
                });
                match c_index {
                    Some(i) => {
                        fullnodes.remove(i);
                    }
                    None => {}
                }

                let r_index = relays.iter().position(|relay| relay == &propagation_source);
                match r_index {
                    Some(i) => {
                        relays.remove(i);
                        if connections.contains(&propagation_source) {
                            swarm.disconnect_peer_id(propagation_source).unwrap();
                        }
                    }
                    None => {}
                }
            }
        }
    }
}
