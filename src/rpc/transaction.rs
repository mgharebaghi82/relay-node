use std::sync::{Arc, Mutex};

use axum::{
    extract::{self, State},
    Json,
};
use libp2p::{gossipsub::IdentTopic, Swarm};
use mongodb::{
    bson::{to_document, Document},
    Collection,
};

use crate::{
    handlers::{check_trx, db_connection::blockchain_db, structures::Transaction}, write_log, CustomBehav
};

use super::server::TxRes;

pub async fn handle_transaction(
    State(swarm): State<Arc<Mutex<Swarm<CustomBehav>>>>,
    extract::Json(transaction): extract::Json<Transaction>,
) -> Json<TxRes> {
    match swarm.lock() {
        Ok(mut s) => {
            let str_gossip = serde_json::to_string(&transaction).unwrap();
            s.behaviour_mut().gossipsub.publish(IdentTopic::new("client"), str_gossip.as_bytes()).unwrap();
        }
        Err(_) => {
            write_log("error from mutext guard in transaction handle on rpc server!");
        }
    }

    //insert transaction into db at first
    let trx_todoc = to_document(&transaction).unwrap();
    let transactions_coll: Collection<Document> =
        blockchain_db().await.unwrap().collection("Transactions");
    transactions_coll.insert_one(trx_todoc, None).await.unwrap();

    //insert transaction reciept into db
    let str_trx = serde_json::to_string(&transaction).unwrap();
    check_trx::handle_transactions(str_trx).await;

    //send response to the client
    let tx_res = TxRes {
        hash: transaction.tx_hash,
        status: "sent".to_string(),
        description: "Wait for submit".to_string(),
    };
    return Json(tx_res);
}
