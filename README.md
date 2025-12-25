# Poke 👆

**Poke your Ethereum node, get instant feedback.**

A lightweight, zero-config, terminal-native tool for Ethereum development and debugging. Stop switching between your terminal and block explorers — just poke.

![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

<!-- TODO: Add demo GIF here -->
<!-- ![Demo](./assets/demo.gif) -->

## Why Poke?

- **Local Development**: Running Anvil but tired of spamming `cast balance`? Poke shows real-time updates.
- **Transaction Debugging**: Failed tx? Get the full call trace without leaving your terminal.
- **No Browser Needed**: SSH'd into a server? WSL? Docker? Poke works everywhere.
- **Node Monitoring**: Check sync status, gas prices, and peers at a glance.

## Features

### Live Dashboard
- Real-time block and transaction stream
- Gas price monitor (base fee tracking)
- Sync status and peer count
- Pause/resume with `Space`

### The "Poke" Action
- **Balance Snapshot** (`p`): View ETH + ERC20 balances instantly
- **Storage Inspector** (`o`): Read contract storage slots
- **Address Watching** (`w`): Get alerts when watched addresses transact

### Transaction Debugger
- Deep call trace visualization (7+ levels)
- Collapsible trace tree with gas breakdown
- Revert reason display
- Auto ABI decoding via:
  - Local ABI files (`out/`, `artifacts/`)
  - 4byte.directory / OpenChain API

### Smart Connection
- Auto-detects node type: Anvil, Geth, Reth
- Supports HTTP, WebSocket, and IPC
- Auto-reconnect on failure
- Multi-endpoint switching

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/daoleno/poke.git
cd poke

# Build and install
cargo install --path .
```

### Requirements

- Rust 1.70+
- An Ethereum node (local or remote)

## Quick Start

```bash
# Connect to local Anvil/Geth (default: localhost:8545)
poke

# Connect to specific HTTP endpoint
poke --rpc http://localhost:8545

# Connect via WebSocket
poke --ws ws://localhost:8546

# Connect via IPC (Unix only)
poke --ipc ~/.ethereum/geth.ipc
```

## Keyboard Shortcuts

### Navigation

| Key | Action |
|-----|--------|
| `Tab` / `h` / `l` | Switch focus between panels |
| `j` / `k` | Move up/down in lists |
| `Enter` | Enter detail view |
| `Esc` | Go back |
| `gg` / `G` | Jump to top/bottom |
| `Ctrl-u` / `Ctrl-d` | Half page up/down |
| `1`-`5` | Jump to section |

### Actions

| Key | Action |
|-----|--------|
| `p` | Poke address (view balances) |
| `o` | Inspect storage slot |
| `t` | Open transaction trace |
| `e` | Expand/collapse trace node |
| `w` | Watch/unwatch address |
| `n` | Set label for address |

### Global

| Key | Action |
|-----|--------|
| `/` | Search (address, tx hash, block number) |
| `Space` | Pause/resume live updates |
| `r` | Refresh current view |
| `s` | Open settings |
| `?` | Show help |
| `q` | Quit |

### Filtering

In the search bar (`/`), use filters:
```
from:0x123...    # Filter by sender
to:0x456...      # Filter by recipient
method:transfer  # Filter by method name
value:>1         # Filter by value
```

## Configuration

Poke stores configuration at:
- Linux/macOS: `~/.config/poke/config.toml`
- Or set `POKE_CONFIG` environment variable

### Example Config

```toml
# RPC endpoints
[[endpoints]]
name = "Local Anvil"
url = "http://localhost:8545"

[[endpoints]]
name = "Mainnet"
url = "https://eth.llamarpc.com"

# Token list for balance snapshots
[[tokens]]
address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
symbol = "USDC"
decimals = 6

[[tokens]]
address = "0xdAC17F958D2ee523a2206206994597C13D831ec7"
symbol = "USDT"
decimals = 6

# ABI scan paths (defaults to ./out and ./artifacts)
[abi]
paths = ["./out", "./artifacts", "./abi"]
```

## Data Storage

Poke stores local data at:
- Linux: `~/.local/share/poke/`
- macOS: `~/Library/Application Support/poke/`

This includes:
- `labels.sqlite3`: Address labels you've created

## Development

### Running with Anvil

```bash
# Terminal 1: Start Anvil
anvil

# Terminal 2: Run Poke
cargo run -- --rpc http://localhost:8545
```

### Development Environment

We provide a script that sets up a rich test environment with DeFi contracts:

```bash
# Start the dev environment (deploys tokens, pools, generates txs)
./scripts/dev-env.sh

# In another terminal
cargo run -- --rpc http://localhost:8545
```

This creates:
- 3 ERC20 tokens
- 2 Liquidity pools (AMM)
- Router with multi-hop swaps
- Staking vault
- Deep call traces (5-7+ levels)

### Running Tests

```bash
cargo test
```

## Roadmap

- [x] Live block/tx dashboard
- [x] Call trace visualization
- [x] Local ABI scanning & decoding
- [x] Auto 4byte signature resolution
- [x] Address watching & labels
- [x] Storage slot inspection
- [ ] Multi-node management UI
- [ ] WebSocket subscriptions
- [ ] Log event decoding
- [ ] Foundry test integration

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

Built with:
- [Ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI framework
- [Alloy](https://github.com/alloy-rs/alloy) - Ethereum library
- [OpenChain](https://openchain.xyz/) - 4byte signature database

---

**Poke** — Stop context switching. Start poking.
