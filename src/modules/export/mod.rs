//! Export Module
//!
//! Provides CSV and JSON export functionality for Explorer lists and traces.
//!
//! - 'e' key triggers export based on current view/section
//! - Lists (Blocks, Transactions, Addresses) → CSV
//! - Traces → JSON
//! - Files saved to ~/.poke/exports/

mod csv_export;
mod json_export;

use crate::app::{App, Section, View};
use crate::core::{Action, NotifyLevel};
use chrono::Local;
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

/// Get the export directory path, creating it if needed
fn get_export_dir() -> std::io::Result<PathBuf> {
    let export_dir = ProjectDirs::from("io", "poke", "poke")
        .map(|dirs| dirs.data_dir().join("exports"))
        .unwrap_or_else(|| PathBuf::from(".poke").join("exports"));
    fs::create_dir_all(&export_dir)?;
    Ok(export_dir)
}

/// Generate a timestamped filename
fn generate_filename(prefix: &str, extension: &str) -> String {
    let timestamp = Local::now().format("%Y-%m-%d-%H%M%S");
    format!("{}-{}.{}", prefix, timestamp, extension)
}

/// Export current view data based on context
///
/// Routes to appropriate exporter based on:
/// - View::Trace → JSON export
/// - Section::Blocks → CSV blocks
/// - Section::Transactions → CSV transactions
/// - Section::Addresses → CSV addresses
pub fn export_current_view(app: &App) -> Action {
    let current_view = app.current_view();

    // Handle trace export
    if current_view == View::Trace {
        return export_trace(app);
    }

    // Handle list exports based on section
    match app.active_section {
        Section::Blocks => export_blocks(app),
        Section::Transactions => export_transactions(app),
        Section::Addresses => export_addresses(app),
        _ => Action::Notify(
            "Nothing to export in this view".to_string(),
            NotifyLevel::Warn,
        ),
    }
}

fn export_blocks(app: &App) -> Action {
    if app.blocks.is_empty() {
        return Action::Notify("No blocks to export".to_string(), NotifyLevel::Warn);
    }

    let export_dir = match get_export_dir() {
        Ok(dir) => dir,
        Err(e) => {
            return Action::Notify(
                format!("Failed to create export directory: {}", e),
                NotifyLevel::Error,
            )
        }
    };

    let filename = generate_filename("blocks", "csv");
    let path = export_dir.join(&filename);

    match csv_export::write_blocks(&path, &app.blocks) {
        Ok(count) => Action::Notify(
            format!("Exported {} blocks to ~/.poke/exports/{}", count, filename),
            NotifyLevel::Info,
        ),
        Err(e) => Action::Notify(format!("Export failed: {}", e), NotifyLevel::Error),
    }
}

fn export_transactions(app: &App) -> Action {
    if app.txs.is_empty() {
        return Action::Notify("No transactions to export".to_string(), NotifyLevel::Warn);
    }

    let export_dir = match get_export_dir() {
        Ok(dir) => dir,
        Err(e) => {
            return Action::Notify(
                format!("Failed to create export directory: {}", e),
                NotifyLevel::Error,
            )
        }
    };

    let filename = generate_filename("transactions", "csv");
    let path = export_dir.join(&filename);

    match csv_export::write_transactions(&path, &app.txs) {
        Ok(count) => Action::Notify(
            format!("Exported {} txs to ~/.poke/exports/{}", count, filename),
            NotifyLevel::Info,
        ),
        Err(e) => Action::Notify(format!("Export failed: {}", e), NotifyLevel::Error),
    }
}

fn export_addresses(app: &App) -> Action {
    if app.addresses.is_empty() {
        return Action::Notify("No addresses to export".to_string(), NotifyLevel::Warn);
    }

    let export_dir = match get_export_dir() {
        Ok(dir) => dir,
        Err(e) => {
            return Action::Notify(
                format!("Failed to create export directory: {}", e),
                NotifyLevel::Error,
            )
        }
    };

    let filename = generate_filename("addresses", "csv");
    let path = export_dir.join(&filename);

    match csv_export::write_addresses(&path, &app.addresses) {
        Ok(count) => Action::Notify(
            format!("Exported {} addresses to ~/.poke/exports/{}", count, filename),
            NotifyLevel::Info,
        ),
        Err(e) => Action::Notify(format!("Export failed: {}", e), NotifyLevel::Error),
    }
}

fn export_trace(app: &App) -> Action {
    if app.traces.is_empty() {
        return Action::Notify("No trace data to export".to_string(), NotifyLevel::Warn);
    }

    let export_dir = match get_export_dir() {
        Ok(dir) => dir,
        Err(e) => {
            return Action::Notify(
                format!("Failed to create export directory: {}", e),
                NotifyLevel::Error,
            )
        }
    };

    let filename = generate_filename("trace", "json");
    let path = export_dir.join(&filename);

    match json_export::write_trace(&path, &app.traces) {
        Ok(count) => Action::Notify(
            format!("Exported {} frames to ~/.poke/exports/{}", count, filename),
            NotifyLevel::Info,
        ),
        Err(e) => Action::Notify(format!("Export failed: {}", e), NotifyLevel::Error),
    }
}
