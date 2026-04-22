//! Block implement of blockchain

use super::*;
use crate::transaction::Transaction;
use sha2::{Sha256, Digest};
use anyhow::anyhow;
use merkle_cbt::merkle_tree::CBMT;
use merkle_cbt::merkle_tree::Merge;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

const TARGET_HEXS: usize = 4;

/// Block keeps block headers
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    timestamp: u128,
    transactions: Vec<Transaction>,
    prev_block_hash: String,
    hash: String,
    nonce: i32,
    height: i32,
    difficulty: usize,
}

impl Block {
    pub fn get_hash(&self) -> String {
        self.hash.clone()
    }

    pub fn get_prev_hash(&self) -> String {
        self.prev_block_hash.clone()
    }

    pub fn get_transaction(&self) -> &Vec<Transaction> {
        &self.transactions
    }

    pub fn get_height(&self) -> i32 {
        self.height
    }

    pub fn get_difficulty(&self) -> usize {
        self.difficulty
    }

    pub fn get_timestamp(&self) -> u128 {
        self.timestamp
    }

    /// NewBlock creates and returns Block
    pub fn new_block(
        transactions: Vec<Transaction>,
        prev_block_hash: String,
        height: i32,
        difficulty: usize,
    ) -> Result<Block> {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_millis();
        let mut block = Block {
            timestamp,
            transactions,
            prev_block_hash,
            hash: String::new(),
            nonce: 0,
            height,
            difficulty,
        };
        block.run_proof_of_work()?;
        Ok(block)
    }

    /// NewGenesisBlock creates and returns genesis Block
    pub fn new_genesis_block(coinbase: Transaction) -> Block {
        Block::new_block(vec![coinbase], String::new(), 0, TARGET_HEXS).unwrap()
    }

    /// Run performs a proof-of-work
    fn run_proof_of_work(&mut self) -> Result<()> {
        info!("Mining the block");
        while !self.validate()? {
            self.nonce += 1;
        }
        let data = self.prepare_hash_data()?;
        let mut hasher = Sha256::new();
        hasher.update(&data[..]);
        self.hash = format!("{:x}", hasher.finalize());
        Ok(())
    }

    /// HashTransactions returns a hash of the transactions in the block
    pub fn hash_transactions_for_spv(&self) -> Result<Vec<u8>> {
        self.hash_transactions()
    }

    /// HashTransactions returns a hash of the transactions in the block
    fn hash_transactions(&self) -> Result<Vec<u8>> {
        let mut hashes = Vec::new();
        for tx in &self.transactions {
            hashes.push(tx.hash()?.as_bytes().to_owned());
        }
        let tree = CBMT::<Vec<u8>, MergeVu8>::build_merkle_tree(&hashes);
        Ok(tree.root())
    }

    /// GetTransactionProof generates a Merkle Proof for a specific transaction
    pub fn get_transaction_proof(&self, tx_id: &str) -> Result<(Vec<u32>, Vec<Vec<u8>>)> {
        let mut hashes = Vec::new();
        let mut index = None;
        for (i, tx) in self.transactions.iter().enumerate() {
            if tx.id == tx_id {
                index = Some(i as u32);
            }
            hashes.push(tx.hash()?.as_bytes().to_owned());
        }

        if let Some(i) = index {
            let tree = CBMT::<Vec<u8>, MergeVu8>::build_merkle_tree(&hashes);
            let proof = tree.build_proof(&[i]).unwrap();
            Ok((proof.indices().to_vec(), proof.lemmas().to_vec()))
        } else {
            Err(anyhow!("Transaction not found in this block"))
        }
    }

    /// VerifyProof verifies a Merkle Proof against a Merkle Root
    pub fn verify_proof(root: &Vec<u8>, tx_hash: &[u8], indices: Vec<u32>, lemmas: Vec<Vec<u8>>) -> bool {
        let proof = merkle_cbt::merkle_tree::MerkleProof::<Vec<u8>, MergeVu8>::new(indices, lemmas);
        proof.verify(root, &vec![tx_hash.to_vec()])
    }

    fn prepare_hash_data(&self) -> Result<Vec<u8>> {
        let content = (
            self.prev_block_hash.clone(),
            self.hash_transactions()?,
            self.timestamp,
            self.difficulty,
            self.nonce,
        );
        let bytes = bincode::serialize(&content)?;
        Ok(bytes)
    }

    /// Validate validates block's PoW
    fn validate(&self) -> Result<bool> {
        let data = self.prepare_hash_data()?;
        let mut hasher = Sha256::new();
        hasher.update(&data[..]);
        let result = format!("{:x}", hasher.finalize());
        let mut vec1: Vec<u8> = Vec::new();
        vec1.resize(self.difficulty, '0' as u8);
        Ok(&result[0..self.difficulty] == String::from_utf8(vec1)?)
    }
}

struct MergeVu8 {}

impl Merge for MergeVu8 {
    type Item = Vec<u8>;
    fn merge(left: &Self::Item, right: &Self::Item) -> Self::Item {
        let mut hasher = Sha256::new();
        let mut data: Vec<u8> = left.clone();
        data.append(&mut right.clone());
        hasher.update(&data);
        hasher.finalize().to_vec()
    }
}
