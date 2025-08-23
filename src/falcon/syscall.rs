use crate::falcon::{memory::Bus, registers::Cpu};

/// Emula syscalls simples baseadas em códigos em `a7`.
/// Retorna `true` se o código é reconhecido, `false` caso contrário.
pub fn handle_syscall<B: Bus>(code: u32, cpu: &mut Cpu, mem: &mut B) -> bool {
    match code {
        // 1: imprimir inteiro contido em a0
        1 => {
            let s = (cpu.read(10) as i32).to_string();
            cpu.stdout.extend_from_slice(s.as_bytes());
            true
        }
        // 2: imprimir string NUL-terminada apontada por a0
        2 => {
            let mut addr = cpu.read(10);
            loop {
                let b = mem.load8(addr);
                if b == 0 {
                    break;
                }
                cpu.stdout.push(b);
                addr = addr.wrapping_add(1);
            }
            true
        }
        // 3: ler inteiro de stdin e gravar em a0
        3 => {
            let mut i = 0;
            while i < cpu.stdin.len() && cpu.stdin[i].is_ascii_whitespace() {
                i += 1;
            }
            let start = i;
            while i < cpu.stdin.len() && cpu.stdin[i].is_ascii_digit() {
                i += 1;
            }
            let num_end = i;
            while i < cpu.stdin.len() && cpu.stdin[i].is_ascii_whitespace() {
                i += 1;
            }
            let token = cpu.stdin[start..num_end].to_vec();
            cpu.stdin.drain(..i);
            if let Ok(s) = std::str::from_utf8(&token) {
                if let Ok(v) = s.parse::<u32>() {
                    cpu.write(10, v);
                    return true;
                }
            }
            cpu.write(10, 0);
            true
        }
        _ => false,
    }
}

