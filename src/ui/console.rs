use std::collections::VecDeque;

pub struct ConsoleLine {
    pub text: String,
    pub is_error: bool,
}

impl Default for ConsoleLine {
    fn default() -> Self {
        Self {
            text: String::new(),
            is_error: false,
        }
    }
}

#[derive(Default)]
pub struct Console {
    /// Lines to be rendered on screen
    pub lines: Vec<ConsoleLine>,
    /// Scroll offset from the bottom (0 = follow latest)
    pub scroll: usize,
    /// Queue of lines waiting to be consumed by the emulator (read syscall)
    input: VecDeque<String>,
    /// When true the emulator is waiting for user input
    pub reading: bool,
    /// Current line being typed by the user
    pub current: String,
}

impl Console {
    pub fn push_line<S: Into<String>>(&mut self, line: S) {
        self.lines.push(ConsoleLine { text: line.into(), is_error: false });
    }

    pub fn push_error<S: Into<String>>(&mut self, line: S) {
        self.lines.push(ConsoleLine { text: line.into(), is_error: true });
    }

    /// Provide a line of user input (displayed and queued)
    pub fn push_input<S: Into<String>>(&mut self, line: S) {
        let line = line.into();
        self.lines.push(ConsoleLine { text: line.clone(), is_error: false });
        self.input.push_back(line);
    }

    /// Retrieve next queued input line for the emulator
    pub fn read_line(&mut self) -> Option<String> {
        self.input.pop_front()
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.scroll = 0;
    }
}

