//! JSON Export
//!
//! Writes trace data to JSON files.

use crate::app::{CallStatus, TraceFrame};
use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Exportable trace frame (excludes UI state like 'collapsed')
#[derive(Serialize)]
struct ExportableTrace {
    depth: usize,
    call: String,
    from: String,
    to: String,
    value: f64,
    gas_used: u64,
    status: String,
    note: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selector: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    input: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    decoded_args: Option<Vec<ExportableArg>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    decode_error: Option<String>,
}

#[derive(Serialize)]
struct ExportableArg {
    name: String,
    kind: String,
    value: String,
}

impl From<&TraceFrame> for ExportableTrace {
    fn from(frame: &TraceFrame) -> Self {
        let status = match frame.status {
            CallStatus::Ok => "ok".to_string(),
            CallStatus::Revert => "revert".to_string(),
        };

        Self {
            depth: frame.depth,
            call: frame.call.clone(),
            from: frame.from.clone(),
            to: frame.to.clone(),
            value: frame.value,
            gas_used: frame.gas_used,
            status,
            note: frame.note.clone(),
            method: frame.method.clone(),
            selector: frame.selector.clone(),
            signature: frame.signature.clone(),
            input: frame.input.clone(),
            decoded_args: frame.decoded_args.as_ref().map(|args| {
                args.iter()
                    .map(|arg| ExportableArg {
                        name: arg.name.clone(),
                        kind: arg.kind.clone(),
                        value: arg.value.clone(),
                    })
                    .collect()
            }),
            decode_error: frame.decode_error.clone(),
        }
    }
}

/// Write trace frames to JSON file
pub fn write_trace(path: &Path, traces: &[TraceFrame]) -> Result<usize, Box<dyn std::error::Error>> {
    let exportable: Vec<ExportableTrace> = traces.iter().map(ExportableTrace::from).collect();

    let json = serde_json::to_string_pretty(&exportable)?;

    let mut file = File::create(path)?;
    file.write_all(json.as_bytes())?;

    Ok(traces.len())
}
