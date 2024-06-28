use common::common::*;

use methods::{PAYMENT_L2_ELF, PAYMENT_L2_ID};
use risc0_zkvm::{default_prover, ExecutorEnv};
use rand::rngs::OsRng;
use clap::Parser;

#[derive(Parser, Debug)]
// #[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = 10)]
    network_size: usize,

    #[arg(short, long, default_value_t = 10)]
    transactions: usize,
}

fn create_input(network_size: usize, num_txns: usize) -> EngineData {
    let mut csprng = OsRng;
    let mut signers = vec![];
    let mut keys = vec![];
    for _ in 0..network_size {
        let f = TxSigner::new(SigningKey::random(&mut csprng));
        keys.push(f.pk);
        signers.push(f);
    }
    let mut engine_data = EngineData::new_batch(keys, 1_000_000_000_000);

    let mut txns = vec![];
    let amount = 1u128;
    for i in 0..num_txns {
        let to = signers[(i + 1) % network_size].pk;
        let from = &mut signers[i % network_size];
        txns.push(Transaction::Pay(Tx::new(from.pk, from.sqn, Payment { to, amount }, &mut from.sk)));
        from.sqn += 1;
    }
    engine_data.txns = txns;
    engine_data.get_partial()
}

fn main() {
    let args = Args::parse();
    println!("network size: {}, number of transactions: {}", args.network_size, args.transactions);
    assert!(args.network_size > 0 && args.transactions > 0);
    let mut input = create_input(args.network_size, args.transactions);
    assert!(input.account_book.verify_partial_root());

    let time_start = clock();
    let env = ExecutorEnv::builder()
        .write(&input)
        .unwrap()
        .build()
        .unwrap();
    let prover = default_prover();
    let receipt = prover.prove(env, PAYMENT_L2_ELF).unwrap();
    let time = clock() - time_start;
    println!("Prover, prove time {}", time / 1000);
    receipt.verify(PAYMENT_L2_ID).expect("proof verification failed");
    let header: BlockHeaderL2 = receipt.journal.decode().unwrap();
    println!("Prover, header hash: {:?}", header.hash());

    let host_header = common::l2_engine::process(&mut input).expect("native run");
    println!("Host,   header hash: {:?}", host_header.hash());
}
