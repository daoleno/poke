//! Metrics tracking and visualization

use super::{OpsResult, OpsStatus};
use crate::core::Action;

/// Time-series metrics collector
#[derive(Clone, Debug, Default)]
pub struct MetricsCollector {
    pub rpc_latency: Vec<u64>,
    pub block_times: Vec<u64>,
    pub peer_counts: Vec<u32>,
    pub gas_prices: Vec<u64>,
    max_len: usize,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            rpc_latency: Vec::new(),
            block_times: Vec::new(),
            peer_counts: Vec::new(),
            gas_prices: Vec::new(),
            max_len: 100,
        }
    }

    pub fn add_rpc_latency(&mut self, latency_ms: u64) {
        self.rpc_latency.push(latency_ms);
        if self.rpc_latency.len() > self.max_len {
            self.rpc_latency.remove(0);
        }
    }

    pub fn add_block_time(&mut self, time_s: u64) {
        self.block_times.push(time_s);
        if self.block_times.len() > self.max_len {
            self.block_times.remove(0);
        }
    }

    pub fn add_peer_count(&mut self, count: u32) {
        self.peer_counts.push(count);
        if self.peer_counts.len() > self.max_len {
            self.peer_counts.remove(0);
        }
    }

    pub fn add_gas_price(&mut self, price_gwei: u64) {
        self.gas_prices.push(price_gwei);
        if self.gas_prices.len() > self.max_len {
            self.gas_prices.remove(0);
        }
    }
}

/// Display metrics with sparklines
pub fn metrics(collector: &MetricsCollector) -> Action {
    let mut result = OpsResult::new("Metrics");

    // RPC Latency
    if !collector.rpc_latency.is_empty() {
        let avg = collector.rpc_latency.iter().sum::<u64>() / collector.rpc_latency.len() as u64;
        let sparkline = crate::ui::widgets::sparkline::sparkline_text(&collector.rpc_latency, 20);
        result = result.add(
            "rpc_latency",
            format!("{} avg {}ms", sparkline, avg),
            if avg < 100 { OpsStatus::Ok } else { OpsStatus::Warning },
        );
    }

    // Block Times
    if !collector.block_times.is_empty() {
        let avg = collector.block_times.iter().sum::<u64>() / collector.block_times.len() as u64;
        let sparkline = crate::ui::widgets::sparkline::sparkline_text(&collector.block_times, 20);
        result = result.add("block_time", format!("{} avg {}s", sparkline, avg), OpsStatus::Ok);
    }

    // Peer Counts
    if !collector.peer_counts.is_empty() {
        let avg = collector.peer_counts.iter().sum::<u32>() / collector.peer_counts.len() as u32;
        let peer_u64: Vec<u64> = collector.peer_counts.iter().map(|&x| x as u64).collect();
        let sparkline = crate::ui::widgets::sparkline::sparkline_text(&peer_u64, 20);
        result = result.add("peers", format!("{} avg {}", sparkline, avg), OpsStatus::Ok);
    }

    // Gas Prices
    if !collector.gas_prices.is_empty() {
        let avg = collector.gas_prices.iter().sum::<u64>() / collector.gas_prices.len() as u64;
        let sparkline = crate::ui::widgets::sparkline::sparkline_text(&collector.gas_prices, 20);
        result = result.add("gas_price", format!("{} avg {}gwei", sparkline, avg), OpsStatus::Ok);
    }

    result.into_action()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector() {
        let mut collector = MetricsCollector::new();
        collector.add_rpc_latency(50);
        collector.add_rpc_latency(60);
        collector.add_rpc_latency(55);

        assert_eq!(collector.rpc_latency.len(), 3);
        let avg = collector.rpc_latency.iter().sum::<u64>() / 3;
        assert_eq!(avg, 55);
    }

    #[test]
    fn test_max_len() {
        let mut collector = MetricsCollector::new();
        for i in 0..150 {
            collector.add_peer_count(i);
        }
        assert_eq!(collector.peer_counts.len(), 100);
    }
}
