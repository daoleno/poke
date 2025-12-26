//! Peer information display

use super::{OpsResult, OpsStatus};
use crate::core::Action;

/// Peer info from RPC
#[derive(Clone, Debug)]
pub struct PeerInfo {
    pub id: String,
    pub name: String,
    pub remote_addr: String,
    pub local_addr: String,
    pub caps: Vec<String>,
}

/// Display peer count and summary
pub fn peers(peer_count: u32, peers: &[PeerInfo]) -> Action {
    let status = if peer_count == 0 {
        OpsStatus::Error
    } else if peer_count < 3 {
        OpsStatus::Warning
    } else {
        OpsStatus::Ok
    };

    let mut result = OpsResult::new("Peers")
        .add("connected", peer_count.to_string(), status);

    // Add first few peers
    for (i, peer) in peers.iter().take(3).enumerate() {
        let name = if peer.name.len() > 20 {
            format!("{}...", &peer.name[..17])
        } else {
            peer.name.clone()
        };
        result = result.add(
            format!("peer{}", i + 1),
            format!("{} ({})", name, peer.remote_addr),
            OpsStatus::Ok,
        );
    }

    if peers.len() > 3 {
        result = result.add("more", format!("+{}", peers.len() - 3), OpsStatus::Ok);
    }

    result.into_action()
}

/// Simple peers display with just count
pub fn peers_count(count: u32) -> Action {
    let status = if count == 0 {
        OpsStatus::Error
    } else if count < 3 {
        OpsStatus::Warning
    } else {
        OpsStatus::Ok
    };

    OpsResult::new("Peers")
        .add("connected", count.to_string(), status)
        .into_action()
}
