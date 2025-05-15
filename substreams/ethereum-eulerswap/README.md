# EulerSwap Substream

This substream is used to extract data from the EulerSwap protocol.

## How To:

### Add EulerSwapAdapter to the Tycho Simulation

```bash
cd ../../evm/
./scripts/buildRuntime.sh -c "EulerSwapAdapter" -s "constructor(address,address)" -a "0xae27c4cb02c73f5594430302d44d6634a0d52d2e,0x4fE0547e7Be0e9a9cED3aC948B83146996f899aE"

```

### Build & Run

- To build the substream, run `cargo build --release --target wasm32-unknown-unknown`
- To run the substream, run `substreams gui ./substreams.yaml -e mainnet.eth.streamingfast.io:443 -t 22488263 --limit-processed-blocks 100000`

### Testing

#### Running python Tycho Indexer tests

Make sure to run `setup_env.sh` in testing folder, this will download python dependency and set conda virtual env.

```bash
# Activate conda environment
conda activate tycho-protocol-sdk-testing

# Setup Environment Variables
export RPC_URL="https://ethereum-mainnet.core.chainstack.com/123123123123" # Make sure to use an RPC that supports debug_storageRangeAt endpoint.
export SUBSTREAMS_API_TOKEN=eyJhbGci...

# Build EulerSwap's Substreams wasm
cd substreams
cargo build --release --package "ethereum-eulerswap" --target wasm32-unknown-unknown
cd ..

# Run Postgres DB using Docker compose
cd testing
docker compose up -d db
cd ..

# Run the testing file
python ./testing/src/runner/cli.py --package "ethereum-eulerswap" --tycho-logs --vm-traces
```

## Data Model


