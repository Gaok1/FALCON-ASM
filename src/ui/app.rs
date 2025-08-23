use super::{
    console::Console,
    editor::Editor,
    input::{handle_key, handle_mouse},
    view::ui,
};
use crate::falcon::{self, Cpu, Ram};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
};
use ratatui::{DefaultTerminal, layout::Rect};
use std::{
    io,
    time::{Duration, Instant},
};

#[derive(PartialEq, Eq, Copy, Clone)]
pub(super) enum Tab {
    Editor,
    Run,
    Docs,
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub(super) enum EditorMode {
    Insert,
    Command,
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub(super) enum MemRegion {
    Data,
    Stack,
    Custom,
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub(super) enum RunButton {
    View,
    Format,
    Sign,
    Bytes,
    Region,
    State,
}

pub struct App {
    pub(super) tab: Tab,
    pub(super) mode: EditorMode,
    // Editor state
    pub(super) editor: Editor,
    pub(super) editor_dirty: bool,
    pub(super) last_edit_at: Option<Instant>,
    pub(super) auto_check_delay: Duration,
    pub(super) last_assemble_msg: Option<String>,
    pub(super) last_compile_ok: Option<bool>,

    // Compile diagnostics
    pub(super) diag_line: Option<usize>, // 0-based line index
    pub(super) diag_msg: Option<String>,
    pub(super) diag_line_text: Option<String>,

    // Execution state
    pub(super) cpu: Cpu,
    pub(super) prev_x: [u32; 32],
    pub(super) prev_pc: u32,
    pub(super) mem: Ram,
    pub(super) mem_size: usize,
    pub(super) base_pc: u32,
    pub(super) data_base: u32,
    pub(super) mem_view_addr: u32,
    pub(super) mem_view_bytes: u32,
    pub(super) mem_region: MemRegion,
    pub(super) show_registers: bool,
    pub(super) show_hex: bool,
    pub(super) show_signed: bool,
    pub(super) imem_width: u16,
    pub(super) hover_imem_bar: bool,
    pub(super) imem_drag: bool,
    pub(super) imem_drag_start_x: u16,
    pub(super) imem_width_start: u16,
    pub(super) console_height: u16,
    pub(super) hover_console_bar: bool,
    pub(super) console_drag: bool,
    pub(super) console_drag_start_y: u16,
    pub(super) console_height_start: u16,
    pub(super) regs_scroll: usize,
    pub(super) is_running: bool,
    pub(super) last_step_time: Instant,
    pub(super) step_interval: Duration,
    pub(super) faulted: bool,
    pub(super) show_exit_popup: bool,
    pub(super) should_quit: bool,

    // Docs state
    pub(super) docs_scroll: usize,

    // Mouse tracking
    pub(super) mouse_x: u16,
    pub(super) mouse_y: u16,
    pub(super) hover_tab: Option<Tab>,
    pub(super) hover_run_button: Option<RunButton>,

    // Console for program I/O
    pub(super) console: Console,
}

impl App {
    pub fn new() -> Self {
        let mut cpu = Cpu::default();
        let base_pc = 0x0000_0000;
        cpu.pc = base_pc;
        let mem_size = 128 * 1024;
        cpu.write(2, mem_size as u32 - 4); // initialize stack pointer to a valid word
        let data_base = base_pc + 0x1000;
        Self {
            tab: Tab::Editor,
            mode: EditorMode::Insert,
            editor: Editor::with_sample(),
            editor_dirty: true,
            last_edit_at: Some(Instant::now()),
            auto_check_delay: Duration::from_millis(400),
            last_assemble_msg: None,
            last_compile_ok: None,
            diag_line: None,
            diag_msg: None,
            diag_line_text: None,
            cpu,
            prev_x: [0; 32],
            prev_pc: base_pc,
            mem_size,
            mem: Ram::new(mem_size),
            base_pc,
            data_base,
            mem_view_addr: data_base,
            mem_view_bytes: 4,
            mem_region: MemRegion::Data,
            show_registers: true,
            show_hex: true,
            show_signed: false,
            imem_width: 38,
            hover_imem_bar: false,
            imem_drag: false,
            imem_drag_start_x: 0,
            imem_width_start: 38,
            console_height: 5,
            hover_console_bar: false,
            console_drag: false,
            console_drag_start_y: 0,
            console_height_start: 5,
            regs_scroll: 0,
            is_running: false,
            last_step_time: Instant::now(),
            step_interval: Duration::from_millis(80),
            faulted: false,
            show_exit_popup: false,
            should_quit: false,
            docs_scroll: 0,
            mouse_x: 0,
            mouse_y: 0,
            hover_tab: None,
            hover_run_button: None,
            console: Console::default(),
        }
    }

    pub(super) fn assemble_and_load(&mut self) {
        use falcon::asm::assemble;
        use falcon::program::{load_bytes, load_words};

        self.prev_x = self.cpu.x; // keep snapshot before reset
        self.mem_size = 128 * 1024;
        self.cpu = Cpu::default();
        self.cpu.pc = self.base_pc;
        self.prev_pc = self.cpu.pc;
        self.cpu.write(2, self.mem_size as u32 - 4); // reset stack pointer
        self.mem = Ram::new(self.mem_size);
        self.faulted = false;

        match assemble(&self.editor.text(), self.base_pc) {
            Ok(prog) => {
                load_words(&mut self.mem, self.base_pc, &prog.text);
                load_bytes(&mut self.mem, prog.data_base, &prog.data);

                self.data_base = prog.data_base;
                self.mem_view_addr = prog.data_base;
                self.mem_region = MemRegion::Data;

                self.last_assemble_msg = Some(format!(
                    "Assembled {} instructions, {} data bytes.",
                    prog.text.len(),
                    prog.data.len()
                ));
                self.last_compile_ok = Some(true);
                self.diag_line = None;
                self.diag_msg = None;
                self.diag_line_text = None;
            }
            Err(e) => {
                self.diag_line = Some(e.line);
                self.diag_msg = Some(e.msg.clone());
                self.diag_line_text = self.editor.lines.get(e.line).cloned();
                self.last_compile_ok = Some(false);
                self.last_assemble_msg =
                    Some(format!("Assemble error at line {}: {}", e.line + 1, e.msg));
            }
        }
    }

    // Lightweight background syntax check (does not reset CPU/mem)
    fn check_assemble(&mut self) {
        use falcon::asm::assemble;
        match assemble(&self.editor.text(), self.base_pc) {
            Ok(prog) => {
                self.last_assemble_msg = Some(format!(
                    "OK: {} instructions, {} data bytes",
                    prog.text.len(),
                    prog.data.len()
                ));
                self.last_compile_ok = Some(true);
                self.diag_line = None;
                self.diag_msg = None;
                self.diag_line_text = None;
            }
            Err(e) => {
                self.diag_line = Some(e.line);
                self.diag_msg = Some(e.msg.clone());
                self.diag_line_text = self.editor.lines.get(e.line).cloned();
                self.last_compile_ok = Some(false);
            }
        }
        self.editor_dirty = false;
    }

    fn tick(&mut self) {
        if self.is_running && self.last_step_time.elapsed() >= self.step_interval {
            self.single_step();
            self.last_step_time = Instant::now();
        }
        // auto syntax check while in editor, with debounce
        if matches!(self.tab, Tab::Editor) && self.editor_dirty {
            if let Some(t) = self.last_edit_at {
                if t.elapsed() >= self.auto_check_delay {
                    self.check_assemble();
                }
            }
        }
    }

    pub(super) fn single_step(&mut self) {
        self.prev_x = self.cpu.x; // snapshot before step
        self.prev_pc = self.cpu.pc;
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            falcon::exec::step(&mut self.cpu, &mut self.mem, &mut self.console)
        }));
        let alive = match res {
            Ok(v) => v,
            Err(_) => {
                self.faulted = true;
                false
            }
        };
        if !alive {
            self.is_running = false;
            if !self.console.reading {
                self.faulted = true;
            }
        }
    }
}

pub fn run(terminal: &mut DefaultTerminal, mut app: App) -> io::Result<()> {
    execute!(terminal.backend_mut(), EnableMouseCapture)?;
    let last_draw = Instant::now();
    loop {
        // Input
        if event::poll(Duration::from_millis(10))? {
            match event::read()? {
                Event::Key(key) => {
                    if handle_key(&mut app, key)? {
                        break;
                    }
                }
                Event::Mouse(me) => {
                    let size = terminal.size()?;
                    let area = Rect::new(0, 0, size.width, size.height);
                    handle_mouse(&mut app, me, area);
                    if app.should_quit {
                        break;
                    }
                }
                _ => {}
            }
        }
        if app.should_quit {
            break;
        }
        // Tick/run
        app.tick();
        // Draw ~60 FPS cap
        if last_draw.elapsed() >= Duration::from_millis(16) {
            terminal.draw(|f| ui(f, &app))?;
            //last_draw = Instant::now();
        }
    }
    execute!(terminal.backend_mut(), DisableMouseCapture)?;
    Ok(())
}
