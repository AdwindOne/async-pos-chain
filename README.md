# Async PoS Blockchain

A simple async proof-of-stake blockchain written in Rust.

## Features

- ✅ PoS proposer selection
- ✅ Mempool
- ✅ SQLite storage
- ✅ P2P network (via TCP + JSON)
- ✅ Account balance persistence
- ✅ Block/transaction/peer query
- ✅ JSON-RPC interface

## Usage Examples

### Start Node and JSON-RPC Server

```sh
cargo run -- run --port 8000
```
- Starts both the PoS node (port 8000) and the JSON-RPC server (port 8545).
- You can submit transactions via CLI or JSON-RPC at the same time.

### Submit a Transaction (CLI)

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

### Add a Peer Node

```sh
cargo run -- add-peer --addr 127.0.0.1:8001
```
- All added peers are saved in `peers.db` and loaded automatically on node startup.

### Query Peer Nodes

```sh
cargo run -- query-peers
```
- Lists all known peer nodes from `peers.db`.

### Query Transaction by Hash

```sh
cargo run -- query-tx --hash 0x1234abcd...
```
- Returns transaction details and the block height it was included in.

### JSON-RPC: Send Transaction

Start the JSON-RPC server (already started with `run`):

```sh
cargo run -- run --port 8000
```

Send a transaction via curl:

```sh
curl -X POST http://127.0.0.1:8545 -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","method":"send_transaction","params":["Alice","Bob",123],"id":1}'
```
- The server will print the transaction and return a JSON-RPC response with a `tx_hash` (with `0x` prefix).

### Multi-Node Demo

1. Start several nodes on different ports:
   ```sh
   cargo run -- run --port 8000
   cargo run -- run --port 8001
   ```
2. Add peers to each node:
   ```sh
   cargo run -- add-peer --addr 127.0.0.1:8001
   # On the other node:
   cargo run -- add-peer --addr 127.0.0.1:8000
   ```
3. Submit transactions to any node and query blocks, balances, or transactions on any node.

### Account Initialization
- On first run, the following accounts are initialized:
  - `admin`: 1,000,000 tokens (also receives block rewards)
  - `Alice`: 100 tokens
  - `Bob`: 100 tokens
- You can add more accounts by modifying the code in `main.rs`.

### Block Rewards
- Each time a block is produced, a reward (default: 50 tokens) is given to the block proposer.
- You can change the reward amount in the code.

### Clean Database (for development)
If you change the database schema or want a fresh start, delete the database files:
```sh
rm chain.db peers.db
```

### Common Issues
- **Error: `table blocks has no column named transactions`**
  - Solution: Delete `chain.db` and restart the node to recreate the table with the new schema.
- **Balance not updated after transaction**
  - Make sure the node is running and the transaction is included in a block (check with `query`).

---

## API Summary

- **CLI Commands:**
  - `run` — Start node and JSON-RPC server
  - `submit` — Submit a transaction
  - `query` — Query block by height
  - `query-balance` — Query account balance
  - `add-peer` — Add a peer node
  - `query-peers` — List all peer nodes
  - `query-tx` — Query transaction by hash
- **JSON-RPC:**
  - `send_transaction` — Send a transaction (returns tx_hash)

---

## License
MIT
