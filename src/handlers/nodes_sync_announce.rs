use super::{structures::FullNodes, structures::ImSync};

pub fn handle_sync_message(fullnode_subs: &mut Vec<FullNodes>, str_msg: &String) {
    if let Ok(new_sync_node) = serde_json::from_str::<ImSync>(&str_msg) {
        let new_fullnode = FullNodes {
            relay: new_sync_node.relay,
            peer_id: new_sync_node.peerid,
            waiting: (fullnode_subs.len() + 1) as i64 * 2,
            public_key: new_sync_node.public_key,
        };
        let mut fullnodes_pid = Vec::new();
        for i in fullnode_subs.clone() {
            fullnodes_pid.push(i.peer_id.clone());
        }
        if !fullnodes_pid.contains(&new_fullnode.peer_id) {
            fullnode_subs.push(new_fullnode);
        }
    }
}
