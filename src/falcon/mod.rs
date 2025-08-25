pub mod arch;
pub mod errors;
pub mod exec;
pub mod instruction;
pub mod memory;
pub mod registers;
pub mod syscall;

pub mod decoder;

// 🆕
pub mod asm;
pub mod encoder;

pub mod program;

pub use instruction::Instruction;
pub use memory::{Bus, Ram};
pub use registers::Cpu;
