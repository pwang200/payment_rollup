use tokio::signal;
use tokio::sync::mpsc::channel;

use rand::rngs::OsRng;
use common::common::{SigningKey, TxSigner};
use crate::client::Client;
use crate::l1_node::L1Node;
use crate::l2_node::L2Node;

mod l1_node;
mod client;
mod l2_node;

pub const CHANNEL_CAPACITY: usize = 1_000;

#[tokio::main]
async fn main()
{
    let (tx_client_l1, rx_client_l1) = channel(CHANNEL_CAPACITY);
    let (tx_client_l2, rx_client_l2) = channel(CHANNEL_CAPACITY);
    let (tx_l1_l2, rx_l1_l2) = channel(CHANNEL_CAPACITY);
    let (tx_l2_l1, rx_l2_l1) = channel(CHANNEL_CAPACITY);

    let mut csprng = OsRng;
    let faucet = TxSigner::new(SigningKey::random(&mut csprng));
    let rollup = TxSigner::new(SigningKey::random(&mut csprng));

    //rollup account setup
    let f_pk = faucet.pk.clone();
    let r_pk = rollup.pk.clone();
    L1Node::spawn(f_pk.clone(), r_pk.clone(), rx_client_l1, rx_l2_l1, tx_l1_l2);
    L2Node::spawn(rollup, f_pk, rx_client_l2, rx_l1_l2, tx_l2_l1);
    Client::spawn(faucet, r_pk, tx_client_l1, tx_client_l2);
    println!("spawned");
    // sleep(Duration::from_millis(1000_000_000)).await;
    match signal::ctrl_c().await {
        Ok(()) => {
            println!("Done!");
        }
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
            // we also shut down in case of error
        }
    }
}
