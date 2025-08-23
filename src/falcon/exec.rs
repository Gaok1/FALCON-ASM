// falcon/exec.rs
use crate::falcon::{instruction::Instruction, memory::Bus, registers::Cpu};

use crate::falcon::syscall::handle_syscall;
use crate::ui::Console;

pub fn step<B: Bus>(cpu: &mut Cpu, mem: &mut B, console: &mut Console) -> bool {
    let pc = cpu.pc;
    let word = mem.load32(pc);
    let instr = match crate::falcon::decoder::decode(word) {
        Ok(i) => i,
        Err(_) => {
            console.push_error(format!(
                "Invalid instruction 0x{word:08X} at 0x{pc:08X}"
            ));
            return false;
        }
    };
    cpu.pc = pc.wrapping_add(4);

    match instr {
        // R
        Instruction::Add { rd, rs1, rs2 } => {
            cpu.write(rd, cpu.read(rs1).wrapping_add(cpu.read(rs2)))
        }
        Instruction::Sub { rd, rs1, rs2 } => {
            cpu.write(rd, cpu.read(rs1).wrapping_sub(cpu.read(rs2)))
        }
        Instruction::And { rd, rs1, rs2 } => cpu.write(rd, cpu.read(rs1) & cpu.read(rs2)),
        Instruction::Or { rd, rs1, rs2 } => cpu.write(rd, cpu.read(rs1) | cpu.read(rs2)),
        Instruction::Xor { rd, rs1, rs2 } => cpu.write(rd, cpu.read(rs1) ^ cpu.read(rs2)),
        Instruction::Sll { rd, rs1, rs2 } => cpu.write(rd, cpu.read(rs1) << (cpu.read(rs2) & 0x1F)),
        Instruction::Srl { rd, rs1, rs2 } => cpu.write(rd, cpu.read(rs1) >> (cpu.read(rs2) & 0x1F)),
        Instruction::Sra { rd, rs1, rs2 } => {
            let s = (cpu.read(rs2) & 0x1F) as u32;
            cpu.write(rd, ((cpu.read(rs1) as i32) >> s) as u32);
        }
        Instruction::Slt { rd, rs1, rs2 } => {
            let v = (cpu.read(rs1) as i32) < (cpu.read(rs2) as i32);
            cpu.write(rd, v as u32);
        }
        Instruction::Sltu { rd, rs1, rs2 } => cpu.write(rd, (cpu.read(rs1) < cpu.read(rs2)) as u32),
        Instruction::Mul { rd, rs1, rs2 } => {
            let res = (cpu.read(rs1) as i32 as i64).wrapping_mul(cpu.read(rs2) as i32 as i64);
            cpu.write(rd, res as u32);
        }
        Instruction::Mulh { rd, rs1, rs2 } => {
            let res = (cpu.read(rs1) as i32 as i64).wrapping_mul(cpu.read(rs2) as i32 as i64);
            cpu.write(rd, (res >> 32) as u32);
        }
        Instruction::Mulhsu { rd, rs1, rs2 } => {
            let res = (cpu.read(rs1) as i32 as i64).wrapping_mul(cpu.read(rs2) as u64 as i64);
            cpu.write(rd, (res >> 32) as u32);
        }
        Instruction::Mulhu { rd, rs1, rs2 } => {
            let res = (cpu.read(rs1) as u64).wrapping_mul(cpu.read(rs2) as u64);
            cpu.write(rd, (res >> 32) as u32);
        }
        Instruction::Div { rd, rs1, rs2 } => {
            let num = cpu.read(rs1) as i32;
            let den = cpu.read(rs2) as i32;
            if den == 0 {
                console.push_error("Division by zero");
                return false;
            }
            let val = { num.wrapping_div(den) };
            cpu.write(rd, val as u32);
        }
        Instruction::Divu { rd, rs1, rs2 } => {
            let den = cpu.read(rs2);
            if den == 0 {
                console.push_error("Division by zero");
                return false;
            }
            let val = cpu.read(rs1).wrapping_div(den);
            cpu.write(rd, val);
        }
        Instruction::Rem { rd, rs1, rs2 } => {
            let num = cpu.read(rs1) as i32;
            let den = cpu.read(rs2) as i32;
            if den == 0 {
                console.push_error("Division by zero");
                return false;
            }
            let val = num.wrapping_rem(den);
            cpu.write(rd, val as u32);
        }
        Instruction::Remu { rd, rs1, rs2 } => {
            let den = cpu.read(rs2);
            if den == 0 {
                console.push_error("Division by zero");
                return false;
            }
            let val = cpu.read(rs1).wrapping_rem(den);

            cpu.write(rd, val);
        }

        // I
        Instruction::Addi { rd, rs1, imm } => cpu.write(rd, cpu.read(rs1).wrapping_add(imm as u32)),
        Instruction::Andi { rd, rs1, imm } => cpu.write(rd, cpu.read(rs1) & (imm as u32)),
        Instruction::Ori { rd, rs1, imm } => cpu.write(rd, cpu.read(rs1) | (imm as u32)),
        Instruction::Xori { rd, rs1, imm } => cpu.write(rd, cpu.read(rs1) ^ (imm as u32)),
        Instruction::Slti { rd, rs1, imm } => {
            let v = (cpu.read(rs1) as i32) < imm;
            cpu.write(rd, v as u32);
        }
        Instruction::Sltiu { rd, rs1, imm } => {
            cpu.write(rd, (cpu.read(rs1) < imm as u32) as u32);
        }
        Instruction::Slli { rd, rs1, shamt } => cpu.write(rd, cpu.read(rs1) << (shamt & 0x1F)),
        Instruction::Srli { rd, rs1, shamt } => cpu.write(rd, cpu.read(rs1) >> (shamt & 0x1F)),
        Instruction::Srai { rd, rs1, shamt } => {
            cpu.write(rd, ((cpu.read(rs1) as i32) >> (shamt & 0x1F)) as u32)
        }

        Instruction::Lb { rd, rs1, imm } => {
            let a = cpu.read(rs1).wrapping_add(imm as u32);
            cpu.write(rd, (mem.load8(a) as i8 as i32) as u32);
        }
        Instruction::Lh { rd, rs1, imm } => {
            let a = cpu.read(rs1).wrapping_add(imm as u32);
            cpu.write(rd, (mem.load16(a) as i16 as i32) as u32);
        }
        Instruction::Lw { rd, rs1, imm } => {
            let a = cpu.read(rs1).wrapping_add(imm as u32);
            cpu.write(rd, mem.load32(a));
        }
        Instruction::Lbu { rd, rs1, imm } => {
            let a = cpu.read(rs1).wrapping_add(imm as u32);
            cpu.write(rd, mem.load8(a) as u32);
        }
        Instruction::Lhu { rd, rs1, imm } => {
            let a = cpu.read(rs1).wrapping_add(imm as u32);
            cpu.write(rd, mem.load16(a) as u32);
        }

        Instruction::Sb { rs2, rs1, imm } => {
            let a = cpu.read(rs1).wrapping_add(imm as u32);
            mem.store8(a, cpu.read(rs2) as u8);
        }
        Instruction::Sh { rs2, rs1, imm } => {
            let a = cpu.read(rs1).wrapping_add(imm as u32);
            mem.store16(a, cpu.read(rs2) as u16);
        }
        Instruction::Sw { rs2, rs1, imm } => {
            let a = cpu.read(rs1).wrapping_add(imm as u32);
            mem.store32(a, cpu.read(rs2));
        }

        // Branches (offset relative to the PC of the fetched instruction)
        Instruction::Beq { rs1, rs2, imm } if cpu.read(rs1) == cpu.read(rs2) => {
            cpu.pc = pc.wrapping_add(imm as u32)
        }
        Instruction::Bne { rs1, rs2, imm } if cpu.read(rs1) != cpu.read(rs2) => {
            cpu.pc = pc.wrapping_add(imm as u32)
        }
        Instruction::Blt { rs1, rs2, imm } if (cpu.read(rs1) as i32) < (cpu.read(rs2) as i32) => {
            cpu.pc = pc.wrapping_add(imm as u32)
        }
        Instruction::Bge { rs1, rs2, imm } if (cpu.read(rs1) as i32) >= (cpu.read(rs2) as i32) => {
            cpu.pc = pc.wrapping_add(imm as u32)
        }
        Instruction::Bltu { rs1, rs2, imm } if cpu.read(rs1) < cpu.read(rs2) => {
            cpu.pc = pc.wrapping_add(imm as u32)
        }
        Instruction::Bgeu { rs1, rs2, imm } if cpu.read(rs1) >= cpu.read(rs2) => {
            cpu.pc = pc.wrapping_add(imm as u32)
        }

        Instruction::Jal { rd, imm } => {
            cpu.write(rd, pc.wrapping_add(4));
            cpu.pc = pc.wrapping_add(imm as u32);
        }
        Instruction::Jalr { rd, rs1, imm } => {
            let target = (cpu.read(rs1).wrapping_add(imm as u32)) & !1;
            cpu.write(rd, pc.wrapping_add(4));
            cpu.pc = target;
        }
        Instruction::Lui { rd, imm } => cpu.write(rd, imm as u32),
        Instruction::Auipc { rd, imm } => cpu.write(rd, pc.wrapping_add(imm as u32)),

        Instruction::Ecall => {
            let old_pc = pc;
            let code = cpu.read(17);
            let cont = handle_syscall(code, cpu, mem, console);
            if !cont && console.reading {
                cpu.pc = old_pc;
                return false;
            }
            return cont;
        }
        Instruction::Halt => {
            console.push_error(format!("HALT at 0x{pc:08X}"));
            return false;
        }
        _ => {}
        }
        true

}

// em src/falcon/exec.rs (logo abaixo de `step`)
pub fn run<B: crate::falcon::memory::Bus>(
    cpu: &mut crate::falcon::registers::Cpu,
    mem: &mut B,
    console: &mut Console,
    max_steps: usize,
) -> usize {
    let mut steps = 0;
    while steps < max_steps && step(cpu, mem, console) {
        steps += 1;
    }
    steps
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::falcon::encoder;
    use crate::falcon::{Ram, instruction::Instruction};

    #[test]
    fn halt_halts() {
        let mut cpu = Cpu::default();
        let mut mem = Ram::new(4);
        let mut console = crate::ui::Console::default();
        let inst = encoder::encode(Instruction::Halt).unwrap();
        mem.store32(0, inst);
        assert!(!step(&mut cpu, &mut mem, &mut console));
    }

    #[test]
    fn sw_stores_word() {
        let mut cpu = Cpu::default();
        let mut mem = Ram::new(64);
        let mut console = crate::ui::Console::default();
        cpu.write(1, 0xDEADBEEF); // value to be stored
        cpu.write(2, 0x20); // base address
        let sw = encoder::encode(Instruction::Sw {
            rs2: 1,
            rs1: 2,
            imm: 0,
        })
        .unwrap();
        let halt = encoder::encode(Instruction::Halt).unwrap();
        mem.store32(0, sw);
        mem.store32(4, halt);
        assert!(step(&mut cpu, &mut mem, &mut console));
        assert_eq!(mem.load32(0x20), 0xDEADBEEF);
        assert!(!step(&mut cpu, &mut mem, &mut console));
    }

    #[test]
    fn syscall_print_int() {
        let mut cpu = Cpu::default();
        let mut mem = Ram::new(4);
        let mut console = crate::ui::Console::default();
        cpu.write(10, 42);
        cpu.write(17, 1);
        let inst = encoder::encode(Instruction::Ecall).unwrap();
        mem.store32(0, inst);
        assert!(step(&mut cpu, &mut mem, &mut console));
        assert_eq!(cpu.stdout, b"42");
    }

    #[test]
    fn syscall_print_string() {
        let mut cpu = Cpu::default();
        let mut mem = Ram::new(64);
        let mut console = crate::ui::Console::default();
        let addr = 8u32;
        let msg = b"hi\0";
        for (i, b) in msg.iter().enumerate() {
            mem.store8(addr + i as u32, *b);
        }
        cpu.write(10, addr);
        cpu.write(17, 2);
        let inst = encoder::encode(Instruction::Ecall).unwrap();
        mem.store32(0, inst);
        assert!(step(&mut cpu, &mut mem, &mut console));
        assert_eq!(cpu.stdout, b"hi");
    }

    #[test]
    fn syscall_read_string() {
        let mut cpu = Cpu::default();
        let mut mem = Ram::new(64);
        let mut console = crate::ui::Console::default();
        console.push_input("hi");
        let addr = 8u32;
        cpu.write(10, addr);
        cpu.write(17, 3);
        let inst = encoder::encode(Instruction::Ecall).unwrap();
        mem.store32(0, inst);

        assert!(step(&mut cpu, &mut mem, &mut console));
        assert_eq!(mem.load8(addr), b'h');
        assert_eq!(mem.load8(addr + 1), b'i');
        assert_eq!(mem.load8(addr + 2), 0);
    }

    #[test]
    fn syscall_read_waits_for_input() {
        let mut cpu = Cpu::default();
        let mut mem = Ram::new(64);
        let mut console = crate::ui::Console::default();
        let addr = 8u32;
        cpu.write(10, addr);
        cpu.write(17, 3);
        let ecall = encoder::encode(Instruction::Ecall).unwrap();
        let halt = encoder::encode(Instruction::Halt).unwrap();
        mem.store32(0, ecall);
        mem.store32(4, halt);

        assert!(!step(&mut cpu, &mut mem, &mut console));
        assert_eq!(cpu.pc, 0);

        console.push_input("hi");
        assert!(step(&mut cpu, &mut mem, &mut console));
        assert_eq!(cpu.pc, 4);
        assert_eq!(mem.load8(addr), b'h');
        assert_eq!(mem.load8(addr + 1), b'i');
        assert_eq!(mem.load8(addr + 2), 0);

        assert!(!step(&mut cpu, &mut mem, &mut console));
    }

}
