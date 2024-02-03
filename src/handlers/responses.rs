use libp2p::{PeerId, Swarm};

use super::{
    create_log::write_log, structures::{Channels, CustomBehav, Res, ResForReq}
};

pub async fn handle_responses(
    response: Res,
    local_peer_id: PeerId,
    channels: &mut Vec<Channels>,
    swarm: &mut Swarm<CustomBehav>,
    client_topic_subscriber: &mut Vec<PeerId>,
    relay_topic_subscribers: &mut Vec<PeerId>,
) {
    if let Ok(mut res) = serde_json::from_str::<ResForReq>(&response.res.clone()) {
        match res.peer.last() {
            Some(last_peerid) => {
                if last_peerid == &local_peer_id {
                    res.peer.pop();
                    let new_res = serde_json::to_string(&res).unwrap();
                    let new_response = Res { res: new_res };
                    let i_channels = channels
                        .iter()
                        .position(|channel| channel.peer == res.peer.last().unwrap().clone());
                    if client_topic_subscriber.contains(res.peer.last().unwrap())
                        || relay_topic_subscribers.contains(res.peer.last().unwrap())
                    {
                        match i_channels {
                            Some(index) => {
                                match swarm
                                    .behaviour_mut()
                                    .req_res
                                    .send_response(channels.remove(index).channel, new_response)
                                {
                                    Ok(_) => (),
                                    Err(e) => write_log(format!("Error from response:\n{:#?}", e)),
                                }
                            }
                            None => {}
                        }
                    }
                } else {
                    let i_channels = channels
                        .iter()
                        .position(|channel| channel.peer == res.peer.last().unwrap().clone());
                    if client_topic_subscriber.contains(res.peer.last().unwrap())
                        || relay_topic_subscribers.contains(res.peer.last().unwrap())
                    {
                        match i_channels {
                            Some(index) => {
                                match swarm
                                    .behaviour_mut()
                                    .req_res
                                    .send_response(channels.remove(index).channel, response.clone())
                                {
                                    Ok(_) => (),
                                    Err(e) => write_log(format!(
                                        "Error from second else resposne:\n{:#?}",
                                        e
                                    )),
                                }
                            }
                            None => {}
                        }
                    } else {
                        match i_channels {
                            Some(index) => {
                                match swarm
                                    .behaviour_mut()
                                    .req_res
                                    .send_response(channels.remove(index).channel, response.clone())
                                {
                                    Ok(_) => (),
                                    Err(_) => {
                                        write_log("error from third else response!".to_string())
                                    }
                                }
                            }
                            None => {}
                        }
                    }
                }
            }
            None => {}
        }
    }
}
