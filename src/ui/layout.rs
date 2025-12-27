#![allow(dead_code)]
use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Debug, Clone, Copy)]
pub struct UiAreas {
    pub size: Rect,
    pub header: Rect,
    pub main: Rect,
    pub footer: Rect,
    pub sidebar: Rect,
    pub sidebar_sections: Rect,
    pub sidebar_watch: Rect,
    pub list: Rect,
    pub details: Rect,
    pub status_line: Rect,
    pub command_line: Rect,
}

pub fn areas(size: Rect) -> UiAreas {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(size);

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(22),
            Constraint::Percentage(39),
            Constraint::Percentage(39),
        ])
        .split(vertical[1]);

    let sidebar_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(9)])
        .split(main_chunks[0]);

    let footer_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(vertical[2]);

    UiAreas {
        size,
        header: vertical[0],
        main: vertical[1],
        footer: vertical[2],
        sidebar: main_chunks[0],
        sidebar_sections: sidebar_chunks[0],
        sidebar_watch: sidebar_chunks[1],
        list: main_chunks[1],
        details: main_chunks[2],
        status_line: footer_chunks[0],
        command_line: footer_chunks[1],
    }
}
