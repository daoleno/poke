//! Comprehensive test for block/tx data flow

use alloy::primitives::U256;
use alloy::providers::{Provider, ProviderBuilder};

#[tokio::test]
async fn test_full_data_flow() {
    let rpc_url_str = std::env::var("RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:8545".to_string());
    let rpc_url = rpc_url_str.parse().expect("valid url");
    let provider = ProviderBuilder::new().connect_http(rpc_url);

    // 1. Get block number
    let block_num = provider.get_block_number().await.expect("should get block number");
    println!("✓ Block number: {}", block_num);

    // 2. Fetch block as raw JSON (same as our provider)
    let block_num_hex = format!("0x{:x}", block_num);
    let json: serde_json::Value = provider
        .raw_request("eth_getBlockByNumber".into(), (&block_num_hex, true))
        .await
        .expect("should get block");

    assert!(!json.is_null(), "Block should not be null");
    println!("✓ Block JSON fetched");

    // 3. Check block header fields
    let number = json.get("number").and_then(|v| v.as_str());
    let gas_used = json.get("gasUsed").and_then(|v| v.as_str());
    let miner = json.get("miner").and_then(|v| v.as_str());
    let base_fee = json.get("baseFeePerGas").and_then(|v| v.as_str());

    println!("  Block header:");
    println!("    number: {:?}", number);
    println!("    gasUsed: {:?}", gas_used);
    println!("    miner: {:?}", miner);
    println!("    baseFeePerGas: {:?}", base_fee);

    assert!(number.is_some(), "Block should have number");
    assert!(gas_used.is_some(), "Block should have gasUsed");
    assert!(miner.is_some(), "Block should have miner");

    // 4. Check transactions
    let txs = json.get("transactions").and_then(|t| t.as_array());
    assert!(txs.is_some(), "Block should have transactions array");

    let txs = txs.unwrap();
    println!("✓ Transaction count: {}", txs.len());
    assert!(txs.len() > 0, "Block should have transactions");

    // 5. Check first transaction fields
    let first_tx = &txs[0];
    println!("\n  First transaction fields:");

    let tx_hash = first_tx.get("hash").and_then(|v| v.as_str());
    let tx_from = first_tx.get("from").and_then(|v| v.as_str());
    let tx_to = first_tx.get("to").and_then(|v| v.as_str());
    let tx_value = first_tx.get("value").and_then(|v| v.as_str());
    let tx_input = first_tx.get("input").and_then(|v| v.as_str());
    let tx_gas = first_tx.get("gas").and_then(|v| v.as_str());

    println!("    hash: {:?}", tx_hash);
    println!("    from: {:?}", tx_from);
    println!("    to: {:?}", tx_to);
    println!("    value: {:?}", tx_value);
    println!("    input length: {:?}", tx_input.map(|s| s.len()));
    println!("    gas: {:?}", tx_gas);

    assert!(tx_hash.is_some(), "Tx should have hash");
    assert!(tx_from.is_some(), "Tx should have from");
    // to can be None for contract creation
    assert!(tx_value.is_some(), "Tx should have value");
    assert!(tx_input.is_some(), "Tx should have input");

    // 6. Parse hex values (same as our parse_hex_u64)
    fn parse_hex_u64(s: &str) -> Option<u64> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        u64::from_str_radix(s, 16).ok()
    }

    let parsed_number = number.and_then(parse_hex_u64);
    let parsed_gas_used = gas_used.and_then(parse_hex_u64);
    let parsed_base_fee = base_fee.and_then(parse_hex_u64);

    println!("\n  Parsed values:");
    println!("    block number: {:?}", parsed_number);
    println!("    gas used: {:?}", parsed_gas_used);
    println!("    base fee: {:?}", parsed_base_fee);

    assert!(parsed_number.is_some(), "Should parse block number");
    assert!(parsed_gas_used.is_some(), "Should parse gas used");

    // 7. Check transaction receipt (for gas_used and status)
    let tx_hash_str = tx_hash.unwrap();
    let receipt: serde_json::Value = provider
        .raw_request("eth_getTransactionReceipt".into(), (tx_hash_str,))
        .await
        .expect("should get receipt");

    if !receipt.is_null() {
        let receipt_gas_used = receipt.get("gasUsed").and_then(|v| v.as_str());
        let receipt_status = receipt.get("status").and_then(|v| v.as_str());
        println!("\n  Receipt:");
        println!("    gasUsed: {:?}", receipt_gas_used);
        println!("    status: {:?}", receipt_status);
    } else {
        println!("\n  Receipt: null (pending tx?)");
    }

    println!("\n✓ All data flow checks passed!");
}

#[tokio::test]
async fn test_value_parsing() {
    // Test our hex parsing logic
    fn parse_hex_u64(s: &str) -> Result<u64, &'static str> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        u64::from_str_radix(s, 16).map_err(|_| "parse error")
    }

    fn parse_hex_u256(s: &str) -> Result<U256, &'static str> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        if s.is_empty() || s == "0" {
            return Ok(U256::ZERO);
        }
        let padded = format!("{:0>64}", s);
        let bytes = hex::decode(&padded).map_err(|_| "hex decode error")?;
        Ok(U256::from_be_slice(&bytes))
    }

    // Test cases
    assert_eq!(parse_hex_u64("0x0").unwrap(), 0);
    assert_eq!(parse_hex_u64("0x1").unwrap(), 1);
    assert_eq!(parse_hex_u64("0x10").unwrap(), 16);
    assert_eq!(parse_hex_u64("0x2612f43").unwrap(), 39923523);

    // Base fee example: 500000 = 0x7a120
    assert_eq!(parse_hex_u64("0x7a120").unwrap(), 500000);

    // Gas used example
    assert_eq!(parse_hex_u64("0x3fde1ea").unwrap(), 66977258);

    println!("✓ Hex parsing tests passed!");
}
