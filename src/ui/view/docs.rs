use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use std::cmp::min;

use super::App;

pub(super) fn render_docs(f: &mut Frame, area: Rect, app: &App) {
    let text = DOC_TEXT;
    let lines: Vec<&str> = text.lines().collect();
    let h = area.height.saturating_sub(2) as usize;
    let start = app.docs_scroll.min(lines.len());
    let end = min(lines.len(), start + h);
    let body = lines[start..end].join("\n");
    let para = Paragraph::new(body)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Docs — Supported Instructions (Up/Down/PageUp/PageDown)"),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(para, area);
}

const DOC_TEXT: &str = r#"Falcon ASM — Supported Instructions (RV32I MVP)

R-type (opcode 0x33):
  ADD, SUB, AND, OR, XOR, SLL, SRL, SRA, SLT, SLTU, MUL, MULH,
  MULHSU, MULHU, DIV, DIVU, REM, REMU

I-type (opcode 0x13):
  ADDI, ANDI, ORI, XORI, SLTI, SLTIU, SLLI, SRLI, SRAI

Loads (opcode 0x03):
  LB, LH, LW, LBU, LHU

Stores (opcode 0x23):
  SB, SH, SW

Branches (opcode 0x63):
  BEQ, BNE, BLT, BGE, BLTU, BGEU

Upper immediates:
  LUI (0x37), AUIPC (0x17)

Jumps:
  JAL (0x6F), JALR (0x67)

System:
  ECALL (0x00000073), EBREAK (0x00100073)

Notes:
• PC advances +4 each instruction. Branch/JAL immediates are byte offsets (must be even).
• Loads/Stores syntax: imm(rs1). Labels supported by 2-pass assembler.
• Pseudoinstructions: nop, mv, li(12-bit), j, call, jr, ret, subi, la, push, pop, print, printString, read.
"#;
