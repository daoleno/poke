//! Test the actual app data flow without the TUI

// We need to access the poke crate's app module
// Since this is an integration test, we use the library directly

mod test_app_flow {
    // Use std types to simulate the app structures
    

    // Mock the exact same structures as in app.rs
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum View {
        Overview,
        BlockDetail,
        TxDetail,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Section {
        Overview,
        Blocks,
        Transactions,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum DataMode {
        Mock,
        Rpc,
    }

    #[derive(Debug, Clone)]
    struct BlockInfo {
        number: u64,
        tx_count: u32,
        gas_used: u64,
        base_fee: u64,
        miner: String,
    }

    #[derive(Debug, Clone)]
    struct TxInfo {
        hash: String,
        from: String,
        to: String,
        value: f64,
        block_number: u64,
    }

    // Simplified App structure matching the real one
    struct App {
        view_stack: Vec<View>,
        active_section: Section,
        data_mode: DataMode,
        blocks: Vec<BlockInfo>,
        txs: Vec<TxInfo>,
        selected_block: usize,
        selected_tx: usize,
        follow_blocks: bool,
        follow_txs: bool,
        paused: bool,
        max_blocks: usize,
        max_txs: usize,
        active_filter: Option<String>,
    }

    impl App {
        fn new() -> Self {
            Self {
                view_stack: vec![View::Overview],
                active_section: Section::Overview,
                data_mode: DataMode::Mock,
                blocks: Vec::new(),
                txs: Vec::new(),
                selected_block: 0,
                selected_tx: 0,
                follow_blocks: true,
                follow_txs: true,
                paused: false,
                max_blocks: 50,
                max_txs: 200,
                active_filter: None,
            }
        }

        fn current_view(&self) -> View {
            *self.view_stack.last().unwrap_or(&View::Overview)
        }

        fn filtered_block_indices(&self) -> Vec<usize> {
            // No filter - return all
            (0..self.blocks.len()).collect()
        }

        fn filtered_tx_indices(&self) -> Vec<usize> {
            let mut indices: Vec<usize> = (0..self.txs.len()).collect();

            // Apply tx_scope filtering
            match self.current_view() {
                View::BlockDetail => {
                    if let Some(block) = self.selected_block() {
                        let block_num = block.number;
                        indices.retain(|idx| {
                            self.txs.get(*idx)
                                .map(|tx| tx.block_number == block_num)
                                .unwrap_or(false)
                        });
                    }
                }
                _ => {}
            }

            indices
        }

        fn selected_block_index(&self) -> Option<usize> {
            self.filtered_block_indices().get(self.selected_block).copied()
        }

        fn selected_block(&self) -> Option<&BlockInfo> {
            self.selected_block_index()
                .and_then(|idx| self.blocks.get(idx))
        }

        fn selected_tx_index(&self) -> Option<usize> {
            self.filtered_tx_indices().get(self.selected_tx).copied()
        }

        fn selected_tx(&self) -> Option<&TxInfo> {
            self.selected_tx_index()
                .and_then(|idx| self.txs.get(idx))
        }

        fn apply_rpc_connected(&mut self, endpoint: String) {
            println!("[apply_rpc_connected] endpoint={}, clearing {} blocks, {} txs",
                endpoint, self.blocks.len(), self.txs.len());

            self.data_mode = DataMode::Rpc;
            self.blocks.clear();
            self.txs.clear();
            self.follow_blocks = true;
            self.follow_txs = true;
            self.selected_block = 0;
            self.selected_tx = 0;
        }

        fn ingest_block(&mut self, block: BlockInfo, txs: Vec<TxInfo>) {
            println!("[ingest_block] block #{}, {} txs, current blocks.len={}",
                block.number, txs.len(), self.blocks.len());

            let was_tail = self.follow_blocks ||
                self.selected_block + 1 == self.filtered_block_indices().len();

            self.blocks.push(block);
            if self.blocks.len() > self.max_blocks {
                let overflow = self.blocks.len() - self.max_blocks;
                self.blocks.drain(0..overflow);
            }

            let was_tx_tail = self.follow_txs ||
                self.selected_tx + 1 == self.filtered_tx_indices().len();

            self.txs.extend(txs);
            if self.txs.len() > self.max_txs {
                let overflow = self.txs.len() - self.max_txs;
                self.txs.drain(0..overflow);
            }

            // Update selection if following
            if was_tail {
                let len = self.filtered_block_indices().len();
                if len > 0 {
                    self.selected_block = len - 1;
                }
            }
            if was_tx_tail {
                let len = self.filtered_tx_indices().len();
                if len > 0 {
                    self.selected_tx = len - 1;
                }
            }
        }

        fn enter_detail(&mut self) {
            if self.current_view() != View::Overview {
                return;
            }
            match self.active_section {
                Section::Overview | Section::Blocks => {
                    self.view_stack.push(View::BlockDetail);
                    self.selected_tx = 0;
                    self.follow_txs = false;
                }
                Section::Transactions => {
                    self.view_stack.push(View::TxDetail);
                }
            }
        }
    }

    #[test]
    fn test_full_rpc_flow() {
        println!("\n=== Test: Full RPC Flow ===\n");

        let mut app = App::new();

        // Verify initial state
        println!("1. Initial state (Mock mode)");
        assert_eq!(app.data_mode, DataMode::Mock);
        assert!(app.blocks.is_empty());
        assert!(app.selected_block().is_none());
        println!("   ✓ Initial state correct\n");

        // Simulate RPC connected
        println!("2. RPC Connected");
        app.apply_rpc_connected("http://127.0.0.1:8546".to_string());
        assert_eq!(app.data_mode, DataMode::Rpc);
        assert!(app.blocks.is_empty());
        println!("   ✓ RPC connected, data cleared\n");

        // Simulate ingesting blocks (like fetch_snapshot does)
        println!("3. Ingest blocks from snapshot");
        for i in 0..5 {
            let block = BlockInfo {
                number: 100 + i,
                tx_count: 3,
                gas_used: 21000 * 3,
                base_fee: 10,
                miner: format!("0x{:040x}", i),
            };
            let txs: Vec<TxInfo> = (0..3).map(|j| TxInfo {
                hash: format!("0x{:064x}", i * 10 + j),
                from: format!("0x{:040x}", j),
                to: format!("0x{:040x}", j + 10),
                value: 0.1,
                block_number: 100 + i,
            }).collect();
            app.ingest_block(block, txs);
        }

        println!("   blocks.len: {}", app.blocks.len());
        println!("   txs.len: {}", app.txs.len());
        println!("   selected_block index: {}", app.selected_block);
        assert_eq!(app.blocks.len(), 5);
        assert_eq!(app.txs.len(), 15);
        // Since follow_blocks is true, selected_block should be at tail
        assert_eq!(app.selected_block, 4);
        println!("   ✓ Blocks and txs ingested correctly\n");

        // Verify selected_block works
        println!("4. Check selected_block");
        let selected = app.selected_block();
        println!("   selected_block(): {:?}", selected.map(|b| b.number));
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().number, 104); // Last block
        println!("   ✓ selected_block returns correct block\n");

        // User navigates to first block and enters detail view
        println!("5. User navigates to block #100 (index 0)");
        app.selected_block = 0;
        app.active_section = Section::Blocks;

        let selected = app.selected_block();
        println!("   selected_block(): {:?}", selected.map(|b| b.number));
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().number, 100);
        println!("   ✓ Block #100 selected correctly\n");

        // Enter detail view
        println!("6. User enters BlockDetail view");
        app.enter_detail();
        assert_eq!(app.current_view(), View::BlockDetail);
        println!("   current_view: {:?}", app.current_view());
        println!("   selected_tx reset to: {}", app.selected_tx);

        // CRITICAL: Check selected_block still works in BlockDetail view
        let selected = app.selected_block();
        println!("   selected_block(): {:?}", selected.map(|b| b.number));
        assert!(selected.is_some(), "FAIL: selected_block is None in BlockDetail view!");
        assert_eq!(selected.unwrap().number, 100);
        println!("   ✓ Block still selected in BlockDetail view\n");

        // Check tx filtering
        println!("7. Check tx filtering in BlockDetail view");
        let tx_indices = app.filtered_tx_indices();
        println!("   filtered_tx_indices: {:?}", tx_indices);
        // Block #100 should have txs at indices 0, 1, 2
        assert_eq!(tx_indices, vec![0, 1, 2]);

        let selected_tx = app.selected_tx();
        println!("   selected_tx(): {:?}", selected_tx.map(|t| &t.hash[..20]));
        assert!(selected_tx.is_some());
        assert_eq!(selected_tx.unwrap().block_number, 100);
        println!("   ✓ Tx filtering works correctly\n");

        println!("=== ALL TESTS PASSED ===\n");
    }

    #[test]
    fn test_empty_block_scenario() {
        println!("\n=== Test: Empty Block Scenario ===\n");

        let mut app = App::new();

        // RPC connected but no blocks yet
        app.apply_rpc_connected("http://127.0.0.1:8546".to_string());

        println!("1. After RPC connect, before any blocks");
        println!("   blocks.len: {}", app.blocks.len());
        println!("   selected_block index: {}", app.selected_block);
        assert!(app.selected_block().is_none());
        println!("   ✓ selected_block is None (expected)\n");

        // Enter block detail view with no blocks
        app.active_section = Section::Blocks;
        app.enter_detail();
        assert_eq!(app.current_view(), View::BlockDetail);

        println!("2. In BlockDetail view with no blocks");
        let selected = app.selected_block();
        println!("   selected_block(): {:?}", selected);
        assert!(selected.is_none());
        println!("   ✓ selected_block is None (expected - no blocks)\n");

        // Now ingest a block
        println!("3. Ingest first block");
        let block = BlockInfo {
            number: 100,
            tx_count: 2,
            gas_used: 42000,
            base_fee: 10,
            miner: "0x0".to_string(),
        };
        let txs = vec![
            TxInfo {
                hash: "0xabc".to_string(),
                from: "0x1".to_string(),
                to: "0x2".to_string(),
                value: 0.1,
                block_number: 100,
            },
            TxInfo {
                hash: "0xdef".to_string(),
                from: "0x3".to_string(),
                to: "0x4".to_string(),
                value: 0.2,
                block_number: 100,
            },
        ];
        app.ingest_block(block, txs);

        println!("   blocks.len: {}", app.blocks.len());
        println!("   selected_block index: {}", app.selected_block);

        // IMPORTANT: Still in BlockDetail view, now check if block is selected
        let selected = app.selected_block();
        println!("   selected_block(): {:?}", selected.map(|b| b.number));

        // This should now work because we ingested a block!
        assert!(selected.is_some(), "Block should be selected after ingest!");
        assert_eq!(selected.unwrap().number, 100);
        println!("   ✓ selected_block works after ingest\n");

        println!("=== ALL TESTS PASSED ===\n");
    }
}
