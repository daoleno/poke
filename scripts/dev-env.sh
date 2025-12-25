#!/bin/bash
# Development Environment for Poke
#
# Usage: ./scripts/dev-env.sh
# Then:  cargo run -- --rpc http://127.0.0.1:8545

set -e

PORT=${ANVIL_PORT:-8545}
RPC_URL="http://127.0.0.1:$PORT"
TMP_DIR=$(mktemp -d)

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

cleanup() {
    echo ""
    echo -e "${YELLOW}Stopping...${NC}"
    pkill -f "anvil.*--port $PORT" 2>/dev/null || true
    kill $BG_PID 2>/dev/null || true
    rm -rf "$TMP_DIR"
    echo "Done."
}
trap cleanup EXIT

pkill -f "anvil.*--port $PORT" 2>/dev/null || true
sleep 1

echo "========================================"
echo "  Poke Development Environment"
echo "========================================"
echo ""

for cmd in anvil cast forge; do
    if ! command -v $cmd &> /dev/null; then
        echo "Error: '$cmd' not found. Install foundry:"
        echo "  curl -L https://foundry.paradigm.xyz | bash && foundryup"
        exit 1
    fi
done

echo "Starting Anvil..."
anvil --block-time 1 --accounts 5 --balance 10000 --port $PORT 2>&1 | grep -v "eth_\|net_\|Transaction\|Contract\|Block" &
ANVIL_PID=$!
sleep 3

if ! cast block-number --rpc-url "$RPC_URL" &>/dev/null; then
    echo "Error: Anvil failed to start"
    exit 1
fi

echo -e "${GREEN}Anvil ready${NC}"
echo ""

ACCOUNTS=(
    "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
    "0x70997970C51812dc3A010C7d01b50e0d17dc79C8"
    "0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC"
    "0x90F79bf6EB2c4f870365E785982E1f101E93b906"
    "0x15d34AAf54267DB7D7c367839AAf71A00a2C6A65"
)
PRIVATE_KEYS=(
    "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
    "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"
    "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a"
    "0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6"
    "0x47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a"
)

# Create DeFi-style contracts for complex call traces
echo "Deploying contracts..."
mkdir -p "$TMP_DIR/src"
cat > "$TMP_DIR/foundry.toml" << 'EOF'
[profile.default]
src = "src"
out = "out"
libs = ["lib"]
EOF

# Multi-contract DeFi system for rich call traces
cat > "$TMP_DIR/src/DeFiSystem.sol" << 'EOF'
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

// Simple ERC20 Token
contract Token {
    string public name;
    string public symbol;
    uint8 public decimals = 18;
    uint256 public totalSupply;
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;

    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);

    constructor(string memory _name, string memory _symbol) {
        name = _name;
        symbol = _symbol;
    }

    function mint(address to, uint256 amount) external {
        totalSupply += amount;
        balanceOf[to] += amount;
        emit Transfer(address(0), to, amount);
    }

    function transfer(address to, uint256 amount) external returns (bool) {
        return _transfer(msg.sender, to, amount);
    }

    function transferFrom(address from, address to, uint256 amount) external returns (bool) {
        uint256 allowed = allowance[from][msg.sender];
        if (allowed != type(uint256).max) {
            allowance[from][msg.sender] = allowed - amount;
        }
        return _transfer(from, to, amount);
    }

    function approve(address spender, uint256 amount) external returns (bool) {
        allowance[msg.sender][spender] = amount;
        emit Approval(msg.sender, spender, amount);
        return true;
    }

    function _transfer(address from, address to, uint256 amount) internal returns (bool) {
        balanceOf[from] -= amount;
        balanceOf[to] += amount;
        emit Transfer(from, to, amount);
        return true;
    }
}

// Liquidity Pool - holds two tokens
contract LiquidityPool {
    Token public token0;
    Token public token1;
    uint256 public reserve0;
    uint256 public reserve1;
    uint256 public totalLiquidity;
    mapping(address => uint256) public liquidity;

    event Swap(address indexed sender, uint256 amountIn, uint256 amountOut, bool zeroForOne);
    event AddLiquidity(address indexed provider, uint256 amount0, uint256 amount1, uint256 liquidity);
    event RemoveLiquidity(address indexed provider, uint256 amount0, uint256 amount1, uint256 liquidity);

    constructor(address _token0, address _token1) {
        token0 = Token(_token0);
        token1 = Token(_token1);
    }

    function getReserves() external view returns (uint256, uint256) {
        return (reserve0, reserve1);
    }

    function addLiquidity(uint256 amount0, uint256 amount1) external returns (uint256) {
        token0.transferFrom(msg.sender, address(this), amount0);
        token1.transferFrom(msg.sender, address(this), amount1);

        uint256 liq;
        if (totalLiquidity == 0) {
            liq = sqrt(amount0 * amount1);
        } else {
            liq = min(amount0 * totalLiquidity / reserve0, amount1 * totalLiquidity / reserve1);
        }

        liquidity[msg.sender] += liq;
        totalLiquidity += liq;
        reserve0 += amount0;
        reserve1 += amount1;

        emit AddLiquidity(msg.sender, amount0, amount1, liq);
        return liq;
    }

    function removeLiquidity(uint256 liq) external returns (uint256, uint256) {
        uint256 amount0 = liq * reserve0 / totalLiquidity;
        uint256 amount1 = liq * reserve1 / totalLiquidity;

        liquidity[msg.sender] -= liq;
        totalLiquidity -= liq;
        reserve0 -= amount0;
        reserve1 -= amount1;

        token0.transfer(msg.sender, amount0);
        token1.transfer(msg.sender, amount1);

        emit RemoveLiquidity(msg.sender, amount0, amount1, liq);
        return (amount0, amount1);
    }

    function swap(uint256 amountIn, bool zeroForOne) external returns (uint256 amountOut) {
        (Token tokenIn, Token tokenOut, uint256 reserveIn, uint256 reserveOut) = zeroForOne
            ? (token0, token1, reserve0, reserve1)
            : (token1, token0, reserve1, reserve0);

        tokenIn.transferFrom(msg.sender, address(this), amountIn);

        // x * y = k, with 0.3% fee
        uint256 amountInWithFee = amountIn * 997;
        amountOut = (amountInWithFee * reserveOut) / (reserveIn * 1000 + amountInWithFee);

        tokenOut.transfer(msg.sender, amountOut);

        if (zeroForOne) {
            reserve0 += amountIn;
            reserve1 -= amountOut;
        } else {
            reserve1 += amountIn;
            reserve0 -= amountOut;
        }

        emit Swap(msg.sender, amountIn, amountOut, zeroForOne);
    }

    function sqrt(uint256 x) internal pure returns (uint256) {
        if (x == 0) return 0;
        uint256 z = (x + 1) / 2;
        uint256 y = x;
        while (z < y) { y = z; z = (x / z + z) / 2; }
        return y;
    }

    function min(uint256 a, uint256 b) internal pure returns (uint256) {
        return a < b ? a : b;
    }
}

// Router - routes swaps, can do multi-hop
contract Router {
    event SwapExecuted(address indexed user, address indexed pool, uint256 amountIn, uint256 amountOut);

    function swapExact(address pool, uint256 amountIn, bool zeroForOne) external returns (uint256) {
        LiquidityPool lp = LiquidityPool(pool);

        // Get tokens and transfer from user
        Token tokenIn = zeroForOne ? lp.token0() : lp.token1();
        tokenIn.transferFrom(msg.sender, address(this), amountIn);
        tokenIn.approve(pool, amountIn);

        uint256 amountOut = lp.swap(amountIn, zeroForOne);

        // Transfer output to user
        Token tokenOut = zeroForOne ? lp.token1() : lp.token0();
        tokenOut.transfer(msg.sender, amountOut);

        emit SwapExecuted(msg.sender, pool, amountIn, amountOut);
        return amountOut;
    }

    function multiHopSwap(
        address[] calldata pools,
        bool[] calldata directions,
        uint256 amountIn
    ) external returns (uint256) {
        require(pools.length == directions.length, "Length mismatch");

        uint256 currentAmount = amountIn;

        // Get first token
        LiquidityPool firstPool = LiquidityPool(pools[0]);
        Token currentToken = directions[0] ? firstPool.token0() : firstPool.token1();
        currentToken.transferFrom(msg.sender, address(this), amountIn);

        for (uint256 i = 0; i < pools.length; i++) {
            LiquidityPool pool = LiquidityPool(pools[i]);
            currentToken.approve(pools[i], currentAmount);
            currentAmount = pool.swap(currentAmount, directions[i]);
            currentToken = directions[i] ? pool.token1() : pool.token0();
        }

        // Transfer final output to user
        currentToken.transfer(msg.sender, currentAmount);
        return currentAmount;
    }
}

// Aggregator - finds best route and executes (simulates 1inch/paraswap)
contract Aggregator {
    Router public router;

    event AggregatedSwap(address indexed user, uint256 amountIn, uint256 amountOut, uint256 numPools);

    constructor(address _router) {
        router = Router(_router);
    }

    function findBestRoute(
        address[] calldata pools,
        uint256 amountIn
    ) external view returns (uint256 bestOutput, uint256 bestPoolIndex) {
        for (uint256 i = 0; i < pools.length; i++) {
            LiquidityPool pool = LiquidityPool(pools[i]);
            (uint256 r0, uint256 r1) = pool.getReserves();
            uint256 output = (amountIn * 997 * r1) / (r0 * 1000 + amountIn * 997);
            if (output > bestOutput) {
                bestOutput = output;
                bestPoolIndex = i;
            }
        }
    }

    function aggregateSwap(
        address[] calldata pools,
        bool[] calldata directions,
        uint256 amountIn,
        uint256 minAmountOut
    ) external returns (uint256 amountOut) {
        // Transfer tokens from user to this contract
        LiquidityPool firstPool = LiquidityPool(pools[0]);
        Token tokenIn = directions[0] ? firstPool.token0() : firstPool.token1();
        tokenIn.transferFrom(msg.sender, address(this), amountIn);
        tokenIn.approve(address(router), amountIn);

        // Build arrays for router (remove first since we handle it)
        amountOut = _executeRoute(pools, directions, amountIn);

        require(amountOut >= minAmountOut, "Slippage too high");

        // Get final token and transfer to user
        LiquidityPool lastPool = LiquidityPool(pools[pools.length - 1]);
        Token tokenOut = directions[directions.length - 1] ? lastPool.token1() : lastPool.token0();
        tokenOut.transfer(msg.sender, amountOut);

        emit AggregatedSwap(msg.sender, amountIn, amountOut, pools.length);
    }

    function _executeRoute(
        address[] calldata pools,
        bool[] calldata directions,
        uint256 amountIn
    ) internal returns (uint256) {
        uint256 currentAmount = amountIn;
        Token currentToken;

        for (uint256 i = 0; i < pools.length; i++) {
            LiquidityPool pool = LiquidityPool(pools[i]);
            currentToken = directions[i] ? pool.token0() : pool.token1();
            currentToken.approve(pools[i], currentAmount);
            currentAmount = pool.swap(currentAmount, directions[i]);
        }

        return currentAmount;
    }
}

// Vault with staking rewards (more complex)
contract StakingVault {
    Token public stakingToken;
    Token public rewardToken;

    uint256 public rewardRate = 100; // rewards per block
    uint256 public lastUpdateBlock;
    uint256 public rewardPerTokenStored;

    mapping(address => uint256) public balances;
    mapping(address => uint256) public userRewardPerTokenPaid;
    mapping(address => uint256) public rewards;
    uint256 public totalStaked;

    event Staked(address indexed user, uint256 amount);
    event Withdrawn(address indexed user, uint256 amount);
    event RewardPaid(address indexed user, uint256 reward);

    constructor(address _stakingToken, address _rewardToken) {
        stakingToken = Token(_stakingToken);
        rewardToken = Token(_rewardToken);
        lastUpdateBlock = block.number;
    }

    function rewardPerToken() public view returns (uint256) {
        if (totalStaked == 0) return rewardPerTokenStored;
        return rewardPerTokenStored +
            ((block.number - lastUpdateBlock) * rewardRate * 1e18) / totalStaked;
    }

    function earned(address account) public view returns (uint256) {
        return (balances[account] * (rewardPerToken() - userRewardPerTokenPaid[account])) / 1e18
            + rewards[account];
    }

    modifier updateReward(address account) {
        rewardPerTokenStored = rewardPerToken();
        lastUpdateBlock = block.number;
        if (account != address(0)) {
            rewards[account] = earned(account);
            userRewardPerTokenPaid[account] = rewardPerTokenStored;
        }
        _;
    }

    function stake(uint256 amount) external updateReward(msg.sender) {
        require(amount > 0, "Cannot stake 0");
        totalStaked += amount;
        balances[msg.sender] += amount;
        stakingToken.transferFrom(msg.sender, address(this), amount);
        emit Staked(msg.sender, amount);
    }

    function withdraw(uint256 amount) external updateReward(msg.sender) {
        require(amount > 0, "Cannot withdraw 0");
        totalStaked -= amount;
        balances[msg.sender] -= amount;
        stakingToken.transfer(msg.sender, amount);
        emit Withdrawn(msg.sender, amount);
    }

    function claimReward() external updateReward(msg.sender) {
        uint256 reward = rewards[msg.sender];
        if (reward > 0) {
            rewards[msg.sender] = 0;
            rewardToken.transfer(msg.sender, reward);
            emit RewardPaid(msg.sender, reward);
        }
    }

    function exit() external updateReward(msg.sender) {
        uint256 amount = balances[msg.sender];
        if (amount > 0) {
            totalStaked -= amount;
            balances[msg.sender] = 0;
            stakingToken.transfer(msg.sender, amount);
            emit Withdrawn(msg.sender, amount);
        }
        uint256 reward = rewards[msg.sender];
        if (reward > 0) {
            rewards[msg.sender] = 0;
            rewardToken.transfer(msg.sender, reward);
            emit RewardPaid(msg.sender, reward);
        }
    }
}
EOF

cd "$TMP_DIR"

echo -e "  ${CYAN}Compiling...${NC}"
forge build --silent 2>/dev/null || forge build

# Deploy Token A
TOKEN_A=$(forge create --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[0]}" --broadcast \
    src/DeFiSystem.sol:Token --constructor-args "TokenA" "TKA" 2>&1 | grep -oE "Deployed to: 0x[a-fA-F0-9]{40}" | cut -d' ' -f3)

# Deploy Token B
TOKEN_B=$(forge create --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[0]}" --broadcast \
    src/DeFiSystem.sol:Token --constructor-args "TokenB" "TKB" 2>&1 | grep -oE "Deployed to: 0x[a-fA-F0-9]{40}" | cut -d' ' -f3)

# Deploy Token C
TOKEN_C=$(forge create --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[0]}" --broadcast \
    src/DeFiSystem.sol:Token --constructor-args "TokenC" "TKC" 2>&1 | grep -oE "Deployed to: 0x[a-fA-F0-9]{40}" | cut -d' ' -f3)

# Deploy Pool A-B
POOL_AB=$(forge create --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[0]}" --broadcast \
    src/DeFiSystem.sol:LiquidityPool --constructor-args "$TOKEN_A" "$TOKEN_B" 2>&1 | grep -oE "Deployed to: 0x[a-fA-F0-9]{40}" | cut -d' ' -f3)

# Deploy Pool B-C
POOL_BC=$(forge create --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[0]}" --broadcast \
    src/DeFiSystem.sol:LiquidityPool --constructor-args "$TOKEN_B" "$TOKEN_C" 2>&1 | grep -oE "Deployed to: 0x[a-fA-F0-9]{40}" | cut -d' ' -f3)

# Deploy Router
ROUTER=$(forge create --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[0]}" --broadcast \
    src/DeFiSystem.sol:Router 2>&1 | grep -oE "Deployed to: 0x[a-fA-F0-9]{40}" | cut -d' ' -f3)

# Deploy Aggregator
AGGREGATOR=$(forge create --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[0]}" --broadcast \
    src/DeFiSystem.sol:Aggregator --constructor-args "$ROUTER" 2>&1 | grep -oE "Deployed to: 0x[a-fA-F0-9]{40}" | cut -d' ' -f3)

# Deploy StakingVault
STAKING=$(forge create --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[0]}" --broadcast \
    src/DeFiSystem.sol:StakingVault --constructor-args "$TOKEN_A" "$TOKEN_B" 2>&1 | grep -oE "Deployed to: 0x[a-fA-F0-9]{40}" | cut -d' ' -f3)

cd - > /dev/null

echo -e "  ${GREEN}Tokens:${NC}"
echo "    TokenA: $TOKEN_A"
echo "    TokenB: $TOKEN_B"
echo "    TokenC: $TOKEN_C"
echo -e "  ${GREEN}Pools:${NC}"
echo "    Pool A-B: $POOL_AB"
echo "    Pool B-C: $POOL_BC"
echo -e "  ${GREEN}Routers:${NC}"
echo "    Router: $ROUTER"
echo "    Aggregator: $AGGREGATOR"
echo -e "  ${GREEN}Staking:${NC}"
echo "    StakingVault: $STAKING"
echo ""

# Generate transactions
echo "Generating transactions..."
tx_count=0

# Setup: Mint tokens to accounts 0-2 only (fewer accounts = faster)
echo -e "  ${CYAN}Minting tokens...${NC}"
MINT_AMOUNT="1000000000000000000000000" # 1M tokens
for i in {0..2}; do
    cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[0]}" \
        "$TOKEN_A" "mint(address,uint256)" "${ACCOUNTS[$i]}" "$MINT_AMOUNT" &>/dev/null
    cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[0]}" \
        "$TOKEN_B" "mint(address,uint256)" "${ACCOUNTS[$i]}" "$MINT_AMOUNT" &>/dev/null
    cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[0]}" \
        "$TOKEN_C" "mint(address,uint256)" "${ACCOUNTS[$i]}" "$MINT_AMOUNT" &>/dev/null
done

# Mint rewards to StakingVault
cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[0]}" \
    "$TOKEN_B" "mint(address,uint256)" "$STAKING" "10000000000000000000000000" &>/dev/null

# Setup: Approve contracts (sequential to avoid nonce issues)
echo -e "  ${CYAN}Approving contracts...${NC}"
MAX_UINT="115792089237316195423570985008687907853269984665640564039457584007913129639935"
for i in {0..2}; do
    cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[$i]}" \
        "$TOKEN_A" "approve(address,uint256)" "$POOL_AB" "$MAX_UINT" &>/dev/null
    cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[$i]}" \
        "$TOKEN_B" "approve(address,uint256)" "$POOL_AB" "$MAX_UINT" &>/dev/null
    cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[$i]}" \
        "$TOKEN_B" "approve(address,uint256)" "$POOL_BC" "$MAX_UINT" &>/dev/null
    cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[$i]}" \
        "$TOKEN_C" "approve(address,uint256)" "$POOL_BC" "$MAX_UINT" &>/dev/null
    cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[$i]}" \
        "$TOKEN_A" "approve(address,uint256)" "$ROUTER" "$MAX_UINT" &>/dev/null
    cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[$i]}" \
        "$TOKEN_B" "approve(address,uint256)" "$ROUTER" "$MAX_UINT" &>/dev/null
    cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[$i]}" \
        "$TOKEN_C" "approve(address,uint256)" "$ROUTER" "$MAX_UINT" &>/dev/null
    cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[$i]}" \
        "$TOKEN_A" "approve(address,uint256)" "$STAKING" "$MAX_UINT" &>/dev/null
done

# Add liquidity to pools (creates 3-4 level call traces)
echo -e "  ${CYAN}Adding liquidity...${NC}"
LIQ_AMOUNT="100000000000000000000000" # 100k tokens
for i in {0..2}; do
    # Pool A-B: addLiquidity -> transferFrom(A) -> transferFrom(B)
    cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[$i]}" \
        "$POOL_AB" "addLiquidity(uint256,uint256)" "$LIQ_AMOUNT" "$LIQ_AMOUNT" &>/dev/null && tx_count=$((tx_count + 1))
    # Pool B-C
    cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[$i]}" \
        "$POOL_BC" "addLiquidity(uint256,uint256)" "$LIQ_AMOUNT" "$LIQ_AMOUNT" &>/dev/null && tx_count=$((tx_count + 1))
done

# Direct pool swaps (3 levels: swap -> transferFrom -> transfer)
echo -e "  ${CYAN}Pool swaps...${NC}"
SWAP_AMOUNT="1000000000000000000000" # 1k tokens
for i in {0..2}; do
    cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[$i]}" \
        "$POOL_AB" "swap(uint256,bool)" "$SWAP_AMOUNT" "true" &>/dev/null && tx_count=$((tx_count + 1))
done

# Router swaps (4 levels: swapExact -> transferFrom -> approve -> swap -> ...)
echo -e "  ${CYAN}Router swaps...${NC}"
for i in {0..2}; do
    cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[$i]}" \
        "$ROUTER" "swapExact(address,uint256,bool)" "$POOL_AB" "$SWAP_AMOUNT" "true" &>/dev/null && tx_count=$((tx_count + 1))
done

# Multi-hop swaps (5+ levels: multiHopSwap -> loop of (approve -> swap -> transferFrom -> transfer))
echo -e "  ${CYAN}Multi-hop swaps (A->B->C)...${NC}"
POOLS_ARR="[$POOL_AB,$POOL_BC]"
DIRS_ARR="[true,true]"
for i in {0..2}; do
    cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[$i]}" \
        "$ROUTER" "multiHopSwap(address[],bool[],uint256)" "$POOLS_ARR" "$DIRS_ARR" "$SWAP_AMOUNT" &>/dev/null && tx_count=$((tx_count + 1))
done

# Staking operations (3-4 levels with reward calculations)
echo -e "  ${CYAN}Staking...${NC}"
STAKE_AMOUNT="10000000000000000000000" # 10k tokens
for i in {0..2}; do
    cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[$i]}" \
        "$STAKING" "stake(uint256)" "$STAKE_AMOUNT" &>/dev/null && tx_count=$((tx_count + 1))
done

# Remove liquidity (3 levels)
echo -e "  ${CYAN}Removing liquidity...${NC}"
cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[0]}" \
    "$POOL_AB" "removeLiquidity(uint256)" "1000000000000000000000" &>/dev/null && tx_count=$((tx_count + 1))

# Staking withdraw + claim (complex: withdraw -> transfer, claimReward -> transfer)
echo -e "  ${CYAN}Staking withdrawals...${NC}"
cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[0]}" \
    "$STAKING" "exit()" &>/dev/null && tx_count=$((tx_count + 1))

sleep 2
echo -e "${GREEN}Generated $tx_count transactions${NC}"
echo ""

# Background activity with varied operations
echo "Starting background activity..."
(
    idx=0
    while true; do
        from_idx=$((idx % 3))
        op=$((idx % 6))

        case $op in
            0)
                # Pool swap A->B
                cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[$from_idx]}" \
                    "$POOL_AB" "swap(uint256,bool)" "100000000000000000000" "true" &>/dev/null
                ;;
            1)
                # Pool swap B->A
                cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[$from_idx]}" \
                    "$POOL_AB" "swap(uint256,bool)" "100000000000000000000" "false" &>/dev/null
                ;;
            2)
                # Router swap
                cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[$from_idx]}" \
                    "$ROUTER" "swapExact(address,uint256,bool)" "$POOL_BC" "100000000000000000000" "true" &>/dev/null
                ;;
            3)
                # Multi-hop A->B->C
                cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[$from_idx]}" \
                    "$ROUTER" "multiHopSwap(address[],bool[],uint256)" "[$POOL_AB,$POOL_BC]" "[true,true]" "100000000000000000000" &>/dev/null
                ;;
            4)
                # Stake
                cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[$from_idx]}" \
                    "$STAKING" "stake(uint256)" "1000000000000000000000" &>/dev/null
                ;;
            5)
                # ETH transfer (simple)
                to_idx=$(((idx + 1) % 3))
                cast send --rpc-url "$RPC_URL" --private-key "${PRIVATE_KEYS[$from_idx]}" \
                    "${ACCOUNTS[$to_idx]}" --value "0.01ether" &>/dev/null
                ;;
        esac

        idx=$((idx + 1))
        sleep 1
    done
) &
BG_PID=$!

echo ""
echo "========================================"
echo -e "  ${GREEN}Ready!${NC}"
echo "========================================"
echo ""
echo "Test data:"
echo "  - 5 accounts with 1M tokens each"
echo "  - 3 ERC20 tokens (TKA, TKB, TKC)"
echo "  - 2 Liquidity Pools (A-B, B-C)"
echo "  - Router with multi-hop swaps"
echo "  - Staking Vault with rewards"
echo ""
echo "Call trace depths:"
echo "  - Pool swap:      3 levels (swap -> transferFrom -> transfer)"
echo "  - Router swap:    5 levels (swapExact -> ... -> swap -> ...)"
echo "  - Multi-hop:      7+ levels (multiHopSwap -> loop)"
echo "  - Staking exit:   4 levels (exit -> withdraw -> claimReward -> ...)"
echo ""
echo -e "Run: ${YELLOW}cargo run -- --rpc $RPC_URL${NC}"
echo ""
echo "Press Ctrl+C to stop."

wait $ANVIL_PID
