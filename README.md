# Async PoS Blockchain

A simple async proof-of-stake blockchain written in Rust.

## Features

- ✅ PoS proposer selection
- ✅ Mempool
- ✅ SQLite storage
- ✅ P2P network (via TCP + JSON)

## Usage Examples

### Start a Node

```sh
cargo run -- run --port 8000
```

You can start multiple nodes on different ports (e.g. 8001, 8002) in separate terminals for a multi-node demo.

### Submit a Transaction

```sh
cargo run -- submit --from Alice --to Bob --amount 200
```

### Query a Block by Height

```sh
cargo run -- query --index 2
```

### Query Account Balance

```sh
cargo run -- query-balance --address Alice
```

### Multi-Node Demo

1. Start several nodes on different ports:
   ```sh
   cargo run -- run --port 8000
   cargo run -- run --port 8001
   ```
2. Submit transactions to any node:
   ```sh
   cargo run -- submit --from Alice --to Bob --amount 50
   cargo run -- submit --from Bob --to Charlie --amount 30
   ```
3. Query blocks and balances on any node:
   ```sh
   cargo run -- query --index 1
   cargo run -- query-balance --address Bob
   ```

### Account Initialization
- On first run, the following accounts are initialized:
  - `admin`: 1,000,000 tokens (also receives block rewards)
  - `Alice`: 100 tokens
  - `Bob`: 100 tokens
- You can add more accounts by modifying the code in `main.rs`.

### Block Rewards
- Each time a block is produced, a reward (default: 50 tokens) is given to the `admin` account.
- You can change the reward amount or recipient in the code.

### Clean Database (for development)
If you change the database schema or want a fresh start, delete the database file:
```sh
rm chain.db
```

### Common Issues
- **Error: `table blocks has no column named transactions`**
  - Solution: Delete `chain.db` and restart the node to recreate the table with the new schema.
- **Balance not updated after transaction**
  - Make sure the node is running and the transaction is included in a block (check with `query`).

### Add a Peer Node

```sh
cargo run -- add-peer --addr 127.0.0.1:8001
```

- All added peers are saved in `peers.db` and will be loaded automatically on node startup.
- You can add as many peer addresses as you want.

### Query Peer Nodes

```sh
cargo run -- query-peers
```

- Lists all known peer nodes from `peers.db`.

### Multi-Node Voting Participation
- Each node can be added via `add-peer` and listed via `query-peers`.
- All nodes in the peer list can participate in block proposal and voting (PoS selection).
- To simulate multi-node voting, run several nodes, add each other's addresses, and observe proposer rotation.

---

## 中文命令示例

### 启动节点
```sh
cargo run -- run --port 8000
```

### 提交交易
```sh
cargo run -- submit --from Alice --to Bob --amount 200
```

### 查询区块
```sh
cargo run -- query --index 2
```

### 查询余额
```sh
cargo run -- query-balance --address Alice
```

### 多节点/多账户
- 可在不同终端用不同端口运行多个节点
- 通过 submit 命令向任意节点提交交易
- 通过 query/query-balance 命令随时查询区块和余额

### 查看/添加节点（中文）

```sh
cargo run -- add-peer --addr 127.0.0.1:8001
```

- 所有添加的节点会保存在 `peers.db`，节点启动时自动加载。
- 可多次添加不同节点地址。

### 查询节点信息（中文）

```sh
cargo run -- query-peers
```

- 显示所有已知节点。

### 多节点投票说明
- 每个节点都可通过 `add-peer` 添加到网络，并用 `query-peers` 查询。
- 所有节点都可参与出块提议和投票（PoS 选举）。
- 多节点环境下，提议者会在节点间轮换。

### JSON-RPC: Send Transaction

Start the JSON-RPC server:

```sh
cargo run -- json-rpc-server --port 8545
```

Send a transaction via curl:

```sh
curl -X POST http://127.0.0.1:8545 -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","method":"send_transaction","params":["Alice","Bob",123],"id":1}'
```

- The server will print the transaction and return a JSON-RPC response.
- You can integrate with web frontends or scripts using this interface.

---

### JSON-RPC 发送交易（中文）

启动 JSON-RPC 服务：

```sh
cargo run -- json-rpc-server --port 8545
```

用 curl 发送交易：

```sh
curl -X POST http://127.0.0.1:8545 -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","method":"send_transaction","params":["Alice","Bob",123],"id":1}'
```

- 服务端会打印交易内容并返回 JSON-RPC 响应。
- 可用于网页前端或脚本集成。

### Start Node and JSON-RPC Server Together

```sh
cargo run -- run --port 8000
```

- This will start both the PoS node (on port 8000) and the JSON-RPC server (on port 8545) in the same process.
- You can submit transactions via CLI or JSON-RPC at the same time.

---

### 同时启动节点和 JSON-RPC 服务（中文）

```sh
cargo run -- run --port 8000
```

- 该命令会同时启动 PoS 节点（8000端口）和 JSON-RPC 服务（8545端口）。
- 可同时通过命令行或 JSON-RPC 提交交易。

### Query Transaction by Hash

```sh
cargo run -- query-tx --hash 0x1234abcd...
```

- Returns transaction details and the block height it was included in.

---

### 根据交易哈希查询交易（中文）

```sh
cargo run -- query-tx --hash 0x1234abcd...
```

- 返回该交易的详细信息和所在区块高度。
