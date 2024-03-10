use relay_node::handle_requests;
use relay_node::run;
use relay_node::SWARM;
// use std::sync::Arc;

#[tokio::main]
async fn main() {
    let (_, _) = tokio::join!(
        run(SWARM.0.clone(), SWARM.1),
        handle_requests()
    );
}
