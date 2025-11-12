# Bridge Vault Setup Guide

This guide will walk you through setting up the Bridge Vault development environment and deploying all components locally or to testnets.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Initial Setup](#initial-setup)
3. [Solana Program Setup](#solana-program-setup)
4. [Ethereum Contracts Setup](#ethereum-contracts-setup)
5. [Relayer Service Setup](#relayer-service-setup)
6. [Web Application Setup](#web-application-setup)
7. [Testing the Bridge](#testing-the-bridge)
8. [Troubleshooting](#troubleshooting)

## Prerequisites

### Required Software

Before you begin, ensure you have the following installed:

#### 1. Rust and Cargo
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify installation
rustc --version  # Should be 1.70 or higher
cargo --version
```

#### 2. Solana CLI Tools
```bash
# Install Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"

# Add to PATH (add to your shell profile)
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"

# Verify installation
solana --version  # Should be 1.17 or higher
```

#### 3. Node.js and npm
```bash
# Install Node.js 18 or higher
# Using nvm (recommended)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install 18
nvm use 18

# Verify installation
node --version  # Should be v18 or higher
npm --version
```

#### 4. Git
```bash
# Verify Git is installed
git --version
```

### Required Accounts and Keys

1. **Solana Wallet**: You'll need a Solana wallet with some SOL for devnet testing
2. **Ethereum Wallet**: MetaMask or similar for Sepolia testnet
3. **Infura Account**: Sign up at [infura.io](https://infura.io) for Ethereum RPC access (free tier works)
4. **Test Tokens**:
   - Solana devnet SOL: Use `solana airdrop`
   - Sepolia testnet ETH: Get from [Sepolia faucet](https://sepoliafaucet.com/)

## Initial Setup

### 1. Clone the Repository

```bash
git clone https://github.com/RaghavenderSingh/bridge-vault.git
cd bridge-vault
```

### 2. Install Root Dependencies

```bash
npm install
```

## Solana Program Setup

### 1. Configure Solana CLI for Devnet

```bash
# Set to devnet
solana config set --url https://api.devnet.solana.com

# Create a new keypair (or use existing)
solana-keygen new --outfile ~/.config/solana/id.json

# Verify your configuration
solana config get

# Get some devnet SOL
solana airdrop 2
```

### 2. Build the Solana Program

```bash
cd programs/bridge-vault

# Install dependencies and build
cargo build-sbf

# Expected output: Compiled program in target/deploy/
```

### 3. Deploy the Program

```bash
# Deploy to devnet
solana program deploy target/deploy/bridge_vault.so

# Save the Program ID that's returned - you'll need this later
# Example output: Program Id: 7xXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
```

### 4. Run Solana Tests

```bash
# Run unit tests
cargo test-sbf

# Expected: All tests should pass
```

## Ethereum Contracts Setup

### 1. Navigate to Contracts Directory

```bash
cd contracts  # From project root
```

### 2. Install Dependencies

```bash
npm install
```

### 3. Configure Environment Variables

```bash
# Copy the example file
cp .env.example .env

# Edit .env with your details
nano .env  # or use your preferred editor
```

Fill in the following in your `.env` file:

```env
# Get from infura.io after signing up
SEPOLIA_RPC_URL=https://sepolia.infura.io/v3/YOUR_INFURA_PROJECT_ID

# Your private key (NEVER commit this or use mainnet keys)
# Export from MetaMask: Settings -> Security -> Export Private Key
PRIVATE_KEY=your_private_key_without_0x_prefix

# Optional: for contract verification
ETHERSCAN_API_KEY=your_etherscan_api_key
```

### 4. Compile Contracts

```bash
npx hardhat compile

# Expected output: Compiled contracts successfully
```

### 5. Deploy to Sepolia Testnet

```bash
# Make sure you have Sepolia ETH in your wallet
npx hardhat run scripts/deploy.ts --network sepolia

# Save all contract addresses from the output:
# - WrappedSOL contract address
# - ValidatorRegistry contract address
# - SolanaBridge contract address
```

### 6. Run Ethereum Tests

```bash
# Run all contract tests
npx hardhat test

# Expected: All tests should pass
```

### 7. Verify Contracts on Etherscan (Optional)

```bash
npx hardhat verify --network sepolia DEPLOYED_CONTRACT_ADDRESS
```

## Relayer Service Setup

The relayer monitors both chains and facilitates cross-chain transfers.

### 1. Navigate to Relayer Directory

```bash
cd relayer  # From project root
```

### 2. Configure Environment

```bash
# Copy example configuration
cp .env.example .env

# Edit with your settings
nano .env
```

Fill in your `.env` file with the deployed contract addresses:

```env
# Solana Configuration
SOLANA_RPC_URL=https://api.devnet.solana.com
SOLANA_WS_URL=wss://api.devnet.solana.com
SOLANA_BRIDGE_PROGRAM_ID=<YOUR_DEPLOYED_PROGRAM_ID>
SOLANA_COMMITMENT=confirmed

# Ethereum Configuration
ETHEREUM_RPC_URL=https://sepolia.infura.io/v3/YOUR_INFURA_KEY
ETHEREUM_WS_URL=wss://sepolia.infura.io/ws/v3/YOUR_INFURA_KEY
ETHEREUM_CHAIN_ID=11155111
ETHEREUM_BRIDGE_CONTRACT=<YOUR_DEPLOYED_BRIDGE_ADDRESS>
ETHEREUM_WRAPPED_SOL_CONTRACT=<YOUR_DEPLOYED_WSOL_ADDRESS>
ETHEREUM_VALIDATOR_REGISTRY_CONTRACT=<YOUR_DEPLOYED_REGISTRY_ADDRESS>
ETHEREUM_CONFIRMATIONS=12

# Relayer Configuration
POLL_INTERVAL_MS=5000
MAX_RETRIES=3
RETRY_DELAY_MS=2000
GAS_PRICE_MULTIPLIER=1.2

# Database
DATABASE_URL=sqlite://relayer.db
DB_MAX_CONNECTIONS=10

# Logging
RUST_LOG=relayer=info,solana_client=warn,ethers=warn
```

### 3. Build the Relayer

```bash
cargo build --release

# For development (faster compilation)
cargo build
```

### 4. Initialize Database

```bash
# The relayer will create the SQLite database automatically on first run
# Or you can manually initialize if needed
sqlite3 relayer.db < migrations/init.sql  # If migrations exist
```

### 5. Run the Relayer

```bash
# Development mode
cargo run

# Production mode (optimized)
cargo run --release

# With specific log level
RUST_LOG=debug cargo run
```

The relayer will start monitoring both chains and processing bridge transactions.

## Web Application Setup

### 1. Navigate to App Directory

```bash
cd app  # From project root
```

### 2. Install Dependencies

```bash
npm install
```

### 3. Configure Environment Variables

```bash
# Create environment file
cp .env.example .env.local  # Next.js uses .env.local

# Edit configuration
nano .env.local
```

Add the following to `.env.local`:

```env
# Solana Configuration
NEXT_PUBLIC_SOLANA_NETWORK=devnet
NEXT_PUBLIC_SOLANA_RPC_URL=https://api.devnet.solana.com
NEXT_PUBLIC_BRIDGE_PROGRAM_ID=<YOUR_DEPLOYED_PROGRAM_ID>

# Ethereum Configuration
NEXT_PUBLIC_ETHEREUM_CHAIN_ID=11155111
NEXT_PUBLIC_ETHEREUM_RPC_URL=https://sepolia.infura.io/v3/YOUR_INFURA_KEY
NEXT_PUBLIC_BRIDGE_CONTRACT=<YOUR_DEPLOYED_BRIDGE_ADDRESS>
NEXT_PUBLIC_WRAPPED_SOL_CONTRACT=<YOUR_DEPLOYED_WSOL_ADDRESS>
```

### 4. Run Development Server

```bash
npm run dev

# App will be available at http://localhost:3000
```

### 5. Build for Production

```bash
npm run build
npm run start

# Or deploy to Vercel/Netlify
```

## Testing the Bridge

### End-to-End Test Flow

#### 1. Prepare Test Wallets

**Solana Wallet:**
```bash
solana-keygen new --outfile test-wallet.json
solana airdrop 2 --keypair test-wallet.json
```

**Ethereum Wallet:**
- Use MetaMask with Sepolia testnet
- Get test ETH from Sepolia faucet

#### 2. Test Solana to Ethereum Bridge

1. Open the web app at `http://localhost:3000`
2. Connect your Solana wallet (Phantom/Solflare)
3. Enter amount of SOL to bridge
4. Confirm transaction
5. Monitor relayer logs for transaction processing
6. Check wrapped SOL balance on Ethereum (MetaMask)

#### 3. Test Ethereum to Solana Bridge

1. Connect MetaMask to the web app
2. Enter amount of wrapped SOL to unwrap
3. Provide your Solana address
4. Confirm Ethereum transaction
5. Monitor relayer logs
6. Check SOL balance increased on Solana

#### 4. Monitor Transactions

```bash
# Watch Solana transaction
solana confirm -v <TRANSACTION_SIGNATURE>

# Watch Ethereum transaction
# Use Sepolia Etherscan: https://sepolia.etherscan.io/tx/<TX_HASH>

# Check relayer logs
tail -f relayer.log
```

## Troubleshooting

### Common Issues

#### Solana Program Deployment Fails

```bash
# Error: Insufficient funds
solana airdrop 2

# Error: Program already deployed
solana program close <PROGRAM_ID> --bypass-warning

# Error: Buffer account creation failed
solana config set --commitment confirmed
```

#### Ethereum Contract Deployment Fails

```bash
# Error: Insufficient funds
# Get more Sepolia ETH from faucet

# Error: Nonce too low
# Reset MetaMask account: Settings -> Advanced -> Clear activity tab data

# Error: RPC connection failed
# Check Infura project ID and RPC URL
```

#### Relayer Issues

```bash
# Relayer not processing transactions
# 1. Check both RPC endpoints are accessible
curl https://api.devnet.solana.com
curl https://sepolia.infura.io/v3/YOUR_KEY

# 2. Verify contract addresses in .env
# 3. Check relayer has sufficient gas
# 4. Review logs for specific errors
RUST_LOG=debug cargo run

# Database locked error
# Stop all relayer instances and remove lock
rm relayer.db-wal relayer.db-shm
```

#### Web Application Issues

```bash
# Wallet connection fails
# - Check browser wallet extension is installed
# - Verify network matches (devnet/sepolia)
# - Clear browser cache and reload

# Transaction fails immediately
# - Check wallet has sufficient balance
# - Verify contract addresses in .env.local
# - Check browser console for errors

# Build fails
rm -rf .next node_modules
npm install
npm run build
```

### Debugging Tips

#### Enable Verbose Logging

**Solana:**
```bash
export RUST_LOG=solana_runtime::system_instruction_processor=trace
solana logs -v
```

**Ethereum:**
```javascript
// In hardhat.config.ts, add:
networks: {
  sepolia: {
    // ... other config
    loggingEnabled: true
  }
}
```

**Relayer:**
```bash
RUST_LOG=relayer=trace,solana_client=debug,ethers=debug cargo run
```

#### Check Account States

**Solana:**
```bash
solana account <ACCOUNT_ADDRESS>
solana program show <PROGRAM_ID>
```

**Ethereum:**
```bash
npx hardhat console --network sepolia
# Then in console:
const bridge = await ethers.getContractAt("SolanaBridge", "CONTRACT_ADDRESS")
await bridge.processedNonces(NONCE)
```

### Getting Help

If you encounter issues not covered here:

1. Check existing [GitHub Issues](https://github.com/RaghavenderSingh/bridge-vault/issues)
2. Review the [documentation](docs/)
3. Reach out on X: [@itsrsc_](https://x.com/itsrsc_)
4. Open a new issue with:
   - Steps to reproduce
   - Error messages/logs
   - Environment details (OS, versions)
   - What you've already tried

## Next Steps

Once everything is set up and working:

1. Review the [Architecture Documentation](docs/architecture.md)
2. Explore the [API Documentation](docs/api.md)
3. Check out [Contributing Guidelines](CONTRIBUTING.md)
4. Join development discussions in GitHub Issues
5. Consider running a validator node

## Security Reminders

- Never commit `.env` files or private keys
- Use separate wallets for testing (not your main wallets)
- Don't use testnet private keys on mainnet
- Always verify contract addresses before transactions
- Keep your dependencies updated
- Run security scans regularly

## Development Workflow

For active development:

```bash
# Terminal 1: Solana logs
solana logs

# Terminal 2: Relayer
cd relayer && cargo run

# Terminal 3: Web app
cd app && npm run dev

# Terminal 4: Contract development
cd contracts && npx hardhat node
```

This setup allows you to see all components working together in real-time.
