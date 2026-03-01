# Contract Deployment and Update Guide

This guide provides detailed instructions for deploying and updating Espresso Sequencer contracts.

## Deployment Options

### Local Testnet Deployment

1. Start a local testnet:
```bash
anvil
```

2. Deploy contracts:
```bash
forge script DeployHotShot --broadcast --rpc-url local
```

### Sepolia Testnet Deployment

1. Set up environment variables:
```bash
export SEPOLIA_RPC_URL="YOUR_RPC_URL"
export MNEMONIC="your mnemonic phrase"
export ETHERSCAN_API_KEY="your_api_key"  # for verification
```

2. Deploy and verify contracts:
```bash
forge script DeployHotShot --broadcast --rpc-url sepolia --verify
```

Deployment details are saved in `contracts/broadcast/$CHAIN_ID/`.

## Updating Contracts

Follow these steps to update existing contracts:

1. Make changes to contract code in `contracts/src/`
2. Compile contracts:
```bash
forge build
```

3. Update Rust bindings:
```bash
just gen-bindings
```

4. Run tests:
```bash
just sol-test
```

5. Check gas consumption (optional but recommended):
```bash
just gas-benchmarks
```

6. Deploy updated contracts following the deployment instructions above

## Gas Profiling

For detailed gas consumption analysis:

1. Set up environment variables as described in the Sepolia deployment section
2. Run profiling:
```bash
just lc-contract-profiling-sepolia
```
3. Create an account on [sentio.xyz](https://app.sentio.xyz/)
4. Use the transaction hash from step 2 when calling `newFinalizedState` to view the gas profile

## Important Notes

- Production contracts should be placed in `contracts/src/`
- Demo contracts should be placed in `contracts/demo/`
- Always verify gas consumption after making contract updates
- Consider using [Sentio](https://sentio.xyz) for detailed gas profiling
- Keep deployment details from `contracts/broadcast/$CHAIN_ID/` for reference
