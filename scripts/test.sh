# Ensure the script exits on any errors
set -e

# Check if the argument is provided
if [ -z "$3" ]; then
    echo "Usage: $0 <creator-str> <worker1-str> <worker2-str>"
    exit 1
fi

CREATOR_STR=$1
WORKER1_STR=$2
WORKER2_STR=$3

# Fetch addresses
CREATOR_ACC=$(soroban config identity address $CREATOR_STR)
echo "CREATOR_ACC:" $CREATOR_ACC
WORKER1_ACC=$(soroban config identity address $WORKER1_STR)
echo "WORKER1_ACC:" $WORKER1_ACC
WORKER2_ACC=$(soroban config identity address $WORKER2_STR)
echo "WORKER2_ACC:" $WORKER2_ACC
CONTRACT_ID=$(cat ../.soroban/contract-id)
echo "CONTRACT_ID:" $CONTRACT_ID
NATIVE_ASSET_ID=$(cat ../.soroban/native-asset-id)
echo "NATIVE_ASSET_ID:" $NATIVE_ASSET_ID

# approving creator's balance to contract
# soroban contract invoke \
# 	--id $NATIVE_ASSET_ID \
# 	--source $CREATOR_STR \
# 	--network Futurenet \
# 	-- approve \
# 	--from $CREATOR_ACC \
# 	--spender $CONTRACT_ID \
# 	--amount 100000000000 \
# 	--expiration_ledger 535680
# soroban contract invoke \
# 	--id $NATIVE_ASSET_ID \
# 	--network Futurenet \
# 	-- allowance \
# 	--from $CREATOR_ACC \
# 	--spender $CONTRACT_ID

# create bounty1, bounty2
soroban contract invoke \
	--id $CONTRACT_ID \
	--source $CREATOR_STR \
	--network Futurenet \
	--fee 1000000 \
	-- create_bounty \
	--creator $CREATOR_ACC \
	--name "Bounty1" \
	--reward 1000000000 \
	--pay_token $NATIVE_ASSET_ID \
	--deadline 86400 \
	> ../.soroban/bounty1-id
echo 'bounty1-id:' $(cat ../.soroban/bounty1-id)
soroban contract invoke \
	--id $CONTRACT_ID \
	--source $CREATOR_STR \
	--network Futurenet \
	--fee 1000000 \
	-- create_bounty \
	--creator $CREATOR_ACC \
	--name "Bounty2" \
	--reward 1000000000 \
	--pay_token $NATIVE_ASSET_ID \
	--deadline 86400 \
	> ../.soroban/bounty2-id
echo 'bounty2-id:' $(cat ../.soroban/bounty2-id)

# apply to bounty1
soroban contract invoke \
	--id $CONTRACT_ID \
	--source $WORKER1_STR \
	--network Futurenet \
	-- apply_bounty \
	--participant $WORKER1_ACC \
	--bounty_id $(cat ../.soroban/bounty1-id) \
	> ../.soroban/work1-id
echo 'work1-id:' $(cat ../.soroban/work1-id)
soroban contract invoke \
	--id $CONTRACT_ID \
	--source $WORKER2_STR \
	--network Futurenet \
	-- apply_bounty \
	--participant $WORKER2_ACC \
	--bounty_id $(cat ../.soroban/bounty1-id) \
	> ../.soroban/work2-id
echo 'work2-id:' $(cat ../.soroban/work2-id)

# submit works
soroban contract invoke \
	--id $CONTRACT_ID \
	--source $WORKER1_STR \
	--network Futurenet \
	-- submit_work \
	--participant $WORKER1_ACC \
	--work_id $(cat ../.soroban/work1-id)
soroban contract invoke \
	--id $CONTRACT_ID \
	--source $WORKER2_STR \
	--network Futurenet \
	-- submit_work \
	--participant $WORKER2_ACC \
	--work_id $(cat ../.soroban/work2-id)

# reject work1, approve work2
soroban contract invoke \
	--id $CONTRACT_ID \
	--source $CREATOR_STR \
	--network Futurenet \
	-- reject_work \
	--creator $CREATOR_ACC \
	--work_id $(cat ../.soroban/work1-id)
soroban contract invoke \
	--id $CONTRACT_ID \
	--source $CREATOR_STR \
	--network Futurenet \
	-- approve_work \
	--creator $CREATOR_ACC \
	--work_id $(cat ../.soroban/work2-id)

# cancel bounty2
soroban contract invoke \
	--id $CONTRACT_ID \
	--source $CREATOR_STR \
	--network Futurenet \
	-- cancel_bounty \
	--creator $CREATOR_ACC \
	--bounty_id $(cat ../.soroban/bounty2-id)
