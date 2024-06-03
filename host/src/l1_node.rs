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
        rollup_pk: VerifyingKey,
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
                engine_data: EngineData::new(faucet_pk.clone(), GENESIS_AMOUNT),
                tx_pool: vec![],
            }
                .run(faucet_pk, rollup_pk)
                .await;
        });
    }

    async fn run(&mut self, faucet_pk: VerifyingKey, rollup_pk: VerifyingKey) {
        let timer = sleep(Duration::from_millis(ONE_SECOND * 4));
        tokio::pin!(timer);

        loop {
            tokio::select! {
                Some(tx) = self.from_client.recv() =>{
                    println!("L1Node, from client, tx {:?}", tx);
                    self.tx_pool.push(tx);
                },
                Some(tx) = self.from_l2.recv() =>{
                    //println!("L1Node, from l2, tx {:?}", tx);
                    println!("L1Node, from l2");
                    self.tx_pool.push(tx);
                },
                () = &mut timer => {
                    let now = clock();
                    println!("L1Node time {}, num txns {}", now/1000, self.tx_pool.len());
                    timer.as_mut().reset(Instant::now() + Duration::from_millis(ONE_SECOND*4));
                    if self.tx_pool.is_empty() {
                        continue;
                    }

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
                            {
                                // debug only
                                println!("L1Node faucet account {:?}", self.engine_data.account_book.get_account(&pk_to_hash(&faucet_pk)));
                                println!("L1Node rollup account {:?}", self.engine_data.account_book.get_account(&pk_to_hash(&rollup_pk)));
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

