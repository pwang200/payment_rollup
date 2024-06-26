#![no_main]

// If you want to try std support, also update the guest Cargo.toml file
// #![no_std]  // std support is experimental
use common::common::EngineData;
use risc0_zkvm::guest::env;
risc0_zkvm::guest::entry!(main);

fn main() {
    let start = env::cycle_count();

    let mut input: EngineData = env::read();
    let verified = input.account_book.verify_partial_root();

    let output = common::l2_engine::process(&mut input).unwrap();
    env::commit(&output);

    let end = env::cycle_count();
    eprintln!("{verified}, cycle count: {}", end - start);
    assert!(verified);
}
