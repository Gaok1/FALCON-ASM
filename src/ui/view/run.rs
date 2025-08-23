use crate::falcon::{self, memory::Bus};
use ratatui::Frame;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, Wrap};

use super::{App, MemRegion, RunButton};

pub(super) fn render_run(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Min(0),
            Constraint::Length(5),
        ])
        .split(area);

    // Build/assemble status
    let (msg, style) = if app.last_compile_ok == Some(false) {
        let line = app.diag_line.map(|n| n + 1).unwrap_or(0);
        let text = app.diag_line_text.as_deref().unwrap_or("");
        let err = app.diag_msg.as_deref().unwrap_or("");
        (
            format!("Error line {}: {} ({})", line, text, err),
            Style::default().bg(Color::Red).fg(Color::Black),
        )
    } else if app.last_compile_ok == Some(true) {
        (
            app.last_assemble_msg.clone().unwrap_or_default(),
            Style::default().bg(Color::Green).fg(Color::Black),
        )
    } else {
        ("Not compiled".to_string(), Style::default())
    };
    let status = Paragraph::new(msg)
        .style(style)
        .block(Block::default().borders(Borders::ALL).title("Build"));
    f.render_widget(status, chunks[0]);

    // Run control status
    render_run_status(f, chunks[1], app);

    // Main area
    let main = chunks[2];
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(38),
            Constraint::Length(app.imem_width),
            Constraint::Min(46),
        ])
        .split(main);

    // --- Left sidebar: registers or RAM memory ---
    if app.show_registers {
        let reg_block = Block::default()
            .borders(Borders::ALL)
            .title("Registers — s:step r:run p:pause");
        let inner = reg_block.inner(cols[0]);
        let lines = inner.height.saturating_sub(2) as usize;
        let total = 33usize; // PC + x0..x31
        let max_scroll = total.saturating_sub(lines);
        let start = app.regs_scroll.min(max_scroll);
        let end = (start + lines).min(total);
        let mut rows = Vec::new();
        for idx in start..end {
            if idx == 0 {
                let val = app.cpu.pc;
                let changed = val != app.prev_pc;
                let style = if changed {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                };
                let val_str = if app.show_hex {
                    format!("0x{val:08x}")
                } else if app.show_signed {
                    format!("{}", val as i32)
                } else {
                    format!("{val}")
                };
                rows.push(Row::new(vec![
                    Cell::from("PC").style(style),
                    Cell::from(val_str).style(style),
                ]));
            } else {
                let reg_index = (idx - 1) as u8;
                let name = reg_name(reg_index);
                let val = app.cpu.x[reg_index as usize];
                let changed = val != app.prev_x[reg_index as usize];
                let style = if changed {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                };
                let val_str = if app.show_hex {
                    format!("0x{val:08x}")
                } else if app.show_signed {
                    format!("{}", val as i32)
                } else {
                    format!("{val}")
                };
                rows.push(Row::new(vec![
                    Cell::from(format!("x{reg_index:02} ({name})")).style(style),
                    Cell::from(val_str).style(style),
                ]));
            }
        }
        let reg_table =
            Table::new(rows, [Constraint::Length(14), Constraint::Length(20)]).block(reg_block);
        f.render_widget(reg_table, cols[0]);
    } else {
        let mem_block = Block::default()
            .borders(Borders::ALL)
            .title("RAM Memory — s:step r:run p:pause");
        f.render_widget(mem_block.clone(), cols[0]);

        let inner = mem_block.inner(cols[0]);
        let mut items = Vec::new();
        let base = app.mem_view_addr;
        let lines = inner.height.saturating_sub(2) as u32;
        let bytes = app.mem_view_bytes;
        for off in (0..lines).map(|i| i * bytes) {
            let addr = base.wrapping_add(off);
            let max = app.mem_size.saturating_sub(bytes as usize) as u32;
            if addr <= max {
                let val_str = match bytes {
                    4 => {
                        let w = app.mem.load32(addr);
                        if app.show_hex {
                            format!("0x{w:08x}")
                        } else if app.show_signed {
                            format!("{}", w as i32)
                        } else {
                            format!("{w}")
                        }
                    }
                    2 => {
                        let w = app.mem.load16(addr);
                        if app.show_hex {
                            format!("0x{w:04x}")
                        } else if app.show_signed {
                            format!("{}", (w as i16))
                        } else {
                            format!("{w}")
                        }
                    }
                    _ => {
                        let w = app.mem.load8(addr);
                        if app.show_hex {
                            format!("0x{w:02x}")
                        } else if app.show_signed {
                            format!("{}", (w as i8))
                        } else {
                            format!("{w}")
                        }
                    }
                };
                let mut text = format!("0x{addr:08x}: {val_str}");
                if addr == app.cpu.x[2] {
                    text.push_str("   ▶ sp");
                    let item = ListItem::new(text).style(Style::default().fg(Color::Yellow));
                    items.push(item);
                } else {
                    items.push(ListItem::new(text));
                }
            }
        }
        let list = List::new(items);
        f.render_widget(list, inner);
    }

    // --- Middle column: instruction memory around PC ---
    let border_style = if app.hover_imem_bar {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let imem_block = Block::default()
        .borders(Borders::ALL)
        .title("Instruction Memory")
        .border_style(border_style);
    f.render_widget(imem_block.clone(), cols[1]);
    let inner = imem_block.inner(cols[1]);
    let mut items = Vec::new();
    let base = app.cpu.pc.saturating_sub(32);
    let lines = inner.height.saturating_sub(2) as u32;
    for off in (0..lines).map(|i| i * 4) {
        let addr = base.wrapping_add(off);
        if in_mem_range(app, addr) {
            let w = app.mem.load32(addr);
            let marker = if addr == app.cpu.pc { "▶" } else { " " };
            let val_str = if app.show_hex {
                format!("0x{w:08x}")
            } else if app.show_signed {
                format!("{}", w as i32)
            } else {
                format!("{w}")
            };
            let dis = disasm_word(w);
            let mut item = ListItem::new(format!("{marker} 0x{addr:08x}: {val_str}  {dis}"));
            if addr == app.cpu.pc {
                item = item.style(Style::default().bg(Color::Yellow).fg(Color::Black));
            }
            items.push(item);
        }
    }
    let list = List::new(items);
    f.render_widget(list, inner);

    // Arrow indicator on right border
    let arrow_style = if app.hover_imem_bar {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let arrow_x = cols[1].x + cols[1].width - 1;
    let arrow_y = cols[1].y + cols[1].height / 2;
    let arrow_area = Rect::new(arrow_x, arrow_y, 1, 1);
    let arrow = Paragraph::new("▶").style(arrow_style);
    f.render_widget(arrow, arrow_area);

    // --- Right column: current instruction details ---
    let mid_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Min(4),
        ])
        .split(cols[2]);

    let (cur_word, disasm_str) = if in_mem_range(app, app.cpu.pc) {
        let w = app.mem.load32(app.cpu.pc);
        let dis = disasm_word(w);
        (w, dis)
    } else {
        (0, "<PC out of RAM>".to_string())
    };

    let pc_line = Paragraph::new(vec![
        Line::from(format!("PC = 0x{:08x}", app.cpu.pc)),
        Line::from(format!("Word = 0x{:08x}", cur_word)),
        Line::from(format!("Instr = {}", disasm_str)),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Current Instruction"),
    );
    f.render_widget(pc_line, mid_chunks[0]);

    let fmt = detect_format(cur_word);
    render_bit_fields(f, mid_chunks[1], cur_word, fmt);
    render_field_values(f, mid_chunks[2], cur_word, fmt);

    // --- Console ---
    render_console(f, chunks[3], app);
}

fn render_run_status(f: &mut Frame, area: Rect, app: &App) {
    let (view_text, view_color) = if app.show_registers {
        ("REGS", Color::Blue)
    } else {
        ("RAM", Color::Green)
    };
    let (fmt_text, fmt_color) = if app.show_hex {
        ("HEX", Color::Magenta)
    } else {
        ("DEC", Color::Cyan)
    };
    let (sign_text, sign_color) = if app.show_signed {
        ("SGN", Color::LightGreen)
    } else {
        ("UNS", Color::LightBlue)
    };
    let (run_text, run_color) = if app.is_running {
        ("RUN", Color::Green)
    } else {
        ("PAUSE", Color::Red)
    };

    let button = |text: &str, color: Color, hovered: bool| {
        let mut style = Style::default().fg(Color::Black).bg(color);
        if hovered {
            // Destaque no hover usando negrito; Color não expõe r/g/b para todos os variantes.
            style = style.add_modifier(Modifier::ITALIC);
        } else {
            style = style.add_modifier(Modifier::DIM).bg(color); // Se não estiver hover, usar a cor original
        }
        Span::styled(format!("[{text}]"), style)
    };

    let mut spans = vec![
        Span::raw("View "),
        button(
            view_text,
            view_color,
            app.hover_run_button == Some(RunButton::View),
        ),
    ];

    if !app.show_registers {
        let (region_text, region_color) = match app.mem_region {
            MemRegion::Data => ("DATA", Color::Yellow),
            MemRegion::Stack => ("STACK", Color::LightBlue),
            MemRegion::Custom => ("ADDR", Color::Gray),
        };
        spans.push(Span::raw("  Region "));
        spans.push(button(
            region_text,
            region_color,
            app.hover_run_button == Some(RunButton::Region),
        ));
    }

    spans.push(Span::raw("  Format "));
    spans.push(button(
        fmt_text,
        fmt_color,
        app.hover_run_button == Some(RunButton::Format),
    ));

    spans.push(Span::raw("  Sign "));
    spans.push(button(
        sign_text,
        sign_color,
        app.hover_run_button == Some(RunButton::Sign),
    ));

    if !app.show_registers {
        let bytes_text = match app.mem_view_bytes {
            4 => "4B",
            2 => "2B",
            _ => "1B",
        };
        spans.push(Span::raw("  Bytes "));
        spans.push(button(
            bytes_text,
            Color::Yellow,
            app.hover_run_button == Some(RunButton::Bytes),
        ));
    }

    spans.push(Span::raw("  State "));
    spans.push(button(
        run_text,
        run_color,
        app.hover_run_button == Some(RunButton::State),
    ));

    let line1 = Line::from(spans);
    let line2 = Line::from("Commands: s=step  r=run  p=pause  Up/Down/PgUp/PgDn scroll");
    let para = Paragraph::new(vec![line1, line2])
        .block(Block::default().borders(Borders::ALL).title("Run Controls"));
    f.render_widget(para, area);
}

fn in_mem_range(app: &App, addr: u32) -> bool {
    (addr as usize) < app.mem_size.saturating_sub(3)
}

fn reg_name(i: u8) -> &'static str {
    match i {
        0 => "zero",
        1 => "ra",
        2 => "sp",
        3 => "gp",
        4 => "tp",
        5 => "t0",
        6 => "t1",
        7 => "t2",
        8 => "s0/fp",
        9 => "s1",
        10 => "a0",
        11 => "a1",
        12 => "a2",
        13 => "a3",
        14 => "a4",
        15 => "a5",
        16 => "a6",
        17 => "a7",
        18 => "s2",
        19 => "s3",
        20 => "s4",
        21 => "s5",
        22 => "s6",
        23 => "s7",
        24 => "s8",
        25 => "s9",
        26 => "s10",
        27 => "s11",
        28 => "t3",
        29 => "t4",
        30 => "t5",
        31 => "t6",
        _ => "",
    }
}

#[derive(Clone, Copy)]
enum EncFormat {
    R,
    I,
    S,
    B,
    U,
    J,
}

fn detect_format(word: u32) -> EncFormat {
    let opc = word & 0x7f;
    match opc {
        0x33 => EncFormat::R,
        0x13 | 0x03 | 0x67 => EncFormat::I,
        0x23 => EncFormat::S,
        0x63 => EncFormat::B,
        0x37 | 0x17 => EncFormat::U,
        0x6f => EncFormat::J,
        _ => EncFormat::R,
    }
}

fn render_bit_fields(f: &mut Frame, area: Rect, w: u32, fmt: EncFormat) {
    use Color::*;
    let (segments, title) = match fmt {
        EncFormat::R => (
            vec![
                ("funct7", 7, Red),
                ("rs2", 5, LightRed),
                ("rs1", 5, LightMagenta),
                ("funct3", 3, Yellow),
                ("rd", 5, LightGreen),
                ("opcode", 7, Cyan),
            ],
            "Field map (R-type)",
        ),
        EncFormat::I => (
            vec![
                ("imm[11:0]", 12, Blue),
                ("rs1", 5, LightMagenta),
                ("funct3", 3, Yellow),
                ("rd", 5, LightGreen),
                ("opcode", 7, Cyan),
            ],
            "Field map (I-type)",
        ),
        EncFormat::S => (
            vec![
                ("imm[11:5]", 7, Blue),
                ("rs2", 5, LightRed),
                ("rs1", 5, LightMagenta),
                ("funct3", 3, Yellow),
                ("imm[4:0]", 5, Blue),
                ("opcode", 7, Cyan),
            ],
            "Field map (S-type)",
        ),
        EncFormat::B => (
            vec![
                ("imm[12]", 1, Blue),
                ("imm[10:5]", 6, Blue),
                ("rs2", 5, LightRed),
                ("rs1", 5, LightMagenta),
                ("funct3", 3, Yellow),
                ("imm[4:1]", 4, Blue),
                ("imm[11]", 1, Blue),
                ("opcode", 7, Cyan),
            ],
            "Field map (B-type)",
        ),
        EncFormat::U => (
            vec![
                ("imm[31:12]", 20, Blue),
                ("rd", 5, LightGreen),
                ("opcode", 7, Cyan),
            ],
            "Field map (U-type)",
        ),
        EncFormat::J => (
            vec![
                ("imm[20]", 1, Blue),
                ("imm[10:1]", 10, Blue),
                ("imm[11]", 1, Blue),
                ("imm[19:12]", 8, Blue),
                ("rd", 5, LightGreen),
                ("opcode", 7, Cyan),
            ],
            "Field map (J-type)",
        ),
    };

    let label_spans: Vec<Span> = segments
        .iter()
        .map(|(label, width, color)| {
            let bar = "▮".repeat((*width).max(1) as usize);
            Span::styled(format!("{} {} ", bar, label), Style::default().fg(*color))
        })
        .collect();

    let bit_str = format!("{:032b}", w);
    let mut bit_spans: Vec<Span> = Vec::new();
    let mut idx = 0usize;
    for (i, (_, width, color)) in segments.iter().enumerate() {
        let end = idx + (*width as usize);
        let slice = &bit_str[idx..end];
        bit_spans.push(Span::styled(slice.to_string(), Style::default().fg(*color)));
        if i + 1 < segments.len() {
            bit_spans.push(Span::raw(" "));
        }
        idx = end;
    }

    let lines = vec![Line::from(label_spans), Line::from(bit_spans)];
    let para = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: true });
    f.render_widget(para, area);
}

fn render_field_values(f: &mut Frame, area: Rect, w: u32, fmt: EncFormat) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Parsed fields");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut text = Vec::new();
    match fmt {
        EncFormat::R => {
            let funct7 = (w >> 25) & 0x7f;
            let rs2 = (w >> 20) & 0x1f;
            let rs1 = (w >> 15) & 0x1f;
            let funct3 = (w >> 12) & 0x7;
            let rd = (w >> 7) & 0x1f;
            let opcode = w & 0x7f;
            text.push(Line::from(format!(
                "funct7={:#04x}  rs2={}  rs1={}  funct3={:#03x}  rd={}  opcode={:#04x}",
                funct7, rs2, rs1, funct3, rd, opcode
            )));
        }
        EncFormat::I => {
            let imm = (((w >> 20) as i32) << 20) >> 20;
            let rs1 = (w >> 15) & 0x1f;
            let funct3 = (w >> 12) & 0x7;
            let rd = (w >> 7) & 0x1f;
            let opcode = w & 0x7f;
            text.push(Line::from(format!(
                "imm={}  rs1={}  funct3={:#03x}  rd={}  opcode={:#04x}",
                imm, rs1, funct3, rd, opcode
            )));
            if matches!(funct3, 0x1 | 0x5) {
                let shamt = (w >> 20) & 0x1f;
                let f7 = (w >> 25) & 0x7f;
                text.push(Line::from(format!(
                    "(shift) funct7={:#04x} shamt={} rs1={} rd={}",
                    f7, shamt, rs1, rd
                )));
            }
        }
        EncFormat::S => {
            let imm_4_0 = (w >> 7) & 0x1f;
            let funct3 = (w >> 12) & 0x7;
            let rs1 = (w >> 15) & 0x1f;
            let rs2 = (w >> 20) & 0x1f;
            let imm_11_5 = (w >> 25) & 0x7f;
            let opcode = w & 0x7f;
            let imm = (((((imm_11_5 << 5) | imm_4_0) as i32) << 20) >> 20) as i32;
            text.push(Line::from(format!("imm[11:5]={:#04x} imm[4:0]={:#03x} => imm={}  rs2={} rs1={} funct3={:#03x} opcode={:#04x}", imm_11_5, imm_4_0, imm, rs2, rs1, funct3, opcode)));
        }
        EncFormat::B => {
            let b12 = (w >> 31) & 0x1;
            let b10_5 = (w >> 25) & 0x3f;
            let rs2 = (w >> 20) & 0x1f;
            let rs1 = (w >> 15) & 0x1f;
            let f3 = (w >> 12) & 0x7;
            let b4_1 = (w >> 8) & 0xf;
            let b11 = (w >> 7) & 0x1;
            let opc = w & 0x7f;
            let imm = (((((b12 << 12) | (b11 << 11) | (b10_5 << 5) | (b4_1 << 1)) as i32) << 19)
                >> 19) as i32;
            text.push(Line::from(format!("b12={} b11={} b10:5={:#04x} b4:1={:#03x} => imm={}  rs2={} rs1={} f3={:#03x} opc={:#04x}", b12, b11, b10_5, b4_1, imm, rs2, rs1, f3, opc)));
        }
        EncFormat::U => {
            let rd = (w >> 7) & 0x1f;
            let opc = w & 0x7f;
            let imm = (w & 0xfffff000) as i32;
            text.push(Line::from(format!(
                "imm[31:12]={:#07x} => imm={}  rd={} opc={:#04x}",
                imm >> 12,
                imm,
                rd,
                opc
            )));
        }
        EncFormat::J => {
            let b20 = (w >> 31) & 1;
            let b10_1 = (w >> 21) & 0x3ff;
            let b11 = (w >> 20) & 1;
            let b19_12 = (w >> 12) & 0xff;
            let rd = (w >> 7) & 0x1f;
            let opc = w & 0x7f;
            let imm = (((((b20 << 20) | (b19_12 << 12) | (b11 << 11) | (b10_1 << 1)) as i32) << 11)
                >> 11) as i32;
            text.push(Line::from(format!(
                "b20={} b19:12={:#04x} b11={} b10:1={:#05x} => imm={} rd={} opc={:#04x}",
                b20, b19_12, b11, b10_1, imm, rd, opc
            )));
        }
    }

    let para = Paragraph::new(text).wrap(Wrap { trim: true });
    f.render_widget(para, inner);
}

fn render_console(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Console — Ctrl+Up/Down scroll");
    let inner = block.inner(area);
    let h = inner.height.saturating_sub(1) as usize;
    let total = app.console.lines.len();
    let scroll = app.console.scroll;
    let start = total.saturating_sub(h + scroll);
    let end = total.saturating_sub(scroll);
    let mut lines: Vec<Line> = app.console.lines[start..end]
        .iter()
        .map(|l| Line::from(l.as_str()))
        .collect();
    if app.console.reading {
        lines.push(Line::from(format!("> {}", app.console.current)));
    }
    let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
    f.render_widget(para, area);
}

fn disasm_word(w: u32) -> String {
    match falcon::decoder::decode(w) {
        Ok(ins) => pretty_instr(&ins),
        Err(e) => format!("<decode error: {e}>"),
    }
}

fn pretty_instr(i: &falcon::instruction::Instruction) -> String {
    use falcon::instruction::Instruction::*;
    match *i {
        Add { rd, rs1, rs2 } => format!("add  x{rd}, x{rs1}, x{rs2}"),
        Sub { rd, rs1, rs2 } => format!("sub  x{rd}, x{rs1}, x{rs2}"),
        And { rd, rs1, rs2 } => format!("and  x{rd}, x{rs1}, x{rs2}"),
        Or { rd, rs1, rs2 } => format!("or   x{rd}, x{rs1}, x{rs2}"),
        Xor { rd, rs1, rs2 } => format!("xor  x{rd}, x{rs1}, x{rs2}"),
        Sll { rd, rs1, rs2 } => format!("sll  x{rd}, x{rs1}, x{rs2}"),
        Srl { rd, rs1, rs2 } => format!("srl  x{rd}, x{rs1}, x{rs2}"),
        Sra { rd, rs1, rs2 } => format!("sra  x{rd}, x{rs1}, x{rs2}"),
        Slt { rd, rs1, rs2 } => format!("slt  x{rd}, x{rs1}, x{rs2}"),
        Sltu { rd, rs1, rs2 } => format!("sltu x{rd}, x{rs1}, x{rs2}"),
        Mul { rd, rs1, rs2 } => format!("mul  x{rd}, x{rs1}, x{rs2}"),
        Mulh { rd, rs1, rs2 } => format!("mulh x{rd}, x{rs1}, x{rs2}"),
        Mulhsu { rd, rs1, rs2 } => format!("mulhsu x{rd}, x{rs1}, x{rs2}"),
        Mulhu { rd, rs1, rs2 } => format!("mulhu x{rd}, x{rs1}, x{rs2}"),
        Div { rd, rs1, rs2 } => format!("div  x{rd}, x{rs1}, x{rs2}"),
        Divu { rd, rs1, rs2 } => format!("divu x{rd}, x{rs1}, x{rs2}"),
        Rem { rd, rs1, rs2 } => format!("rem  x{rd}, x{rs1}, x{rs2}"),
        Remu { rd, rs1, rs2 } => format!("remu x{rd}, x{rs1}, x{rs2}"),
        Addi { rd, rs1, imm } => format!("addi x{rd}, x{rs1}, {imm}"),
        Andi { rd, rs1, imm } => format!("andi x{rd}, x{rs1}, {imm}"),
        Ori { rd, rs1, imm } => format!("ori  x{rd}, x{rs1}, {imm}"),
        Xori { rd, rs1, imm } => format!("xori x{rd}, x{rs1}, {imm}"),
        Slti { rd, rs1, imm } => format!("slti x{rd}, x{rs1}, {imm}"),
        Sltiu { rd, rs1, imm } => format!("sltiu x{rd}, x{rs1}, {imm}"),
        Slli { rd, rs1, shamt } => format!("slli x{rd}, x{rs1}, {shamt}"),
        Srli { rd, rs1, shamt } => format!("srli x{rd}, x{rs1}, {shamt}"),
        Srai { rd, rs1, shamt } => format!("srai x{rd}, x{rs1}, {shamt}"),
        Lb { rd, rs1, imm } => format!("lb   x{rd}, {imm}(x{rs1})"),
        Lh { rd, rs1, imm } => format!("lh   x{rd}, {imm}(x{rs1})"),
        Lw { rd, rs1, imm } => format!("lw   x{rd}, {imm}(x{rs1})"),
        Lbu { rd, rs1, imm } => format!("lbu  x{rd}, {imm}(x{rs1})"),
        Lhu { rd, rs1, imm } => format!("lhu  x{rd}, {imm}(x{rs1})"),
        Sb { rs2, rs1, imm } => format!("sb   x{rs2}, {imm}(x{rs1})"),
        Sh { rs2, rs1, imm } => format!("sh   x{rs2}, {imm}(x{rs1})"),
        Sw { rs2, rs1, imm } => format!("sw   x{rs2}, {imm}(x{rs1})"),
        Beq { rs1, rs2, imm } => format!("beq  x{rs1}, x{rs2}, {imm}"),
        Bne { rs1, rs2, imm } => format!("bne  x{rs1}, x{rs2}, {imm}"),
        Blt { rs1, rs2, imm } => format!("blt  x{rs1}, x{rs2}, {imm}"),
        Bge { rs1, rs2, imm } => format!("bge  x{rs1}, x{rs2}, {imm}"),
        Bltu { rs1, rs2, imm } => format!("bltu x{rs1}, x{rs2}, {imm}"),
        Bgeu { rs1, rs2, imm } => format!("bgeu x{rs1}, x{rs2}, {imm}"),
        Lui { rd, imm } => format!("lui  x{rd}, {imm}"),
        Auipc { rd, imm } => format!("auipc x{rd}, {imm}"),
        Jal { rd, imm } => format!("jal  x{rd}, {imm}"),
        Jalr { rd, rs1, imm } => format!("jalr x{rd}, x{rs1}, {imm}"),
        Ecall => "ecall".to_string(),
        Ebreak => "ebreak".to_string(),
    }
}
