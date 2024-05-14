#![no_main]
// If you want to try std support, also update the guest Cargo.toml file
// #![no_std]  // std support is experimental
use common::{AccountBook, PaymentTx, PaymentTxns, EngineInput};
use risc0_zkvm::guest::env;
risc0_zkvm::guest::entry!(main);

fn main() {
    let mut input: EngineInput = env::read();
    let header = input.process();
    env::commit(&header);
}
