use tokio::sync::mpsc::{Sender, Receiver};
use ed25519_dalek::VerifyingKey;
use tokio::time::{sleep, Duration, Instant};
use common::common::*;

use methods::{
    PAYMENT_L2_ELF//, PAYMENT_L2_ID,
};
use risc0_zkvm::{default_prover, ExecutorEnv};

pub struct L2Node {
    rollup: TxSigner,
    rollup_sqn: u32,

    from_client: Receiver<Transaction>,
    from_l1: Receiver<Transaction>,
    to_l1: Sender<Transaction>,

    engine_data: EngineData,
    tx_pool: Vec<Transaction>,
}

impl L2Node {
    pub fn spawn(
        rollup: TxSigner,
        faucet_pk: VerifyingKey,
        from_client: Receiver<Transaction>,
        from_l1: Receiver<Transaction>,
        to_l1: Sender<Transaction>,
    )
    {
        tokio::spawn(async move {
            Self {
                rollup,
                rollup_sqn: 0u32,

                from_client,
                from_l1,
                to_l1,

                engine_data: EngineData::new(faucet_pk, GENESIS_AMOUNT),
                tx_pool: vec![],
            }
                .run()
                .await
        });
    }

    async fn run(&mut self) {
        let timer = sleep(Duration::from_millis(ONE_SECOND * 40));
        tokio::pin!(timer);

        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
            .init();

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
                () = &mut timer => {
                    let now = clock();
                    println!("L2Node time {}, num txns {}", now/1000, self.tx_pool.len());
                    timer.as_mut().reset(Instant::now() + Duration::from_millis(ONE_SECOND*4));
                    if self.tx_pool.is_empty(){
                        continue;
                    }

                    self.engine_data.txns.append(&mut self.tx_pool);

                    let receipt = {
                        let env = ExecutorEnv::builder()
                        .write(&self.engine_data)
                        .unwrap()
                        .build()
                        .unwrap();

                        let prover = default_prover();
                        prover
                        .prove(env, PAYMENT_L2_ELF)
                        .unwrap()
                    };

                    let data: Vec<u8> = bincode::serialize(&receipt).unwrap();
                    let tx = Tx::new(self.rollup.pk.clone(), self.rollup_sqn,
                        RollupStateUpdate {proof_receipt: data}, &mut self.rollup.sk);
                    self.rollup.sqn += 1;
                    self.to_l1.send(Transaction::RollupUpdate(tx)).await.expect("L2Node err sent l1");

                    let header: BlockHeaderL2 = receipt.journal.decode().unwrap();
                    println!("L2Node, sent proof to l1, header {:?}", header);
                }
            }
        }
    }
}
