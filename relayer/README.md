# Bridge Relayer

The relayer is the core component that monitors both Solana and Ethereum blockchains for bridge events, collects validator signatures, and submits transactions to complete cross-chain transfers.

## Architecture

The relayer consists of several key components:

1. **Solana Monitor** - Watches for `TokensLocked` events on the Solana bridge program
2. **Ethereum Monitor** - Watches for `TokensBurned` events on the Ethereum bridge contract
3. **Validator Client** - Collects signatures from validator nodes
4. **Transaction Submitter** - Submits transactions to destination chains with collected signatures
5. **Database** - Tracks transaction states and manages the relayer workflow

## How It Works

### Solana → Ethereum Flow

1. User locks SOL on Solana by calling the `lock_tokens` instruction
2. Solana Monitor detects the `TokensLocked` event and creates a database entry
3. Transaction Submitter requests signatures from validators
4. Validators verify the Solana transaction and sign the mint message
5. Once enough signatures are collected, submitter calls `mintWrapped` on Ethereum
6. wSOL tokens are minted to the user's Ethereum address

### Ethereum → Solana Flow

1. User burns wSOL on Ethereum by calling `burnAndBridge`
2. Ethereum Monitor detects the `TokensBurned` event and creates a database entry
3. Transaction Submitter requests signatures from validators
4. Validators verify the Ethereum transaction and sign the unlock message
5. Once enough signatures are collected, submitter calls `unlock_tokens` on Solana
6. SOL is unlocked from the vault to the user's Solana address

## Configuration

Create a `.env` file in the relayer directory with the following variables:

```env
# Solana Configuration
SOLANA_RPC_URL=https://api.devnet.solana.com
SOLANA_WS_URL=wss://api.devnet.solana.com
SOLANA_BRIDGE_PROGRAM_ID=7DazfS5hDxNJMJcxs1uKk3yoob7cbPLBFMXA3iRotjRH
SOLANA_COMMITMENT=confirmed

# Ethereum Configuration
ETHEREUM_RPC_URL=https://sepolia.infura.io/v3/YOUR_INFURA_KEY
ETHEREUM_WS_URL=wss://sepolia.infura.io/ws/v3/YOUR_INFURA_KEY
ETHEREUM_CHAIN_ID=11155111
ETHEREUM_BRIDGE_CONTRACT=0xaBD6f99Fbb77051B28942abe3118bf4D8Ea9F2CA
ETHEREUM_WRAPPED_SOL_CONTRACT=0xF718C74C9b298bCDd48Ed8801325E6ddBE2a5A5c
ETHEREUM_VALIDATOR_REGISTRY_CONTRACT=0xE45DC6606979b9086375561Ff7d8f66f8C506816
ETHEREUM_CONFIRMATIONS=12

# Relayer Configuration
POLL_INTERVAL_MS=5000
MAX_RETRIES=3
RETRY_DELAY_MS=2000
GAS_PRICE_MULTIPLIER=1.2

# Database
DATABASE_URL=sqlite://relayer.db
DB_MAX_CONNECTIONS=10

# Validator Configuration (configure your validator network)
VALIDATOR1_ETH_ADDRESS=0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0
VALIDATOR1_SOL_PUBKEY=5XqZ...
VALIDATOR1_ENDPOINT=http://localhost:8080

# Add more validators as needed
# VALIDATOR2_ETH_ADDRESS=...
# VALIDATOR3_ETH_ADDRESS=...

# Logging
RUST_LOG=relayer=info,solana_client=warn
```

## Running the Relayer

### Prerequisites

1. Rust 1.70 or higher
2. Access to Solana RPC node (devnet/mainnet)
3. Access to Ethereum RPC node (Sepolia/mainnet)
4. Validator nodes running and accessible

### Build

```bash
cargo build --release
```

### Run

```bash
cargo run --release
```

Or using the binary directly:

```bash
./target/release/relayer
```

## Database Schema

The relayer uses SQLite to track transactions:

```sql
CREATE TABLE relayer_transactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    nonce INTEGER NOT NULL UNIQUE,
    from_chain TEXT NOT NULL,
    to_chain TEXT NOT NULL,
    from_tx_hash TEXT NOT NULL UNIQUE,
    to_tx_hash TEXT,
    sender TEXT NOT NULL,
    recipient TEXT NOT NULL,
    amount INTEGER NOT NULL,
    status TEXT NOT NULL,
    signatures TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

### Transaction States

- **Pending** - Event detected, waiting to collect signatures
- **SignaturesCollected** - Sufficient signatures obtained, ready to submit
- **Submitted** - Transaction submitted to destination chain
- **Confirmed** - Transaction confirmed on destination chain
- **Failed** - Transaction failed (will retry up to MAX_RETRIES)

## Validator Integration

The relayer communicates with validator nodes to collect signatures. Each validator:

1. Receives a signature request with transaction details
2. Independently verifies the source chain transaction
3. Signs a message hash if verification succeeds
4. Returns the signature to the relayer

### Validator Endpoints

Validators should expose an HTTP API with endpoints:

- `POST /sign-ethereum` - Sign a message for Ethereum verification
- `POST /sign-solana` - Sign a message for Solana verification
- `GET /health` - Health check

## Monitoring

The relayer logs detailed information about:

- Events detected on each chain
- Signature collection progress
- Transaction submission status
- Confirmation status
- Errors and retries

### Statistics

On startup and during operation, the relayer displays transaction statistics:

```
Transaction Statistics:
  Total: 42
  Pending: 3
  Signatures Collected: 2
  Submitted: 5
  Confirmed: 30
  Failed: 2
```

## Development

### Testing

```bash
# Run unit tests
cargo test

# Run with detailed logging
RUST_LOG=debug cargo run
```

### Code Structure

```
relayer/
├── src/
│   ├── main.rs                   # Main entry point, orchestrates all components
│   ├── config.rs                 # Configuration management
│   ├── db.rs                     # Database operations
│   ├── error.rs                  # Error types
│   ├── types.rs                  # Core types (Chain, BridgeEvent, etc.)
│   ├── solana_monitor.rs         # Solana event monitoring
│   ├── ethereum_monitor.rs       # Ethereum event monitoring
│   ├── validator_client.rs       # Validator signature collection
│   └── transaction_submitter.rs  # Transaction submission logic
├── Cargo.toml
└── README.md
```

## Security Considerations

1. **Private Keys**: Never commit private keys to version control. Use environment variables or secure key management systems.

2. **RPC Endpoints**: Use trusted RPC providers. Consider running your own nodes for production.

3. **Validator Consensus**: Ensure sufficient validator threshold (e.g., 2-of-3 minimum for production).

4. **Replay Protection**: Nonces prevent replay attacks across chains.

5. **Confirmation Depths**: Wait for sufficient confirmations before processing events.

## Troubleshooting

### Relayer not detecting events

- Check RPC URLs are correct and accessible
- Verify bridge contract/program addresses are correct
- Check commitment level settings
- Review logs for RPC errors

### Signatures not collecting

- Verify validator endpoints are accessible
- Check validator configuration
- Ensure validators are running and healthy
- Review validator logs for errors

### Transactions not submitting

- Verify relayer has sufficient ETH/SOL for gas
- Check private keys are properly configured
- Review gas price settings
- Check for RPC rate limits

### Database errors

- Ensure database file is writable
- Check disk space
- Verify DATABASE_URL is correct
- Review SQLite logs

## Production Deployment

For production deployment:

1. **Use WebSocket subscriptions** instead of polling for better performance
2. **Run multiple relayer instances** for redundancy
3. **Implement monitoring and alerting** (Prometheus, Grafana)
4. **Use secure key management** (AWS KMS, HashiCorp Vault)
5. **Set up proper logging** (structured logs, log aggregation)
6. **Configure rate limiting** to avoid RPC throttling
7. **Implement health checks** and automatic restarts
8. **Use managed database** (PostgreSQL instead of SQLite)

## License

See main project LICENSE file.
