# Ensure the script exits on any errors
set -e

# Check if the argument is provided
if [ -z "$2" ]; then
    echo "Usage: $0 <admin-id-str> <fee-acc-str>"
    exit 1
fi

ADMIN_ID_STR=$1
FEE_WALLET_STR=$2

echo "Building and optimizing contract..."

make build > /dev/null
echo "Contract compiled."

# echo "Optimizing contract..."
# soroban contract optimize --wasm target/wasm32-unknown-unknown/release/soroban_escrow_smart_contract.wasm
# echo "Contract optimized."


# Fetch addresses
ADMIN_ACC=$(soroban config identity address $ADMIN_ID_STR)
echo "ADMIN_ACC:" $ADMIN_ACC
FEE_ACC=$(soroban config identity address $FEE_WALLET_STR)
echo "FEE_ACC:" $FEE_ACC

echo "Deploying the contract and capturing its contract ID..."

# soroban contract deploy \
#     --wasm target/wasm32-unknown-unknown/release/soroban_token_exchange_contract.optimized.wasm \
#     --source $ADMIN_ID_STR \
#     --network Futurenet \
#     > ../.soroban/contract-id
soroban contract deploy \
    --wasm target/wasm32-unknown-unknown/release/soroban_token_exchange_contract.wasm \
    --source $ADMIN_ID_STR \
    --network Futurenet \
    > ../.soroban/contract-id

echo "Contract deployed."

CONTRACT_ID=$(cat ../.soroban/contract-id)
echo "CONTRACT_ID:" $CONTRACT_ID

echo "Initializing contract..."

# soroban contract invoke \
#     --id $CONTRACT_ID \
#     --source $ADMIN_ID_STR \
#     --network Futurenet \
#     -- init \
#     --admin $ADMIN_ACC

soroban contract invoke \
    --id $CONTRACT_ID \
    --source $ADMIN_ID_STR \
    --network Futurenet \
    -- set_fee \
    --fee_rate 25 \
    --fee_wallet $FEE_ACC

soroban contract invoke \
	--id $CONTRACT_ID \
	--network Futurenet \
	-- allow_token \
	--token $(cat ../.soroban/ngnc-id)
soroban contract invoke \
	--id $(cat ../.soroban/contract-id) \
	--network Futurenet \
	-- allow_token \
	--token $(cat ../.soroban/usdc-id)

echo "Contract initialized."
