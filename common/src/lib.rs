use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};

use ed25519_dalek::{SigningKey, Verifier};
use ed25519_dalek::Signature;
use ed25519_dalek::VerifyingKey;
use ed25519_dalek::ed25519::signature::SignerMut;

//monotree = { git = "https://github.com/pwang200/monotree.git" }
use monotree::Monotree;

pub const HASH_LEN: usize = 32;

pub type Hash = [u8; HASH_LEN];
pub type AccountID = Hash;
pub type PaymentTxns = Vec<PaymentTx>;


pub fn pk_to_hash(pk: &VerifyingKey) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(pk.as_bytes());
    let x: Hash = hasher.finalize().as_slice().try_into().expect("hash");
    x
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PaymentTxPayLoad {
    //TODO cross platform and cross language serialization for hashing
    from: VerifyingKey,
    to: VerifyingKey,
    amount: u128,
    sqn: u32,
}

impl PaymentTxPayLoad {
    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(self.from.as_bytes());
        hasher.update(self.to.as_bytes());
        hasher.update(self.amount.to_be_bytes());
        hasher.update(self.sqn.to_be_bytes());
        let x: Hash = hasher.finalize().as_slice().try_into().expect("hash");
        x
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PaymentTx {
    payload: PaymentTxPayLoad,
    sig: Signature,
}

impl PaymentTx {
    pub fn new(from: VerifyingKey,
               to: VerifyingKey,
               amount: u128,
               sqn: u32,
               signing_key: &mut SigningKey,
    ) -> PaymentTx {
        let payload = PaymentTxPayLoad {
            from,
            to,
            amount,
            sqn,
        };
        //let data: Vec<u8> = bincode::serialize(&payload).unwrap();
        let sig: Signature = signing_key.sign(&payload.hash());
        PaymentTx { payload, sig }
    }

    pub fn verify(&self) -> bool {
        //let data: Vec<u8> = bincode::serialize(&self.payload).unwrap(); &*data
        self.payload.from.verify(&self.payload.hash(), &self.sig).is_ok()
    }

    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(self.payload.hash());
        hasher.update(self.sig.to_bytes());//TODO data copy
        let x: Hash = hasher.finalize().as_slice().try_into().expect("hash");
        x
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionSet {
    //parent: Hash,
    //sqn: u32,
    txns: PaymentTxns,
}

impl TransactionSet {
    pub fn new() -> TransactionSet {//parent: Hash, sqn: u32
        TransactionSet { txns: vec![] }
    }

    pub fn add_tx(&mut self, tx: PaymentTx) {
        self.txns.push(tx);
    }

    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        // hasher.update(self.parent);
        // hasher.update(self.sqn.to_be_bytes());
        for tx in &self.txns {
            hasher.update(tx.hash());
        }
        let x: Hash = hasher.finalize().as_slice().try_into().expect("hash");
        x
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Account {
    owner: VerifyingKey,
    amount: u128,
    sqn_expect: u32,
}

impl Account {
    pub fn new(owner: VerifyingKey,
               amount: u128) -> Account
    {
        Account { owner, amount, sqn_expect: 0 }
    }

    pub fn pay_out(&mut self, amount: u128) -> bool {
        if self.amount >= amount {
            self.amount -= amount;
            true
        } else {
            false
        }
    }

    pub fn credit(&mut self, amount: u128) {
        assert!(self.amount <= u128::MAX - amount);
        self.amount += amount;
    }

    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(self.owner.as_bytes());
        hasher.update(self.amount.to_be_bytes());
        hasher.update(self.sqn_expect.to_be_bytes());
        //let result = hasher.finalize();
        let x: Hash = hasher.finalize().as_slice().try_into().expect("Wrong");
        x
    }

    pub fn id(&self) -> Hash {
        pk_to_hash(&self.owner)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AccountBook {
    proof_tree: Monotree,
    pub root: Hash,
    accounts: HashMap<AccountID, Account>,
}

impl AccountBook {
    pub fn new(faucet_key: VerifyingKey, faucet_amout: u128) -> AccountBook {
        let mut tree = Monotree::default();
        let root = None;
        let mut b = HashMap::new();
        let a = Account::new(faucet_key, faucet_amout);
        let h = a.id();
        let hh = a.hash();
        b.insert(h, a);
        let r = tree.insert(root.as_ref(), &h, &hh).unwrap().unwrap();
        AccountBook { proof_tree: tree, root: r, accounts: b }
    }

    pub fn get_account(&self, a: &AccountID) -> Option<&Account> {
        self.accounts.get(a)
    }

    pub fn get_num_accounts(&self) -> usize {
        self.accounts.len()
    }

    fn process_payment(&mut self, payment: &PaymentTx) -> Result<((AccountID, Hash), (AccountID, Hash)), &'static str> {
        if !payment.verify() {
            return Err("sig");
        }
        let id_f = pk_to_hash(&payment.payload.from);
        match self.accounts.get_mut(&id_f) {
            None => Err("No payer account"),
            Some(a_f) => {
                if a_f.sqn_expect != payment.payload.sqn {
                    return Err("sqn");
                }
                if a_f.amount < payment.payload.amount {
                    return Err("balance");
                }
                a_f.amount -= payment.payload.amount;
                a_f.sqn_expect += 1;
                let a_f_h = a_f.hash();

                let id_t = pk_to_hash(&payment.payload.to);
                let hashes = match self.accounts.get_mut(&id_t) {
                    None => {
                        let a_t = Account::new(payment.payload.to, payment.payload.amount);//TODO lifetime
                        let a_t_h = a_t.hash();
                        self.accounts.insert(id_t, a_t);
                        ((id_f, a_f_h), (id_t, a_t_h))
                    }
                    Some(a_t) => {
                        a_t.amount += payment.payload.amount;
                        let a_t_h = a_t.hash();
                        ((id_f, a_f_h), (id_t, a_t_h))
                    }
                };
                Ok(hashes)
            }
        }
    }

    pub fn process_payment_txns(&mut self, payments: &TransactionSet) -> Vec<u8> {
        let mut to_update = std::collections::HashMap::new();
        let mut results = Vec::new();
        for payment in &payments.txns {
            match self.process_payment(payment) {
                Ok(v) => {
                    to_update.insert(v.0.0, v.0.1);
                    to_update.insert(v.1.0, v.1.1);
                    results.push(1);
                }
                Err(_) => {
                    results.push(0);
                }
            }
        }
        let mut ids = Vec::new();
        let mut vs = Vec::new();
        for (k, v) in to_update.drain() {
            ids.push(k);
            vs.push(v);
        }
        self.root = self.proof_tree.inserts(Some(&self.root), &ids, &vs).unwrap().unwrap();
        results
    }

    #[cfg(test)]
    fn hash_verify(&mut self, pk: &VerifyingKey, is_valid: impl Fn(&Account) -> bool) -> bool {
        // has account
        // account info correct
        // computed account hash is the same as Merkle tree leaf
        // can get proof
        // proof verifies
        let id = pk_to_hash(pk);
        let account = self.accounts.get(&id);
        if account.is_none() {
            return false;
        }
        let account = account.unwrap();
        if !is_valid(account) {
            return false;
        }
        let account_hash = account.hash();

        let leaf = self.proof_tree.get(Some(&self.root), &id);
        if leaf.is_err() {
            return false;
        }
        let leaf = leaf.unwrap();
        if leaf.is_none() {
            return false;
        }
        let leaf = leaf.unwrap();
        if account_hash != leaf {
            return false;
        }

        let proof = self.proof_tree.get_merkle_proof(Some(&self.root), &id).unwrap();
        monotree::verify_proof(Some(&self.root), &leaf, proof.as_ref())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BlockHeader {
    parent: Hash,
    state_root: Hash,
    sqn: u32,
    txns_hash: Hash,
    ex_results: Vec<u8>, //TODO
}

impl BlockHeader {
    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(self.parent);
        hasher.update(self.state_root);
        hasher.update(self.sqn.to_be_bytes());
        hasher.update(self.txns_hash);
        hasher.update(self.ex_results.as_slice());
        let x: Hash = hasher.finalize().as_slice().try_into().expect("hash");
        x
    }
}


#[derive(Serialize, Deserialize, Debug)]
pub struct EngineInput {
    pub parent: Hash,
    //TODO
    pub account_book: AccountBook,
    pub txns: TransactionSet,
    pub sqn: u32,
}

impl EngineInput {
    pub fn process(&mut self) -> BlockHeader {
        let txns_hash = self.txns.hash();
        let results = self.account_book.process_payment_txns(&self.txns);

        BlockHeader { parent: self.parent, state_root: self.account_book.root, txns_hash: txns_hash, sqn: self.sqn, ex_results: results }
    }

    pub fn new_block(&mut self, parent: Hash, txns: TransactionSet) {
        self.parent = parent;
        self.txns = txns;
        self.sqn += 1;
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;

    // run the test with the following command, note the manifest-path is relative
    // RUST_BACKTRACE=1 cargo test --lib tests::block_process_works --manifest-path ./common/Cargo.toml
    #[test]
    fn block_process_works() {
        let n = 33usize;
        let genesis_amount = 1000000u128;
        let payment_amount = 10u128;
        let mut csprng = OsRng;
        let mut faucet_signing_key: SigningKey = SigningKey::generate(&mut csprng);
        let faucet_verifying_key: VerifyingKey = faucet_signing_key.verifying_key();
        let book = AccountBook::new(faucet_verifying_key, genesis_amount);

        /////////////////////////////////////////////////////
        // create txns
        let mut txns = TransactionSet::new();
        let mut alices = Vec::new();
        for i in 0..n {
            let alice_signing_key: SigningKey = SigningKey::generate(&mut csprng);
            let alice_verifying_key: VerifyingKey = alice_signing_key.verifying_key();
            alices.push((alice_signing_key, alice_verifying_key.clone()));
            txns.add_tx(PaymentTx::new(faucet_verifying_key, alice_verifying_key, payment_amount, i as u32, &mut faucet_signing_key));
        }
        assert_eq!(alices.len(), n);
        // process txns in one block
        let mut input = EngineInput { parent: Hash::default(), account_book: book, txns: txns, sqn: 0 };
        let header = input.process();
        assert_eq!(header.ex_results.len(), n);
        assert_eq!(header.sqn, 0);
        for r in &header.ex_results {
            assert_eq!(*r, 1u8);
        }
        // n accounts are created
        for (_, pk) in &alices {
            assert!(input.account_book.hash_verify(pk, |a| a.sqn_expect == 0 && a.amount == payment_amount && a.owner == *pk));
        }
        // genesis account
        assert!(input.account_book.hash_verify(&faucet_verifying_key, |a| a.sqn_expect == n as u32 && a.amount == genesis_amount - payment_amount * n as u128 && a.owner == faucet_verifying_key));

        /////////////////////////////////////////////////////
        // more txns
        let mut txns = TransactionSet::new();
        for (sk, pk) in &mut alices {
            txns.add_tx(PaymentTx::new(pk.clone(), faucet_verifying_key, payment_amount, 0u32, sk));
        }
        input.new_block(header.hash(), txns);
        let header_new = input.process();
        assert_eq!(header_new.ex_results.len(), n);
        assert_eq!(header_new.sqn, 1);
        for r in header_new.ex_results {
            assert_eq!(r, 1);
        }
        assert_eq!(header_new.parent, header.hash());
        // n accounts
        for (_, pk) in &alices {
            assert!(input.account_book.hash_verify(pk, |a| a.sqn_expect == 1 && a.amount == 0 && a.owner == *pk));
        }
        // genesis account
        assert!(input.account_book.hash_verify(&faucet_verifying_key, |a| a.sqn_expect == n as u32 && a.amount == genesis_amount && a.owner == faucet_verifying_key));
    }
}
