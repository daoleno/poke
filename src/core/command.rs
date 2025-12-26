//! Command parser for the : command system

/// Parsed command from user input
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    // Navigation commands
    Blocks,
    Transactions,
    Address(String),
    Trace(String),

    // Toolkit commands - data processing
    Encode(Option<String>),
    Decode(Option<String>),
    Hash(Option<String>),
    Hex(Option<String>),

    // Toolkit commands - query/convert
    Selector(Option<String>),
    FourByte(Option<String>),
    Convert(Option<String>),
    Timestamp(Option<String>),

    // Toolkit commands - contract interaction
    Call(Option<String>),
    Gas(Option<String>),
    Slot(Option<String>),

    // Toolkit commands - address calculation
    Create(Option<String>),
    Create2(Option<String>),
    Checksum(Option<String>),

    // Ops commands
    Health,
    Peers,
    Logs,
    Mempool,
    RpcStats,

    // Node management
    Connect(String),
    Anvil(Vec<String>),
    Impersonate(String),
    Mine(Option<u64>),
    Snapshot,
    Revert(Option<String>),

    // Unknown command
    Unknown(String),
}

/// Parse a command string (without the leading :)
pub fn parse_command(input: &str) -> Command {
    let input = input.trim();
    let mut parts = input.splitn(2, ' ');
    let cmd = parts.next().unwrap_or("");
    let args = parts.next().map(|s| s.trim().to_string());

    match cmd.to_lowercase().as_str() {
        // Navigation
        "blocks" | "blk" => Command::Blocks,
        "transactions" | "txs" | "tx" => Command::Transactions,
        "address" | "addr" => {
            if let Some(addr) = args {
                Command::Address(addr)
            } else {
                Command::Unknown(input.to_string())
            }
        }
        "trace" => {
            if let Some(hash) = args {
                Command::Trace(hash)
            } else {
                Command::Unknown(input.to_string())
            }
        }

        // Toolkit - data processing
        "encode" | "enc" => Command::Encode(args),
        "decode" | "dec" => Command::Decode(args),
        "hash" | "keccak" | "keccak256" => Command::Hash(args),
        "hex" => Command::Hex(args),

        // Toolkit - query/convert
        "selector" | "sig" => Command::Selector(args),
        "4byte" | "fourbyte" => Command::FourByte(args),
        "convert" | "conv" => Command::Convert(args),
        "timestamp" | "time" | "ts" => Command::Timestamp(args),

        // Toolkit - contract
        "call" => Command::Call(args),
        "gas" => Command::Gas(args),
        "slot" => Command::Slot(args),

        // Toolkit - address
        "create" => Command::Create(args),
        "create2" => Command::Create2(args),
        "checksum" | "check" => Command::Checksum(args),

        // Ops
        "health" => Command::Health,
        "peers" => Command::Peers,
        "logs" | "log" => Command::Logs,
        "mempool" | "pool" => Command::Mempool,
        "rpc-stats" | "rpcstats" | "stats" => Command::RpcStats,

        // Node management
        "connect" | "conn" => {
            if let Some(url) = args {
                Command::Connect(url)
            } else {
                Command::Unknown(input.to_string())
            }
        }
        "anvil" => {
            let anvil_args = args
                .map(|s| s.split_whitespace().map(|s| s.to_string()).collect())
                .unwrap_or_default();
            Command::Anvil(anvil_args)
        }
        "impersonate" | "imp" => {
            if let Some(addr) = args {
                Command::Impersonate(addr)
            } else {
                Command::Unknown(input.to_string())
            }
        }
        "mine" => {
            let n = args.and_then(|s| s.parse().ok());
            Command::Mine(n)
        }
        "snapshot" | "snap" => Command::Snapshot,
        "revert" => Command::Revert(args),

        _ => Command::Unknown(input.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_navigation_commands() {
        assert_eq!(parse_command("blocks"), Command::Blocks);
        assert_eq!(parse_command("blk"), Command::Blocks);
        assert_eq!(parse_command("txs"), Command::Transactions);
        assert_eq!(
            parse_command("address 0x1234"),
            Command::Address("0x1234".to_string())
        );
    }

    #[test]
    fn test_parse_toolkit_commands() {
        assert_eq!(parse_command("encode"), Command::Encode(None));
        assert_eq!(
            parse_command("encode transfer(address,uint256)"),
            Command::Encode(Some("transfer(address,uint256)".to_string()))
        );
        assert_eq!(
            parse_command("decode 0xabcd"),
            Command::Decode(Some("0xabcd".to_string()))
        );
        assert_eq!(
            parse_command("convert 1.5 ether"),
            Command::Convert(Some("1.5 ether".to_string()))
        );
    }

    #[test]
    fn test_parse_ops_commands() {
        assert_eq!(parse_command("health"), Command::Health);
        assert_eq!(parse_command("peers"), Command::Peers);
        assert_eq!(parse_command("logs"), Command::Logs);
    }

    #[test]
    fn test_parse_anvil_commands() {
        assert_eq!(parse_command("anvil"), Command::Anvil(vec![]));
        assert_eq!(
            parse_command("anvil --fork mainnet"),
            Command::Anvil(vec!["--fork".to_string(), "mainnet".to_string()])
        );
        assert_eq!(parse_command("mine"), Command::Mine(None));
        assert_eq!(parse_command("mine 10"), Command::Mine(Some(10)));
    }

    #[test]
    fn test_parse_unknown() {
        assert_eq!(
            parse_command("notacommand"),
            Command::Unknown("notacommand".to_string())
        );
    }
}
