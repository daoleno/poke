//! Alert framework for ops monitoring

use super::{OpsResult, OpsStatus};
use crate::core::Action;

/// Alert rule definition
#[derive(Clone, Debug)]
pub struct AlertRule {
    pub name: String,
    pub condition: AlertCondition,
    pub message: String,
}

/// Alert condition types
#[derive(Clone, Debug)]
pub enum AlertCondition {
    PeerCountBelow(u32),
    RpcLatencyAbove(u64),
    SyncStalled,
}

/// Alert checker
pub struct AlertChecker {
    rules: Vec<AlertRule>,
}

impl AlertChecker {
    pub fn new() -> Self {
        Self {
            rules: Self::default_rules(),
        }
    }

    fn default_rules() -> Vec<AlertRule> {
        vec![
            AlertRule {
                name: "peer_count_low".to_string(),
                condition: AlertCondition::PeerCountBelow(3),
                message: "Peer count critically low".to_string(),
            },
            AlertRule {
                name: "rpc_latency_high".to_string(),
                condition: AlertCondition::RpcLatencyAbove(500),
                message: "RPC latency spike detected".to_string(),
            },
        ]
    }

    pub fn check_alerts(
        &self,
        peer_count: u32,
        rpc_latency_ms: Option<u64>,
        syncing: bool,
    ) -> Vec<String> {
        let mut alerts = Vec::new();

        for rule in &self.rules {
            match rule.condition {
                AlertCondition::PeerCountBelow(threshold) => {
                    if peer_count < threshold {
                        alerts.push(format!("⚠ {}: {}", rule.name, rule.message));
                    }
                }
                AlertCondition::RpcLatencyAbove(threshold) => {
                    if let Some(latency) = rpc_latency_ms {
                        if latency > threshold {
                            alerts.push(format!("⚠ {}: {}", rule.name, rule.message));
                        }
                    }
                }
                AlertCondition::SyncStalled => {
                    // Would need additional state to detect stall
                    // For now, just check if syncing
                    if syncing {
                        // Placeholder logic
                    }
                }
            }
        }

        alerts
    }
}

/// Display active alerts
pub fn alerts(peer_count: u32, rpc_latency_ms: Option<u64>, syncing: bool) -> Action {
    let checker = AlertChecker::new();
    let active_alerts = checker.check_alerts(peer_count, rpc_latency_ms, syncing);

    let mut result = OpsResult::new("Alerts");

    if active_alerts.is_empty() {
        result = result.add("status", "All clear", OpsStatus::Ok);
    } else {
        result = result.add("count", active_alerts.len().to_string(), OpsStatus::Warning);
        for (i, alert) in active_alerts.iter().take(3).enumerate() {
            result = result.add(format!("alert{}", i + 1), alert, OpsStatus::Warning);
        }
    }

    result.into_action()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_checker() {
        let checker = AlertChecker::new();
        let alerts = checker.check_alerts(2, Some(600), false);
        assert_eq!(alerts.len(), 2); // Both peer count and latency alerts
    }

    #[test]
    fn test_no_alerts() {
        let checker = AlertChecker::new();
        let alerts = checker.check_alerts(10, Some(50), false);
        assert_eq!(alerts.len(), 0);
    }
}
