//! cli process

use super::*;
use crate::block::Block;
use crate::blockchain::*;
use crate::server::*;
use crate::transaction::*;
use crate::utxoset::*;
use crate::wallets::*;
use bitcoincash_addr::Address;
use clap::{Arg, Command, ArgAction};

pub struct Cli {}

impl Cli {
    pub fn new() -> Cli {
        Cli {}
    }

    pub fn run(&mut self) -> Result<()> {
        info!("run app");
        let matches = Command::new("blockchain-rs")
            .version("0.1")
            .author("Jay")
            .about("reimplement blockchain_go in rust: a simple blockchain for learning")
            .subcommand(Command::new("printchain").about("print all the chain blocks"))
            .subcommand(Command::new("createwallet").about("create a wallet"))
            .subcommand(Command::new("listaddresses").about("list all addresses"))
            .subcommand(Command::new("reindex").about("reindex UTXO"))
            .subcommand(
                Command::new("verifytx")
                    .about("Verify a transaction using Merkle Proof (SPV style)")
                    .arg(Arg::new("txid").help("Transaction ID").required(true)),
            )
            .subcommand(
                Command::new("startnode")
                    .about("start the node server")
                    .arg(Arg::new("port").help("the port server bind to locally").required(true)),
            )
            .subcommand(
                Command::new("startminer")
                    .about("start the minner server")
                    .arg(Arg::new("port").help("the port server bind to locally").required(true))
                    .arg(Arg::new("address").help("wallet address").required(true)),
            )
            .subcommand(
                Command::new("getbalance")
                    .about("get balance in the blockchain")
                    .arg(Arg::new("address").help("The address to get balance for").required(true)),
            )
            .subcommand(Command::new("createblockchain").about("create blockchain").arg(
                Arg::new("address").help("The address to send genesis block reward to").required(true),
            ))
            .subcommand(
                Command::new("send")
                    .about("send in the blockchain")
                    .arg(Arg::new("from").help("Source wallet address").required(true))
                    .arg(Arg::new("to").help("Destination wallet address").required(true))
                    .arg(Arg::new("amount").help("Amount to send").required(true))
                    .arg(Arg::new("mine")
                        .short('m')
                        .long("mine")
                        .help("the from address mine immediately")
                        .action(ArgAction::SetTrue)),
            )
            .get_matches();

        match matches.subcommand() {
            Some(("getbalance", ref matches)) => {
                let address = matches.get_one::<String>("address").unwrap();
                let balance = cmd_get_balance(address)?;
                println!("Balance: {}\n", balance);
            }
            Some(("createwallet", _)) => {
                println!("address: {}", cmd_create_wallet()?);
            }
            Some(("printchain", _)) => {
                cmd_print_chain()?;
            }
            Some(("reindex", _)) => {
                let count = cmd_reindex()?;
                println!("Done! There are {} transactions in the UTXO set.", count);
            }
            Some(("verifytx", ref matches)) => {
                let txid = matches.get_one::<String>("txid").unwrap();
                cmd_verify_tx(txid)?;
            }
            Some(("listaddresses", _)) => {
                cmd_list_address()?;
            }
            Some(("createblockchain", ref matches)) => {
                let address = matches.get_one::<String>("address").unwrap();
                cmd_create_blockchain(address)?;
            }
            Some(("send", ref matches)) => {
                let from = matches.get_one::<String>("from").unwrap();
                let to = matches.get_one::<String>("to").unwrap();
                let amount: i32 = matches.get_one::<String>("amount").unwrap().parse()?;
                let mine = matches.get_flag("mine");
                cmd_send(from, to, amount, mine)?;
            }
            Some(("startnode", ref matches)) => {
                let port = matches.get_one::<String>("port").unwrap();
                println!("Start node...");
                let bc = Blockchain::new()?;
                let utxo_set = UTXOSet { blockchain: bc };
                let server = Server::new(port, "", utxo_set)?;
                server.start_server()?;
            }
            Some(("startminer", ref matches)) => {
                let address = matches.get_one::<String>("address").unwrap();
                let port = matches.get_one::<String>("port").unwrap();
                println!("Start miner node...");
                let bc = Blockchain::new()?;
                let utxo_set = UTXOSet { blockchain: bc };
                let server = Server::new(port, address, utxo_set)?;
                server.start_server()?;
            }
            _ => {
                println!("No subcommand was used");
            }
        }

        Ok(())
    }
}

fn cmd_send(from: &str, to: &str, amount: i32, mine_now: bool) -> Result<()> {
    let bc = Blockchain::new()?;
    let mut utxo_set = UTXOSet { blockchain: bc };
    let wallets = Wallets::new()?;
    let wallet = wallets.get_wallet(from).unwrap();
    let tx = Transaction::new_UTXO(wallet, to, amount, &utxo_set)?;
    if mine_now {
        let height = utxo_set.blockchain.get_best_height()? + 1;
        let cbtx = Transaction::new_coinbase(from.to_string(), String::from("reward!"), height, 0)?;
        let new_block = utxo_set.blockchain.mine_block(vec![cbtx, tx])?;

        utxo_set.update(&new_block)?;
    } else {
        Server::send_transaction(&tx, utxo_set)?;
    }

    println!("success!");
    Ok(())
}

fn cmd_create_wallet() -> Result<String> {
    let mut ws = Wallets::new()?;
    let address = ws.create_wallet();
    ws.save_all()?;
    Ok(address)
}

fn cmd_reindex() -> Result<i32> {
    let bc = Blockchain::new()?;
    let utxo_set = UTXOSet { blockchain: bc };
    utxo_set.reindex()?;
    utxo_set.count_transactions()
}

fn cmd_create_blockchain(address: &str) -> Result<()> {
    let address = String::from(address);
    let bc = Blockchain::create_blockchain(address)?;

    let utxo_set = UTXOSet { blockchain: bc };
    utxo_set.reindex()?;
    println!("create blockchain");
    Ok(())
}

fn cmd_get_balance(address: &str) -> Result<i32> {
    let pub_key_hash = Address::decode(address).unwrap().body;
    let bc = Blockchain::new()?;
    let utxo_set = UTXOSet { blockchain: bc };
    let utxos = utxo_set.find_UTXO(&pub_key_hash)?;

    let mut balance = 0;
    for out in utxos.outputs {
        balance += out.value;
    }
    Ok(balance)
}

fn cmd_print_chain() -> Result<()> {
    let bc = Blockchain::new()?;
    for b in bc.iter() {
        println!("{:#?}", b);
    }
    Ok(())
}

fn cmd_list_address() -> Result<()> {
    let ws = Wallets::new()?;
    let addresses = ws.get_all_addresses();
    println!("addresses: ");
    for ad in addresses {
        println!("{}", ad);
    }
    Ok(())
}

fn cmd_verify_tx(txid: &str) -> Result<()> {
    let bc = Blockchain::new()?;
    for block in bc.iter() {
        if let Ok((indices, lemmas)) = block.get_transaction_proof(txid) {
            println!("Transaction found in block: {}", block.get_hash());
            
            // 模拟轻节点持有的数据：区块头中的 Merkle Root
            // 注意：在实际系统中，区块头不存储 Merkle Root 字段，而是通过 hash_transactions() 计算出来的。
            // 为了演示，我们在这里获取它。
            let mut tx_hash = Vec::new();
            for tx in block.get_transaction() {
                if tx.id == txid {
                    tx_hash = tx.hash()?.as_bytes().to_owned();
                    break;
                }
            }

            // 获取 Merkle Root (通常在区块头中)
            // 我们需要临时访问 Block 里的私有方法或重新计算
            // 这里为了简单，我们通过 block 重新获取 root (这在实际 SPV 中是由全节点提供的)
            let root = block.hash_transactions_for_spv()?; 

            if Block::verify_proof(&root, &tx_hash, indices, lemmas) {
                println!("Verification SUCCESS: Transaction is valid and part of the blockchain!");
            } else {
                println!("Verification FAILED: Proof is invalid.");
            }
            return Ok(());
        }
    }
    println!("Transaction {} not found in any block.", txid);
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_locally() {
        let addr1 = cmd_create_wallet().unwrap();
        let addr2 = cmd_create_wallet().unwrap();
        cmd_create_blockchain(&addr1).unwrap();

        let b1 = cmd_get_balance(&addr1).unwrap();
        let b2 = cmd_get_balance(&addr2).unwrap();
        assert_eq!(b1, 50);
        assert_eq!(b2, 0);

        cmd_send(&addr1, &addr2, 5, true).unwrap();

        let b1 = cmd_get_balance(&addr1).unwrap();
        let b2 = cmd_get_balance(&addr2).unwrap();
        assert_eq!(b1, 95); // 50 (new block) + 50 (old) - 5 (sent) = 95
        assert_eq!(b2, 5);

        cmd_send(&addr2, &addr1, 15, true).unwrap_err();
        let b1 = cmd_get_balance(&addr1).unwrap();
        let b2 = cmd_get_balance(&addr2).unwrap();
        assert_eq!(b1, 95);
        assert_eq!(b2, 5);
    }
}
