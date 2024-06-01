use tokio::sync::mpsc::{channel, Sender, Receiver};
use ed25519_dalek::VerifyingKey;
use tokio::time::{sleep, Duration, Instant};
use common::common::*;

use methods::{
    PAYMENT_L2_ELF//, PAYMENT_L2_ID,
};
use risc0_zkvm::{default_prover, ExecutorEnv, Receipt};

struct Prover {
    engine_data: EngineData,
    from_node: Receiver<Vec<Transaction>>,
    to_node: Sender<ResultT<Receipt>>,
}

impl Prover {
    fn spawn(
        faucet_pk: VerifyingKey,
        from_node: Receiver<Vec<Transaction>>,
        to_node: Sender<ResultT<Receipt>>,
    ) {
        tokio::spawn(async move {
            Self {
                engine_data: EngineData::new(faucet_pk.clone(), 0),
                from_node,
                to_node,
            }
                .run(faucet_pk)
                .await;
        });
    }

    async fn run(&mut self, faucet_pk: VerifyingKey) {
        loop {
            tokio::select! {
                Some(mut txns) = self.from_node.recv() =>{
                    let time_start = clock();
                    println!("Prover, from node, time {}, {} txns", time_start/1000, txns.len());

                    self.engine_data.txns.append(&mut txns);
                    {
                        // debug only
                        println!("Prover, before prove: {:?}", self.engine_data);
                    }
                    let receipt = {
                        let env = ExecutorEnv::builder()
                        .write(&self.engine_data)
                        .unwrap()
                        .build()
                        .unwrap();

                        let prover = default_prover();
                        prover.prove(env, PAYMENT_L2_ELF)
                        // .unwrap()
                    };
                    let time = clock() - time_start;
                    println!("Prover, prove time {}", time/1000);
                     {
                        // debug only
                        println!("Prover, after prove: {:?}", self.engine_data);
                     }

                    // run the execution too, since prover won't update self.engine_data
                    let time_start = clock();
                    let _header = common::l2_engine::process(&mut self.engine_data).expect("native run");
                    let time = clock() - time_start;
                    println!("Prover, native execute time {}/1000 seconds", time);

                    {
                        // debug only
                        println!("Prover, after execute: {:?}", self.engine_data);
                        println!("Prover, execute header: {:?}", _header);
                        println!("Prover, execute header hash: {:?}", _header.hash());
                        // let prover_header : BlockHeaderL2 = receipt.clone().unwrap().journal.decode().unwrap();
                        // println!("Prover, prove header: {:?}", prover_header);
                        // println!("Prover, prove header hash: {:?}", prover_header.hash());
                        println!("Prover faucet account {:?}", self.engine_data.account_book.get_account(&pk_to_hash(&faucet_pk)));
                    }

                    let receipt : ResultT<Receipt> = receipt.map_err(|_|"prover");
                    self.to_node.send(receipt).await.expect("Prover err sent to node");
                    println!("Prover, sent proof to node");
                }
            }
        }
    }
}

pub struct L2Node {
    rollup: TxSigner,
    tx_pool: Vec<Transaction>,
    prover_busy: bool,

    from_client: Receiver<Transaction>,
    from_l1: Receiver<Transaction>,
    to_l1: Sender<Transaction>,
    to_prover: Sender<Vec<Transaction>>,
    from_prover: Receiver<ResultT<Receipt>>,
}

impl L2Node {
    pub fn spawn(
        rollup: TxSigner,
        faucet_pk: VerifyingKey,
        from_client: Receiver<Transaction>,
        from_l1: Receiver<Transaction>,
        to_l1: Sender<Transaction>,
    ) {
        tokio::spawn(async move {
            let (to_prover, from_node) = channel(1);
            let (to_node, from_prover) = channel(1);

            Prover::spawn(faucet_pk, from_node, to_node);

            Self {
                rollup,
                tx_pool: vec![],
                prover_busy: false,

                from_client,
                from_l1,
                to_l1,
                to_prover,
                from_prover,
            }
                .run()
                .await;
        });
    }

    async fn run(&mut self) {
        let timer = sleep(Duration::from_millis(ONE_SECOND * 40));
        tokio::pin!(timer);

        // tracing_subscriber::fmt()
        //     .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        //     .init();

        loop {
            tokio::select! {
                Some(tx) = self.from_client.recv() =>{
                    println!("L2Node, from client, tx {:?}", tx);
                    self.tx_pool.push(tx);
                },
                Some(tx) = self.from_l1.recv() =>{
                    println!("L2Node, from l1, tx {:?}", tx);
                    self.tx_pool.push(tx);
                },
                Some(receipt) = self.from_prover.recv() => {
                    //println!("L2Node, from prover, receipt {:?}", receipt);
                    self.prover_busy = false;
                    let receipt = receipt.expect("prover error");
                    let data: Vec<u8> = bincode::serialize(&receipt).unwrap();
                    println!("L2Node, proof data size {:?}", data.len());
                    let tx = Tx::new(self.rollup.pk.clone(), self.rollup.sqn,
                        RollupStateUpdate {proof_receipt: data}, &mut self.rollup.sk);
                    self.rollup.sqn += 1;
                    self.to_l1.send(Transaction::RollupUpdate(tx)).await.expect("L2Node err sent to l1");
                    {
                        // debug only
                        let header: BlockHeaderL2 = receipt.journal.decode().unwrap();
                        println!("L2Node, sent proof to l1, header {:?}", header);
                        println!("L2Node, header hash {:?}", header.hash());
                    }
                },
                () = &mut timer => {
                    let now = clock();
                    println!("L2Node time {}, num txns {}", now/1000, self.tx_pool.len());
                    timer.as_mut().reset(Instant::now() + Duration::from_millis(ONE_SECOND*4));
                    if self.tx_pool.is_empty() || self.prover_busy {
                        continue;
                    }
                    self.prover_busy = true;
                    let mut to_send = Vec::new();
                    to_send.append(&mut self.tx_pool);
                    self.to_prover.send(to_send).await.expect("L2Node err sent to prover");
                }
            }
        }
    }
}
