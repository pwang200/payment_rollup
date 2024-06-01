use tokio::signal;
use tokio::sync::mpsc::channel;

use rand::rngs::OsRng;
use ed25519_dalek::SigningKey;
use common::common::TxSigner;
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
    let faucet = TxSigner::new(SigningKey::generate(&mut csprng));
    let rollup = TxSigner::new(SigningKey::generate(&mut csprng));

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



// prove is slow, use dev mode for testing:
// RISC0_DEV_MODE=true cargo run --color=always --bin host --manifest-path ./host/Cargo.toml
// fn main() {
//     // 1 tx:  3m18.492s, with 24 core CPU
//     // 5 tx: 15m31.657s
//     //let num_txns = 5u32;
//
//     // Initialize tracing. In order to view logs, run `RUST_LOG=info cargo run`
//     tracing_subscriber::fmt()
//         .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
//         .init();
//
//     // let mut csprng = OsRng;
//     // let mut faucet_signing_key: SigningKey = SigningKey::generate(&mut csprng);
//     // let faucet_verifying_key: VerifyingKey = faucet_signing_key.verifying_key();
//     //
//     // let book = AccountBook::new(faucet_verifying_key, 1000000u128);
//     // let mut txns = TransactionSet::new();
//     // for i in 0..num_txns {
//     //     let alice_signing_key: SigningKey = SigningKey::generate(&mut csprng);
//     //     let alice_verifying_key: VerifyingKey = alice_signing_key.verifying_key();
//     //     txns.add_tx(PaymentTx::new(faucet_verifying_key, alice_verifying_key, 10, i, &mut faucet_signing_key));
//     // }
//     //
//     // let mut input = EngineInput { parent: Hash::default(), account_book: book, txns: txns, sqn: 0 };
//     let input: u32 = 15 * u32::pow(2, 27) + 1;
//     let env = ExecutorEnv::builder()
//         .write(&input)
//         .unwrap()
//         .build()
//         .unwrap();
//
//     // let env = ExecutorEnv::builder()
//     // .write(&input)
//     //     .unwrap()
//     //     .build()
//     //     .unwrap();
//
//     // Obtain the default prover.
//     let prover = default_prover();
//
//     // Produce a receipt by proving the specified ELF binary.
//     let receipt = prover
//         .prove(env, PAYMENT_L2_ELF)
//         .unwrap();
//
//     // Retrieving receipt journal
//     // let header_guest: BlockHeader = receipt.journal.decode().unwrap();
//     let output : u32 = receipt.journal.decode().unwrap();
//     println!("output: {}", output);
//     // The receipt was verified at the end of proving, but the below code is an
//     // example of how someone else could verify this receipt.
//     receipt
//         .verify(PAYMENT_L2_ID)
//         .unwrap();
//
//     // only for testing:
//     // process input in host, and compare the resulting block header with the guest output header
//     // let header_host = input.process();
//     // println!("header_host : {:?}", header_host);
//     // println!("header_guest: {:?}", header_guest);
//     // println!("header_host hash : {:?}", header_host.hash());
//     // println!("header_guest hash: {:?}", header_guest.hash());
//     // println!("same header hash: {}", header_guest.hash() == header_host.hash());
// }
