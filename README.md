# Blockchain-RS: 深入浅出的 Rust 区块链实现

本项目是一个使用 Rust 编写的简化版区块链，旨在通过代码实现理解比特币的核心原理。它涵盖了区块结构、工作量证明（PoW）、UTXO 模型、交易签名、钱包管理、UTXO 索引优化以及基于 TCP 的网络传播机制。

## 1. 核心架构逻辑分析

本项目采用模块化设计，每个模块负责区块链的一个核心功能层。

### 1.1 区块模块 (`src/block.rs`)
区块是区块链的基础单位。
*   **结构定义**: `Block` 包含时间戳、交易列表、父区块哈希、当前区块哈希、随机数（Nonce）和高度。
*   **工作量证明 (PoW)**:
    *   通过 `run_proof_of_work` 方法实现。
    *   目标是找到一个 Nonce，使得区块哈希的前 N 位为零（本项目中 `TARGET_HEXS = 4`）。
    *   哈希对象包含：父哈希、交易的 Merkle 根、时间戳、难度目标和 Nonce。
*   **Merkle 树**: 使用 `merkle-cbt` 构建交易的 Merkle 树。区块头只存储根哈希，保证了交易的完整性验证。

### 1.2 交易模块 (`src/transaction.rs`)
本项目完整实现了 **UTXO (Unspent Transaction Output)** 模型，这是比特币的核心机制。
*   **TXInput & TXOutput**:
    *   `TXInput`: 引用之前的某个输出（TxID + Index），并包含签名和原始公钥。
    *   `TXOutput`: 包含金额和公钥哈希（用于锁定资金）。
*   **交易哈希**: 交易的唯一标识，是对交易结构（排除签名信息）进行序列化后的 SHA256 哈希。
*   **签名与验证**:
    *   **签名**: 发送者使用私钥对交易副本进行签名。签名过程中，输入中的 `pub_key` 会被临时替换为引用的输出的 `pub_key_hash`。
    *   **验证**: 节点使用发送者的公钥验证签名的合法性，确保资金只能被所有者使用。
*   **Coinbase 交易**: 每产生一个新块时的奖励交易，没有输入，只有输出。

### 1.3 UTXO 集合优化 (`src/utxoset.rs`)
为了避免遍历整个区块链来检查余额，本项目维护了一个 `UTXOSet` 索引层（存储在 `data/utxos` 数据库中）。
*   **功能**:
    *   `reindex`: 扫描整个区块链，提取所有未使用的输出并存入数据库。
    *   `update`: 每当新区块产生时，自动更新索引：移除被新交易引用的输入，添加新产生的输出。
    *   `find_spendable_outputs`: 快速查询某个地址可用的 UTXO 列表，用于构建新交易。

### 1.4 区块链管理 (`src/blockchain.rs`)
负责维护主链逻辑和持久化存储。
*   **存储引擎**: 使用高性能键值对数据库 `sled`。
    *   `data/blocks`: 存储区块哈希到区块二进制数据的映射。
    *   `LAST`: 存储当前主链尖端的哈希。
*   **链遍历**: 实现了一个迭代器 `BlockchainIterator`，从 `tip` 开始通过 `prev_block_hash` 向后回溯至创世区块。

### 1.5 钱包与地址 (`src/wallets.rs`)
*   **加密算法**: 使用 **Ed25519** 曲线生成密钥对。
*   **地址生成逻辑**:
    1.  `PublicKey` -> `SHA256` -> `RIPEMD160` -> `PubKeyHash` (20字节)。
    2.  `PubKeyHash` -> `Base58` 编码（使用 `bitcoincash-addr` 库）-> 最终钱包地址。
*   **持久化**: 钱包数据存储在 `data/wallets` 中。

### 1.6 网络与节点通信 (`src/server.rs`)
实现了一个简化的 P2P 网络协议。
*   **节点角色**:
    *   普通节点：同步区块，转发交易。
    *   矿工节点：监听交易池（Mempool），打包交易并进行 PoW 挖掘。
*   **消息协议**:
    *   `Version`: 握手消息，比较节点间的区块高度，触发同步。
    *   `GetBlocks/Inv`: 索取/宣告区块列表。
    *   `GetData`: 获取特定的区块或交易。
    *   `Tx/Block`: 广播新创建的交易或挖掘出的区块。

## 2. 核心数据流

### 2.1 转账流程
1.  **客户端指令**: 用户输入 `send <from> <to> <amount>`。
2.  **创建交易**: 
    *   查询 `UTXOSet` 找到属于 `<from>` 且总额足够的输出。
    *   构造 `TXInput` 引用这些输出。
    *   构造 `TXOutput` 发给 `<to>`，如有找零则构造找零输出发回 `<from>`。
3.  **签名**: 使用私钥对交易进行签名。
4.  **广播**: 将交易发送至网络。矿工收到后存入 `Mempool`。
5.  **打包**: 矿工从 `Mempool` 提取交易，验证合法性，加入新块。
6.  **挖矿**: 矿工进行 PoW 计算。
7.  **确认**: 挖掘成功后广播新块，全网节点更新 `Blockchain` 和 `UTXOSet`。

## 3. 技术栈说明
*   **Language**: Rust (Edition 2024)
*   **Storage**: [Sled](https://github.com/spacejam/sled) (嵌入式数据库)
*   **Serialization**: [Bincode](https://github.com/bincode-org/bincode) & [Serde](https://serde.rs/)
*   **Cryptography**:
    *   `sha2`: SHA256 哈希。
    *   `ripemd`: RIPEMD160 哈希。
    *   `ed25519-dalek`: 高性能数字签名。
*   **CLI**: [Clap 4.5](https://github.com/clap-rs/clap)
*   **Error Handling**: `anyhow` & `thiserror`

## 4. 运行指南
由于 `sled` 数据库的文件锁限制，运行测试时建议限制线程数：
```bash
cargo test -- --test-threads=1
```

使用 CLI：
```bash
# 创建钱包
cargo run -- createwallet
# 创建区块链（奖励给指定地址）
cargo run -- createblockchain <address>
# 打印账本
cargo run -- printchain
# 转账
cargo run -- send <from> <to> <amount> -m
```
