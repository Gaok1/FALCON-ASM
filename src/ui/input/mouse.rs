use crate::ui::{
    app::{App, EditorMode, MemRegion, Tab},
    editor::Editor,
};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub fn handle_mouse(app: &mut App, me: MouseEvent, area: Rect) {
    app.mouse_x = me.column;
    app.mouse_y = me.row;

    if app.show_exit_popup {
        handle_exit_popup_mouse(app, me, area);
        return;
    }

    // Hover tabs
    app.hover_tab = None;
    if me.row == area.y + 1 {
        let x = me.column.saturating_sub(area.x + 1);
        let titles = [
            ("Editor", Tab::Editor),
            ("Run", Tab::Run),
            ("Docs", Tab::Docs),
        ];
        let divider = " │ ".len() as u16;
        let mut pos: u16 = 0;
        for (i, (title, tab)) in titles.iter().enumerate() {
            let w = title.len() as u16;
            if x >= pos && x < pos + w {
                app.hover_tab = Some(*tab);
                if matches!(me.kind, MouseEventKind::Down(MouseButton::Left)) {
                    app.tab = *tab;
                    app.mode = EditorMode::Command;
                }
                break;
            }
            pos += w;
            if i + 1 < titles.len() {
                pos += divider;
            }
        }
    }

    // Scrolls
    match me.kind {
        MouseEventKind::ScrollUp => match app.tab {
            Tab::Editor => app.editor.move_up(),
            Tab::Run => {
                if app.show_registers {
                    app.regs_scroll = app.regs_scroll.saturating_sub(1);
                } else {
                    app.mem_view_addr = app.mem_view_addr.saturating_sub(app.mem_view_bytes);
                    app.mem_region = MemRegion::Custom;
                }
            }
            Tab::Docs => app.docs_scroll = app.docs_scroll.saturating_sub(1),
        },
        MouseEventKind::ScrollDown => match app.tab {
            Tab::Editor => app.editor.move_down(),
            Tab::Run => {
                if app.show_registers {
                    app.regs_scroll = app.regs_scroll.saturating_add(1);
                } else {
                    let max = app.mem_size.saturating_sub(app.mem_view_bytes as usize) as u32;
                    if app.mem_view_addr < max {
                        app.mem_view_addr = app
                            .mem_view_addr
                            .saturating_add(app.mem_view_bytes)
                            .min(max);
                    }
                    app.mem_region = MemRegion::Custom;
                }
            }
            Tab::Docs => app.docs_scroll += 1,
        },
        _ => {}
    }

    if let Tab::Editor = app.tab {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(1),
            ])
            .split(area);
        let editor_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(5), Constraint::Min(3)])
            .split(chunks[1]);
        let editor_area = editor_chunks[1];

        let start = {
            let visible_h = editor_area.height.saturating_sub(2) as usize;
            let len = app.editor.lines.len();
            let mut s = 0usize;
            if len > visible_h {
                if app.editor.cursor_row <= visible_h / 2 {
                    s = 0;
                } else if app.editor.cursor_row >= len.saturating_sub(visible_h / 2) {
                    s = len.saturating_sub(visible_h);
                } else {
                    s = app.editor.cursor_row - visible_h / 2;
                }
            }
            s
        };

        let within = |x: u16, y: u16| {
            x >= editor_area.x + 1
                && x < editor_area.x + editor_area.width - 1
                && y >= editor_area.y + 1
                && y < editor_area.y + editor_area.height - 1
        };

        match me.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if within(me.column, me.row) {
                    let y = (me.row - (editor_area.y + 1)) as usize;
                    let row = (start + y).min(app.editor.lines.len().saturating_sub(1));
                    let x = me.column.saturating_sub(editor_area.x + 1) as usize;
                    let col = x.min(Editor::char_count(&app.editor.lines[row]));
                    app.editor.cursor_row = row;
                    app.editor.cursor_col = col;
                    app.editor.selection_anchor = Some((row, col));
                    if app.mode == EditorMode::Command {
                        app.mode = EditorMode::Insert;
                    }
                } else if app.mode == EditorMode::Insert {
                    app.mode = EditorMode::Command;
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if within(me.column, me.row) {
                    let y = (me.row - (editor_area.y + 1)) as usize;
                    let row = (start + y).min(app.editor.lines.len().saturating_sub(1));
                    let x = me.column.saturating_sub(editor_area.x + 1) as usize;
                    let col = x.min(Editor::char_count(&app.editor.lines[row]));
                    app.editor.cursor_row = row;
                    app.editor.cursor_col = col;
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if let Some((r, c)) = app.editor.selection_anchor {
                    if r == app.editor.cursor_row && c == app.editor.cursor_col {
                        app.editor.clear_selection();
                    }
                }
            }
            _ => {}
        }
    }

    // Run tab interactions
    if let Tab::Run = app.tab {
        if matches!(me.kind, MouseEventKind::Down(MouseButton::Left)) {
            handle_run_status_click(app, me, area);
        }
    }
}

fn handle_run_status_click(app: &mut App, me: MouseEvent, area: Rect) {
    let root_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(area);
    let run_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Min(0),
        ])
        .split(root_chunks[1]);
    let status = run_chunks[1];
    if me.row != status.y + 1 {
        return;
    }

    let view_text = if app.show_registers { "REGS" } else { "RAM" };
    let fmt_text = if app.show_hex { "HEX" } else { "DEC" };
    let bytes_text = match app.mem_view_bytes {
        4 => "4B",
        2 => "2B",
        _ => "1B",
    };
    let region_text = match app.mem_region {
        MemRegion::Data => "DATA",
        MemRegion::Stack => "STACK",
        MemRegion::Custom => "ADDR",
    };
    let run_text = if app.is_running { "RUN" } else { "PAUSE" };

    let mut pos = status.x + 1;

    let range = |start: &mut u16, label: &str| {
        let s = *start;
        *start += 1 + label.len() as u16 + 1;
        (s, *start)
    };
    let skip = |start: &mut u16, s: &str| {
        *start += s.len() as u16;
    };

    skip(&mut pos, "View ");
    let (view_start, view_end) = range(&mut pos, view_text);

    skip(&mut pos, "  Format ");
    let (fmt_start, fmt_end) = range(&mut pos, fmt_text);

    let (bytes_start, bytes_end) = if !app.show_registers {
        skip(&mut pos, "  Bytes ");
        range(&mut pos, bytes_text)
    } else {
        (0, 0)
    };

    let (region_start, region_end) = if !app.show_registers {
        skip(&mut pos, "  Region ");
        range(&mut pos, region_text)
    } else {
        (0, 0)
    };

    skip(&mut pos, "  State ");
    let (state_start, state_end) = range(&mut pos, run_text);

    let col = me.column;
    if col >= view_start && col < view_end {
        app.show_registers = !app.show_registers;
    } else if col >= fmt_start && col < fmt_end {
        app.show_hex = !app.show_hex;
    } else if !app.show_registers && col >= bytes_start && col < bytes_end {
        app.mem_view_bytes = match app.mem_view_bytes {
            4 => 2,
            2 => 1,
            _ => 4,
        };
    } else if !app.show_registers && col >= region_start && col < region_end {
        app.mem_region = match app.mem_region {
            MemRegion::Data => {
                app.mem_view_addr = app.cpu.x[2];
                MemRegion::Stack
            }
            MemRegion::Stack => MemRegion::Custom,
            MemRegion::Custom => {
                app.mem_view_addr = app.data_base;
                MemRegion::Data
            }
        };
    } else if col >= state_start && col < state_end {
        if app.is_running {
            app.is_running = false;
        } else if !app.faulted {
            app.is_running = true;
        }
    }
}

fn handle_exit_popup_mouse(app: &mut App, me: MouseEvent, area: Rect) {
    let popup = centered_rect(area.width / 3, area.height / 4, area);
    if me.kind != MouseEventKind::Down(MouseButton::Left) {
        return;
    }
    if me.column < popup.x + 1
        || me.column >= popup.x + popup.width - 1
        || me.row < popup.y + 1
        || me.row >= popup.y + popup.height - 1
    {
        app.show_exit_popup = false;
        return;
    }
    let inner_x = me.column - (popup.x + 1);
    let inner_y = me.row - (popup.y + 1);
    const EXIT: &str = "[Exit]";
    const CANCEL: &str = "[Cancel]";
    const GAP: u16 = 3;
    if inner_y == 3 {
        let line_width = EXIT.len() as u16 + GAP + CANCEL.len() as u16;
        let start = ((popup.width - 2).saturating_sub(line_width)) / 2;
        if inner_x >= start && inner_x < start + EXIT.len() as u16 {
            app.should_quit = true;
        } else if inner_x
            >= start + EXIT.len() as u16 + GAP
            && inner_x < start + EXIT.len() as u16 + GAP + CANCEL.len() as u16
        {
            app.show_exit_popup = false;
        }
    }
}


fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    Rect::new(
        r.x + (r.width.saturating_sub(width)) / 2,
        r.y + (r.height.saturating_sub(height)) / 2,
        width,
        height,
    )
}
