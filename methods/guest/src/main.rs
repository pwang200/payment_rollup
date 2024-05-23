#![no_main]
// If you want to try std support, also update the guest Cargo.toml file
// #![no_std]  // std support is experimental
use common::common::EngineData;
use risc0_zkvm::guest::env;
risc0_zkvm::guest::entry!(main);

fn main() {
    let mut input: EngineData = env::read();
    let output = common::l2_engine::process(&mut input).unwrap();
    env::commit(&output);
}
