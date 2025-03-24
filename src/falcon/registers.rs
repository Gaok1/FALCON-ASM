#[derive(Debug, Clone,Copy,Default)]
pub struct Register {
    pub bytes: [u8; 8],
}

//constructor
impl Register {
    pub fn new() -> Self {
        Register { bytes: [0; 8] }
    }

    pub fn valued(value: u64) -> Self {
        Register { bytes: value.to_be_bytes() }
    }
}

// read and store
impl Register {
    // Lê os 4 primeiros bytes como u32
    fn read_word(&self) -> u32 {
        u32::from_be_bytes(self.bytes[0..4].try_into().unwrap())
    }

    // Lê os 8 bytes como u64 (DWORD)
    fn read_d_word(&self) -> u64 {
        u64::from_be_bytes(self.bytes.try_into().unwrap())
    }

    // Lê só o primeiro byte como u8 (QWORD)
    fn read_q_word(&self) -> u8 {
        self.bytes[0]
    }

    fn write_word(&mut self, value: u32) {
        self.bytes[0..4].copy_from_slice(&value.to_be_bytes());
    }

    fn write_d_word(&mut self, value: u64) {
        self.bytes.copy_from_slice(&value.to_be_bytes());
    }

    fn write_q_word(&mut self, value: u8) {
        self.bytes[0] = value;
    }

    // Float: 4 bytes → usa [0..4]
    fn read_float(&self) -> f32 {
        f32::from_be_bytes(self.bytes[0..4].try_into().unwrap())
    }

    fn write_float(&mut self, value: f32) {
        self.bytes[0..4].copy_from_slice(&value.to_be_bytes());
    }

    // Double: 8 bytes
    fn read_double(&self) -> f64 {
        f64::from_be_bytes(self.bytes.try_into().unwrap())
    }

    fn write_double(&mut self, value: f64) {
        self.bytes.copy_from_slice(&value.to_be_bytes());
    }
}
  