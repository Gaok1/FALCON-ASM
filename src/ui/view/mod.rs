use ratatui::{
    Frame,
    prelude::*,
    widgets::{Block, Borders, Paragraph, Tabs},
};

pub(super) use super::app::{App, EditorMode, MemRegion, Tab};
pub(super) use super::editor::Editor;

mod docs;
mod editor;
mod run;

use docs::render_docs;
use editor::{render_editor, render_editor_status};
use run::render_run;

pub fn ui(f: &mut Frame, app: &App) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(size);

    let titles = ["Editor", "Run", "Docs"]
        .into_iter()
        .enumerate()
        .map(|(i, t)| {
            let mut line = Line::from(t);
            let tab = match i {
                0 => Tab::Editor,
                1 => Tab::Run,
                _ => Tab::Docs,
            };
            if Some(tab) == app.hover_tab && tab != app.tab {
                line = line.style(Style::default().bg(Color::DarkGray));
            }
            line
        })
        .collect::<Vec<_>>();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("Falcon ASM"))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::styled(" │ ", Style::default().fg(Color::DarkGray)))
        .select(match app.tab {
            Tab::Editor => 0,
            Tab::Run => 1,
            Tab::Docs => 2,
        });
    f.render_widget(tabs, chunks[0]);

    match app.tab {
        Tab::Editor => {
            let editor_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(5), Constraint::Min(3)])
                .split(chunks[1]);
            render_editor_status(f, editor_chunks[0], app);
            render_editor(f, editor_chunks[1], app);
        }
        Tab::Run => render_run(f, chunks[1], app),
        Tab::Docs => render_docs(f, chunks[1], app),
    }

    let mode = match app.mode {
        EditorMode::Insert => "INSERT",
        EditorMode::Command => "COMMAND",
    };
    let status = format!(
        "Mode: {}  |  Ctrl+R=Assemble  |  Ctrl+O=Import  |  Ctrl+S=Export  |  1/2/3 switch tabs (Command mode)",
        mode
    );

    let status = Paragraph::new(status).block(Block::default().borders(Borders::ALL));
    f.render_widget(status, chunks[2]);
}
