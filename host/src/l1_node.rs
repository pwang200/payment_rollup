use common::common::*;
use methods::PAYMENT_L2_ID;
use tokio::sync::mpsc::{Sender, Receiver};
use ed25519_dalek::{VerifyingKey};
use tokio::time::{sleep, Duration, Instant};
use risc0_zkvm::Receipt;


pub struct L1Node {
    from_client: Receiver<Transaction>,
    from_l2: Receiver<Transaction>,
    to_l2: Sender<Transaction>,

    engine_data: EngineData,
    tx_pool: Vec<Transaction>,
}

impl L1Node {
    pub fn spawn(
        faucet_pk: VerifyingKey,
        from_client: Receiver<Transaction>,
        from_l2: Receiver<Transaction>,
        to_l2: Sender<Transaction>,
    )
    {
        tokio::spawn(async move {
            Self {
                from_client,
                from_l2,
                to_l2,
                engine_data: EngineData::new(faucet_pk, GENESIS_AMOUNT),
                tx_pool: vec![],
            }
                .run()
                .await
        });
    }

    async fn run(&mut self){
        let timer = sleep(Duration::from_millis(ONE_SECOND*4));
        tokio::pin!(timer);

        loop {
            tokio::select! {
                Some(tx) = self.from_client.recv() =>{
                    println!("L1Node, from client, tx {:?}", tx);
                    self.tx_pool.push(tx);
                },
                Some(tx) = self.from_l2.recv() =>{
                    println!("L1Node, from l2, tx {:?}", tx);
                    self.tx_pool.push(tx);
                },
                () = &mut timer => {
                    let now = clock();
                    println!("L1Node time {}, num txns {}", now/1000, self.tx_pool.len());
                    timer.as_mut().reset(Instant::now() + Duration::from_millis(ONE_SECOND*4));
                    self.engine_data.txns.append(&mut self.tx_pool);
                    match common::l1_engine::process(&mut self.engine_data, |data|
                    {
                        let receipt: Receipt = bincode::deserialize(data).unwrap();
                        if receipt.verify(PAYMENT_L2_ID).is_ok() {
                            let header: BlockHeaderL2 = receipt.journal.decode().unwrap();
                            Ok(header)
                        } else {
                            Err("receipt decode")
                        }
                    }){
                        Ok(header) => {
                            for d in header.deposits{
                                match self.to_l2.send(Transaction::DepositL2(d)).await{
                                    Ok(_) => {}
                                    Err(e) => {
                                        println!("L1Node err sent l2 {:?}", e);
                                        break;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            println!("L1Node err process tx {}", e);
                            break;
                        }
                    }
                }
            }
        }
    }
}


//
// pub fn process(input : &mut EngineData,
//                valid_receipt: impl Fn(&Vec<u8>) -> ResultT<BlockHeaderL2>) -> ResultT<BlockHeaderL1> {
//     let txns_hash = tx_set_hash(&input.txns);
//     let mut to_update = std::collections::HashMap::new();
//     let mut deposits = Vec::new();
//     for t in &input.txns {
//         let mut updates = match t {
//             Transaction::Pay(tx) => {
//                 input.account_book.process_payment(tx)?
//             }
//             Transaction::Deposit(tx) => {
//                 let r = input.account_book.process_deposit_l1(tx)?;
//                 deposits.push((*tx).clone());
//                 r
//             }
//             Transaction::RollupCreate(tx) => {
//                 input.account_book.process_create_rollup_account(tx)?
//             }
//             Transaction::RollupUpdate(tx) => {
//                 input.account_book.process_rollup_state_update(tx, &valid_receipt)?
//             }
//             // |data|
//             // {
//             // let receipt: Receipt = bincode::deserialize(data).unwrap();
//             // if receipt.verify(PAYMENT_L2_ID).is_ok() {
//             // let header: BlockHeaderL2 = receipt.journal.decode().unwrap();
//             // Ok(header)
//             // } else {
//             // Err("receipt decode")
//             // }
//             // }
//             _ => {
//                 return Err("tx type");
//             }
//         };
//         for (k, v) in updates.drain(..) {
//             to_update.insert(k, v);
//         }
//     }
//     input.account_book.update_tree(to_update);
//     input.txns.clear();
//
//     Ok(BlockHeaderL1 {
//         parent: input.parent,
//         state_root: input.account_book.root,
//         sqn: input.sqn,
//         txns_hash,
//         deposits,
//     })
// }
