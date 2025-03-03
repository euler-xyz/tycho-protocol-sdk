# EulerSwap Substream

This substream is used to extract data from the EulerSwap protocol.

## How To:

### Add EulerSwapAdapter to the Tycho Simulation

```bash
cd ../../evm/
./scripts/buildRuntime.sh -c "EulerSwapAdapter" -s "constructor(address,address)" -a "0x04C54FF83e4BC428FD1eDA2f41cdBd583A2e9cF8,0x64A8410D7D2ecF3Aaf32b6C3932e4586f3C42ecE"

```

### Build & Run

- To build the substream, run `cargo build --release --target wasm32-unknown-unknown`
- To run the substream, run `substreams gui substreams.yaml map_protocol_changes`

### Testing

#### Running python Tycho Indexer tests

```bash
# Activate conda environment
conda activate tycho-protocol-sdk-testing

# Setup Environment Variables
export RPC_URL="https://ethereum-mainnet.core.chainstack.com/123123123123" # Make sure to use an RPC that supports debug_storageRangeAt endpoint.
export SUBSTREAMS_API_TOKEN=eyJhbGci...

# Build EulerSwap's Substreams wasm
cd substreams
cargo build --release --package ethereum-eulerswap --target wasm32-unknown-unknown
cd ..

# Run Postgres DB using Docker compose
cd testing
docker compose up -d db
cd ..

# Run the testing file
python ./testing/src/runner/cli.py --package "ethereum-eulerswap" --tycho-logs --vm-traces
```

## Data Model


