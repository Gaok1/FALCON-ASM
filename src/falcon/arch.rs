use super::{memory::Memory, registers::Register};

#[derive(Debug, Default, Clone)]
struct MemorySegmentsPointer {
    pub data_section_start_pointer : u64,
    pub data_section_end_pointer : u64,
    pub text_section_start_pointer : u64,
    pub text_section_end_pointer : u64,
    pub stack_pointer : u64,
}

#[derive( Clone)]
pub struct FalconArch {
    pub registers : [Register; 20],
    pub pc : Register,
    pub sp : Register,
    pub zero : Register,
    pub memory : Memory,
    segments_pointer : MemorySegmentsPointer,
    //parser : Parser,
}

impl FalconArch {
    
    fn new() -> Self {
        FalconArch {
            registers: [Register::new(); 20],
            pc : Register::valued(0),
            sp : Register::valued(0),
            zero : Register::valued(0),
            memory : Memory::new(),
            segments_pointer : MemorySegmentsPointer::default(),
        }
    }
}