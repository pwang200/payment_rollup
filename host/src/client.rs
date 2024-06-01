use tokio::sync::mpsc::{Sender};
use ed25519_dalek::VerifyingKey;
// use ed25519_dalek::{SigningKey, VerifyingKey};
// use rand::rngs::OsRng;
use tokio::time::{sleep, Duration, Instant};//, Instant};
use common::common::*;

pub struct Client {
    faucet_l1: TxSigner,
    faucet_l2: TxSigner,

    rollup_pk: VerifyingKey,
    to_l1: Sender<Transaction>,
    to_l2: Sender<Transaction>,
}

impl Client {
    pub fn spawn(
        faucet: TxSigner,
        rollup_pk: VerifyingKey,
        to_l1: Sender<Transaction>,
        to_l2: Sender<Transaction>,
    )
    {
        tokio::spawn(async move {
            Self {
                faucet_l1: faucet.clone(),
                faucet_l2: faucet,
                rollup_pk,
                to_l1,
                to_l2,
            }
                .run()
                .await;
        });
    }

    async fn run(&mut self) {
        // create rollup account on L1
        let tx = Tx::new(self.faucet_l1.pk.clone(), self.faucet_l1.sqn,
                         CreateRollupAccount { rollup_pk: self.rollup_pk.clone() },
                         &mut self.faucet_l1.sk);
        self.faucet_l1.sqn += 1;
        self.to_l1.send(Transaction::RollupCreate(tx)).await.expect("Client err sent l1");

        let tx = Tx::new(self.faucet_l1.pk.clone(), self.faucet_l1.sqn,
                         L1ToL2Deposit { rollup_pk: self.rollup_pk.clone(), amount: 100 },
                         &mut self.faucet_l1.sk);
        self.faucet_l1.sqn += 1;
        self.to_l1.send(Transaction::Deposit(tx)).await.expect("Client err sent l1");


        let timer = sleep(Duration::from_millis(ONE_SECOND * 10));
        tokio::pin!(timer);
        tokio::select! {
            () = &mut timer => {
                let now = clock();
                println!("Client time {}", now / 1000);
                let tx = Tx::new(self.faucet_l2.pk.clone(), self.faucet_l2.sqn,
                                 L2ToL1Withdrawal { amount: 100 },
                                 &mut self.faucet_l2.sk);
                self.faucet_l2.sqn += 1;
                self.to_l2.send(Transaction::Withdrawal(tx)).await.expect("Client err sent l2");
            }
        }

        timer.as_mut().reset(Instant::now() + Duration::from_millis(ONE_SECOND * 200));
        tokio::select! {
            () = &mut timer => {
                let now = clock();
                println!("Client time {}", now / 1000);
                let tx = Tx::new(self.faucet_l1.pk.clone(), self.faucet_l1.sqn,
                    L1ToL2Deposit { rollup_pk: self.rollup_pk.clone(), amount: 100 },
                    &mut self.faucet_l1.sk);
                self.faucet_l1.sqn += 1;
                self.to_l1.send(Transaction::Deposit(tx)).await.expect("Client err sent l1");
            }
        }

        timer.as_mut().reset(Instant::now() + Duration::from_millis(ONE_SECOND * 10));
        tokio::select! {
            () = &mut timer => {
                let now = clock();
                println!("Client time {}", now / 1000);
                let tx = Tx::new(self.faucet_l2.pk.clone(), self.faucet_l2.sqn,
                    L2ToL1Withdrawal { amount: 100 },
                    &mut self.faucet_l2.sk);
                self.faucet_l2.sqn += 1;
                self.to_l2.send(Transaction::Withdrawal(tx)).await.expect("Client err sent l2");
            }
        }

        // let mut a = 0u128;
        // accounts
        // let n = 1000;
        // let mut csprng = OsRng;
        // let mut alices = Vec::new();
        // for _ in 0..n {
        //     let alice_signing_key: SigningKey = SigningKey::generate(&mut csprng);
        //     let alice_verifying_key: VerifyingKey = alice_signing_key.verifying_key();
        //     alices.push((alice_signing_key, alice_verifying_key.clone()));
        // }
        // let mut idx = 0usize;
        // let n = alices.len();


        // send a payment to l1 every 4 seconds
        // loop {
        //     tokio::select! {
        //         () = &mut timer => {
        //             let now = clock();
        //             println!("Client time {}", now/1000);
        //             a += 1;
        //             idx = (idx+1)% n;
        //             let tx = Tx::new(self.faucet.pk.clone(), self.faucet.sqn, Payment { to: alices[idx].1.clone(), amount: a }, &mut self.faucet.sk);
        //             self.to_l1.send(Transaction::Pay(tx)).await.expect("Failed to send to l1");
        //             self.faucet.sqn += 1;
        //             // self.to_l2.send(Transaction::Pay(a)).await.expect("Failed to send l2tx");
        //             timer.as_mut().reset(Instant::now() + Duration::from_millis(ONE_SECOND*4));
        //         }
        //     }
        // }
    }
}
