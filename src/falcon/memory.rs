const MEMORY_SIZE: usize = 4096;
#[derive(Clone)]
pub struct Memory {
    mem: Vec<u8>,
}

impl Memory {
    pub fn new() -> Self {
        Memory {
            mem: vec![0; MEMORY_SIZE],
        }
    }
    pub fn read_word(&self, pointer: usize) -> u32 {
        u32::from_be_bytes(self.mem[pointer..pointer + 4].try_into().unwrap())
    }
    pub fn write_word(&mut self, pointer: usize, value: u32) {
        self.mem[pointer..pointer + 4].copy_from_slice(&value.to_be_bytes());
    }

    pub fn read_d_word(&self, pointer: usize) -> u64 {
        u64::from_be_bytes(self.mem[pointer..pointer + 8].try_into().unwrap())
    }
    pub fn write_d_word(&mut self, pointer: usize, value: u64) {
        self.mem[pointer..pointer + 8].copy_from_slice(&value.to_be_bytes());
    }

    pub fn read_q_word(&self, pointer: usize) -> u8 {
        self.mem[pointer]
    }
    pub fn write_q_word(&mut self, pointer: usize, value: u8) {
        self.mem[pointer] = value;
    }

    pub fn read_float(&self, pointer: usize) -> f32 {
        f32::from_be_bytes(self.mem[pointer..pointer + 4].try_into().unwrap())
    }
    pub fn write_float(&mut self, pointer: usize, value: f32) {
        self.mem[pointer..pointer + 4].copy_from_slice(&value.to_be_bytes());
    }

    pub fn read_double(&self, pointer: usize) -> f64 {
        f64::from_be_bytes(self.mem[pointer..pointer + 8].try_into().unwrap())
    }

    pub fn write_double(&mut self, pointer: usize, value: f64) {
        self.mem[pointer..pointer + 8].copy_from_slice(&value.to_be_bytes());
    }
}
