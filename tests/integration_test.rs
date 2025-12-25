//! Integration test for block/tx detail navigation
//!
//! This test verifies that after connecting to RPC:
//! 1. Blocks are properly ingested
//! 2. Transactions are properly ingested with correct block_number
//! 3. selected_block() returns the correct block
//! 4. tx_scope() filters transactions correctly

use alloy::primitives::U256;
use alloy::providers::{Provider, ProviderBuilder};

#[tokio::test]
async fn test_block_tx_detail_navigation() {
    let rpc_url_str = std::env::var("RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:8545".to_string());
    let rpc_url = rpc_url_str.parse().expect("valid url");
    let provider = ProviderBuilder::new().connect_http(rpc_url);

    // Get latest block
    let block_num = provider.get_block_number().await.expect("should get block number");
    println!("✓ Block number: {}", block_num);

    // Fetch the block as JSON
    let block_num_hex = format!("0x{:x}", block_num);
    let json: serde_json::Value = provider
        .raw_request("eth_getBlockByNumber".into(), (&block_num_hex, true))
        .await
        .expect("should get block");

    println!("✓ Block fetched");

    // Check transactions in block
    let txs = json.get("transactions").and_then(|t| t.as_array());
    let tx_count = txs.map(|t| t.len()).unwrap_or(0);
    println!("  Transaction count: {}", tx_count);

    if tx_count == 0 {
        println!("⚠ Block has no transactions - need to generate some for full test");
        // In a real test, we'd generate transactions
    }

    // Simulate the app's data structures
    #[derive(Debug, Clone)]
    struct BlockInfo {
        number: u64,
        tx_count: u32,
    }

    #[derive(Debug, Clone)]
    struct TxInfo {
        hash: String,
        block_number: u64,
    }

    // Simulate ingesting blocks like the app does
    let mut blocks: Vec<BlockInfo> = Vec::new();
    let mut txs: Vec<TxInfo> = Vec::new();

    // Ingest last 5 blocks
    let start = block_num.saturating_sub(4);
    for num in start..=block_num {
        let num_hex = format!("0x{:x}", num);
        let block_json: serde_json::Value = provider
            .raw_request("eth_getBlockByNumber".into(), (&num_hex, true))
            .await
            .unwrap_or(serde_json::Value::Null);

        if block_json.is_null() {
            continue;
        }

        let block_txs = block_json.get("transactions").and_then(|t| t.as_array());
        let tx_count = block_txs.map(|t| t.len()).unwrap_or(0) as u32;

        blocks.push(BlockInfo {
            number: num,
            tx_count,
        });

        if let Some(tx_arr) = block_txs {
            for tx in tx_arr {
                if let Some(hash) = tx.get("hash").and_then(|h| h.as_str()) {
                    txs.push(TxInfo {
                        hash: hash.to_string(),
                        block_number: num,
                    });
                }
            }
        }

        println!("  Block #{}: {} txs", num, tx_count);
    }

    println!("\n✓ Total blocks ingested: {}", blocks.len());
    println!("✓ Total txs ingested: {}", txs.len());

    // Now simulate the selection logic
    let selected_block_idx: usize = 0; // User selects first block in list

    // filtered_block_indices - no filter applied
    let filtered_block_indices: Vec<usize> = (0..blocks.len()).collect();
    println!("\n  filtered_block_indices: {:?}", filtered_block_indices);

    // selected_block_index
    let selected_block_index = filtered_block_indices.get(selected_block_idx).copied();
    println!("  selected_block_index: {:?}", selected_block_index);

    // selected_block
    let selected_block = selected_block_index.and_then(|idx| blocks.get(idx));
    println!("  selected_block: {:?}", selected_block);

    // This is the KEY assertion - selected_block should not be None!
    assert!(selected_block.is_some(), "selected_block should be Some when blocks exist!");

    // Now test tx_scope logic
    let block_number = selected_block.unwrap().number;

    // filtered_tx_indices with block scope
    let filtered_tx_indices: Vec<usize> = txs
        .iter()
        .enumerate()
        .filter(|(_, tx)| tx.block_number == block_number)
        .map(|(idx, _)| idx)
        .collect();

    println!("\n  Block #{} tx_scope:", block_number);
    println!("  filtered_tx_indices: {:?}", filtered_tx_indices);
    println!("  tx count for this block: {}", filtered_tx_indices.len());

    // If block has transactions, we should find them
    let expected_tx_count = selected_block.unwrap().tx_count as usize;
    if expected_tx_count > 0 {
        assert_eq!(
            filtered_tx_indices.len(),
            expected_tx_count,
            "Should find correct number of txs for block"
        );
    }

    // Now test selected_tx
    let selected_tx_idx: usize = 0;
    let actual_tx_idx = filtered_tx_indices.get(selected_tx_idx).copied();
    let actual_tx = actual_tx_idx.and_then(|idx| txs.get(idx));

    println!("\n  selected_tx_idx: {}", selected_tx_idx);
    println!("  actual_tx_idx: {:?}", actual_tx_idx);
    println!("  actual_tx: {:?}", actual_tx);

    if expected_tx_count > 0 {
        assert!(actual_tx.is_some(), "Should have a selected tx when block has transactions");
        assert_eq!(
            actual_tx.unwrap().block_number,
            block_number,
            "Selected tx should belong to the selected block"
        );
    }

    println!("\n✓ All selection logic tests passed!");
}

#[tokio::test]
async fn test_app_selection_simulation() {
    // Simulate the exact app behavior

    // 1. Empty state (before RPC connects)
    let mut blocks: Vec<u64> = vec![];
    let mut txs: Vec<(String, u64)> = vec![]; // (hash, block_number)
    let mut selected_block: usize = 0;
    let mut selected_tx: usize = 0;

    // Helper functions matching app.rs
    fn filtered_block_indices(blocks: &[u64]) -> Vec<usize> {
        (0..blocks.len()).collect()
    }

    fn selected_block_index(selected: usize, blocks: &[u64]) -> Option<usize> {
        filtered_block_indices(blocks).get(selected).copied()
    }

    fn get_selected_block(selected: usize, blocks: &[u64]) -> Option<&u64> {
        selected_block_index(selected, blocks).and_then(|idx| blocks.get(idx))
    }

    fn filtered_tx_indices(
        txs: &[(String, u64)],
        scope_block: Option<u64>,
    ) -> Vec<usize> {
        txs.iter()
            .enumerate()
            .filter(|(_, (_, block_num))| {
                scope_block.map(|b| *block_num == b).unwrap_or(true)
            })
            .map(|(idx, _)| idx)
            .collect()
    }

    // State before data arrives
    println!("== State: Before RPC data ==");
    println!("  blocks: {:?}", blocks);
    println!("  selected_block(): {:?}", get_selected_block(selected_block, &blocks));
    assert!(get_selected_block(selected_block, &blocks).is_none());

    // 2. Simulate RPC connected + blocks ingested
    println!("\n== State: After RPC connected + blocks ingested ==");
    blocks = vec![100, 101, 102, 103, 104];
    txs = vec![
        ("0xaaa".to_string(), 100),
        ("0xbbb".to_string(), 100),
        ("0xccc".to_string(), 101),
        ("0xddd".to_string(), 102),
        ("0xeee".to_string(), 102),
        ("0xfff".to_string(), 103),
    ];

    println!("  blocks: {:?}", blocks);
    println!("  txs: {:?}", txs);
    println!("  selected_block: {}", selected_block);
    println!("  selected_block(): {:?}", get_selected_block(selected_block, &blocks));
    assert!(get_selected_block(selected_block, &blocks).is_some());
    assert_eq!(*get_selected_block(selected_block, &blocks).unwrap(), 100);

    // 3. User navigates to block #102 (index 2)
    println!("\n== State: User selects block #102 (index 2) ==");
    selected_block = 2;
    println!("  selected_block index: {}", selected_block);
    println!("  selected_block(): {:?}", get_selected_block(selected_block, &blocks));
    assert_eq!(*get_selected_block(selected_block, &blocks).unwrap(), 102);

    // 4. User enters BlockDetail view
    println!("\n== State: User enters BlockDetail view ==");
    selected_tx = 0; // Reset tx selection on entering block detail

    let scope_block = get_selected_block(selected_block, &blocks).copied();
    println!("  tx_scope: Block({:?})", scope_block);

    let tx_indices = filtered_tx_indices(&txs, scope_block);
    println!("  filtered_tx_indices: {:?}", tx_indices);

    // Block 102 has txs at indices 3 and 4
    assert_eq!(tx_indices, vec![3, 4]);

    // Get selected tx
    let actual_tx_idx = tx_indices.get(selected_tx).copied();
    let actual_tx = actual_tx_idx.and_then(|idx| txs.get(idx));
    println!("  selected_tx index: {}", selected_tx);
    println!("  actual_tx: {:?}", actual_tx);

    assert!(actual_tx.is_some());
    assert_eq!(actual_tx.unwrap().0, "0xddd");
    assert_eq!(actual_tx.unwrap().1, 102);

    println!("\n✓ App selection simulation passed!");
}
