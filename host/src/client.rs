use tokio::sync::mpsc::{Sender};
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use tokio::time::{sleep, Duration, Instant};
use common::common::*;

pub struct Client{
    faucet_sk: SigningKey,
    faucet_pk: VerifyingKey,
    faucet_sqn: u32,

    rollup_pk: VerifyingKey,
    to_l1: Sender<Transaction>,
    to_l2: Sender<Transaction>,
}

impl Client {
    pub fn spawn(
        faucet_sk: SigningKey,
        faucet_pk: VerifyingKey,
        rollup_pk: VerifyingKey,
        to_l1: Sender<Transaction>,
        to_l2: Sender<Transaction>,
    )
    {
        tokio::spawn(async move {
            Self {
                faucet_sk,
                faucet_pk,
                faucet_sqn: 0u32,
                rollup_pk,
                to_l1,
                to_l2,
            }
                .run()
                .await
        });
    }

    async fn run(&mut self){
        let timer = sleep(Duration::from_millis(ONE_SECOND*4));
        tokio::pin!(timer);
        let mut a = 0u128;

        // accounts
        let n = 1000;
        let mut csprng = OsRng;
        let mut alices = Vec::new();
        for _ in 0..n {
            let alice_signing_key: SigningKey = SigningKey::generate(&mut csprng);
            let alice_verifying_key: VerifyingKey = alice_signing_key.verifying_key();
            alices.push((alice_signing_key, alice_verifying_key.clone()));
        }
        let mut idx = 0usize;
        let n = alices.len();

        // create rollup account on L1
        let tx = Tx::new(self.faucet_pk.clone(), self.faucet_sqn,
                         CreateRollupAccount {rollup_pk: self.rollup_pk.clone()},
                         &mut self.faucet_sk);
        self.faucet_sqn += 1;
        self.to_l1.send(Transaction::RollupCreate(tx)).await.expect("Client err sent l1");

        // send a payment to l1 every 4 seconds
        loop {
            tokio::select! {
                () = &mut timer => {
                    let now = clock();
                    println!("Client time {}", now/1000);
                    a += 1;
                    idx = (idx+1)% n;
                    let tx = Tx::new(self.faucet_pk.clone(), self.faucet_sqn, Payment { to: alices[idx].1.clone(), amount: a }, &mut self.faucet_sk);
                    self.to_l1.send(Transaction::Pay(tx)).await.expect("Failed to send to l1");
                    self.faucet_sqn += 1;
                    // self.to_l2.send(Transaction::Pay(a)).await.expect("Failed to send l2tx");
                    timer.as_mut().reset(Instant::now() + Duration::from_millis(ONE_SECOND*4));
                }
            }
        }
    }
}
