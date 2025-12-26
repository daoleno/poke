# Phase 3: Complete Toolkit Implementation

> **Date:** 2025-12-26
> **Dependencies:** Phase 1 (basic toolkit), existing ABI infrastructure

## Goal

Implement remaining developer tool commands: encode, decode, call, gas, slot, create, create2

---

## Task Breakdown

### Task 1: Slot Calculation (Low complexity, no dependencies)

**File:** `src/modules/toolkit/slot.rs`

**Features:**
- Calculate storage slot for mappings
- Calculate array element slots
- Support nested mappings

**Syntax:**
- `:slot mapping <slot> <key>` → Calculate mapping[key] slot
- `:slot array <slot> <index>` → Calculate array[index] slot

**Implementation:**
```rust
pub fn slot(input: Option<String>) -> Action
```

Uses keccak256 for slot calculation per Solidity storage layout rules.

---

### Task 2: CREATE Address Calculation

**File:** `src/modules/toolkit/create.rs`

**Features:**
- Calculate CREATE address from deployer + nonce
- Use alloy primitives for RLP encoding

**Syntax:**
- `:create <deployer> <nonce>` → Calculate address

**Implementation:**
```rust
pub fn create_address(deployer: Address, nonce: u64) -> Address
```

Uses: `keccak256(rlp([deployer, nonce]))[12:]`

---

### Task 3: CREATE2 Address Calculation

**File:** `src/modules/toolkit/create2.rs`

**Features:**
- Calculate CREATE2 address from deployer + salt + initcode_hash
- Support both initcode and initcode_hash input

**Syntax:**
- `:create2 <deployer> <salt> <initcode_hash>` → Calculate address

**Implementation:**
```rust
pub fn create2_address(deployer: Address, salt: B256, initcode_hash: B256) -> Address
```

Uses: `keccak256(0xff ++ deployer ++ salt ++ initcode_hash)[12:]`

---

### Task 4: ABI Encode

**File:** `src/modules/toolkit/encode.rs`

**Features:**
- Parse function signature: `transfer(address,uint256)`
- Parse argument values: `0x123..., 1000000`
- Encode to calldata using alloy
- Return hex-encoded calldata

**Syntax:**
- `:encode transfer(address,uint256) 0xABC 1000` → `0xa9059cbb...`
- `:encode` (interactive mode - show help)

**Implementation:**
```rust
pub fn encode(input: Option<String>) -> Action
```

Parse signature → Parse args → Encode → Format output

---

### Task 5: ABI Decode

**File:** `src/modules/toolkit/decode.rs`

**Features:**
- Decode calldata (with function selector)
- Decode raw data (without selector)
- Try to resolve function from selector using cache
- Support manual signature input

**Syntax:**
- `:decode 0xa9059cbb...` → Auto-resolve and decode
- `:decode 0xa9059cbb... transfer(address,uint256)` → Use provided signature

**Implementation:**
```rust
pub fn decode(input: Option<String>, signature_cache: &BTreeMap<String, (String, String)>) -> Action
```

---

### Task 6: Contract Call

**File:** `src/modules/toolkit/call.rs`

**Features:**
- Parse call syntax: `<address>.<function>(<args>)`
- Encode calldata
- Execute eth_call via RPC
- Decode return value
- Support both full syntax and shorthand

**Syntax:**
- `:call 0xUSDC.balanceOf(0x123)` → `1000000`
- `:call 0xUSDC.0xa9059cbb... 0x123` → Use raw selector

**Implementation:**
```rust
pub async fn call(input: Option<String>, provider: &Provider) -> Action
```

This is async! Needs runtime integration.

---

### Task 7: Gas Estimation

**File:** `src/modules/toolkit/gas.rs`

**Features:**
- Same parsing as `:call`
- Execute eth_estimateGas
- Return gas estimate

**Syntax:**
- `:gas 0xRouter.swap(...)` → `150000 gas`

**Implementation:**
```rust
pub async fn estimate_gas(input: Option<String>, provider: &Provider) -> Action
```

Also async, needs runtime integration.

---

### Task 8: Wire Up Commands in App

Update `src/app.rs` `execute_command()` to:
1. Add sync commands (slot, create, create2, encode, decode)
2. Handle async commands (call, gas) via runtime bridge
3. Add command hints for new commands

---

## Implementation Strategy

### Execution Order

1. **Sync commands first** (Tasks 1-5): slot → create → create2 → encode → decode
2. **Async commands** (Tasks 6-7): call → gas (needs runtime pattern)
3. **Integration** (Task 8): Wire up in App

### For Async Commands

Two approaches:
1. **Queue to runtime** - Send command to async worker, poll for result
2. **Block with timeout** - Use tokio::Runtime::block_on in command context

For now, use approach 2 (simpler). Future: move to approach 1 for better UX.

---

## Testing Strategy

Each command should have:
- Unit tests for core logic
- Integration tests with example inputs
- Error case handling

---

## Success Criteria

✓ All 7 toolkit commands implemented
✓ Commands work with valid inputs
✓ Error messages clear for invalid inputs
✓ Tests pass
✓ Commands wired up in App
✓ Help text updated

---

## Commits Structure

~10 commits:
1. `feat(toolkit): add storage slot calculation`
2. `feat(toolkit): add CREATE address calculation`
3. `feat(toolkit): add CREATE2 address calculation`
4. `feat(toolkit): add ABI encode command`
5. `feat(toolkit): add ABI decode command`
6. `feat(toolkit): add contract call command`
7. `feat(toolkit): add gas estimation command`
8. `feat(toolkit): wire up new commands in App`
9. `docs(toolkit): update command hints for new features`
10. `test(toolkit): add integration tests for encode/decode/call`
