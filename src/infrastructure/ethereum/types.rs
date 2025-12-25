//! Type conversions between Alloy types and bridge types

use alloy::primitives::{Address, U256};
use alloy::rpc::types::trace::geth::GethTrace;

use crate::infrastructure::runtime::{CallStatus, TraceFrame};

/// Convert Alloy trace result to bridge TraceFrames
pub fn convert_trace_frames(trace: GethTrace) -> Vec<TraceFrame> {
    let mut frames = Vec::new();

    match trace {
        GethTrace::CallTracer(call) => {
            flatten_call_frame(&call, 0, &mut frames);
        }
        _ => {
            // Other trace types not supported
        }
    }

    frames
}

/// Recursively flatten a call tracer frame into a list
fn flatten_call_frame(
    frame: &alloy::rpc::types::trace::geth::CallFrame,
    depth: usize,
    out: &mut Vec<TraceFrame>,
) {
    let call_type = frame.typ.to_string();
    let from = format!("{:?}", frame.from);
    let to = format!("{:?}", frame.to.unwrap_or(Address::ZERO));
    let value = wei_to_eth(frame.value.unwrap_or(U256::ZERO));
    let gas_used = frame.gas_used.to::<u64>();

    let (status, note) = if let Some(error) = &frame.error {
        (CallStatus::Revert, format!("revert: {}", error))
    } else {
        (CallStatus::Ok, "ok".to_string())
    };

    let input = frame.input.to_vec();
    let selector = if input.len() >= 4 {
        Some(format!("0x{}", hex::encode(&input[..4])))
    } else {
        None
    };

    out.push(TraceFrame {
        depth,
        call: call_type,
        from,
        to,
        value,
        gas_used,
        status,
        note,
        collapsed: false,
        input: Some(format!("0x{}", hex::encode(&input))),
        selector,
        method: None,
        signature: None,
        decoded_args: None,
        decode_error: None,
    });

    // Recursively process child calls
    for child in &frame.calls {
        flatten_call_frame(child, depth + 1, out);
    }
}

/// Wei to ETH as f64 (for display)
fn wei_to_eth(wei: U256) -> f64 {
    let eth_in_wei = U256::from(1_000_000_000_000_000_000u64);
    let whole = wei / eth_in_wei;
    let frac = wei % eth_in_wei;

    let whole_f64: f64 = whole.to_string().parse().unwrap_or(0.0);
    let frac_f64: f64 = frac.to_string().parse().unwrap_or(0.0);

    whole_f64 + frac_f64 / 1e18
}
