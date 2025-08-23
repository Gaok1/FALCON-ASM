use crate::{
    falcon::{memory::Bus, registers::Cpu},
    ui::Console,
};

/// Emula syscalls simples baseadas em códigos em `a7`.
/// Retorna `true` se o código é reconhecido, `false` caso contrário.
pub fn handle_syscall<B: Bus>(
    code: u32,
    cpu: &mut Cpu,
    mem: &mut B,
    console: &mut Console,
) -> bool {
    match code {
        // 1: imprimir inteiro contido em a0
        1 => {
            let s = (cpu.read(10) as i32).to_string();
            cpu.stdout.extend_from_slice(s.as_bytes());
            console.push_line(s);
            true
        }
        // 2: imprimir string NUL-terminada apontada por a0
        2 => {
            let mut addr = cpu.read(10);
            let mut bytes = Vec::new();
            loop {
                let b = mem.load8(addr);
                if b == 0 {
                    break;
                }
                cpu.stdout.push(b);
                bytes.push(b);
                addr = addr.wrapping_add(1);
            }
            if let Ok(s) = std::str::from_utf8(&bytes) {
                console.push_line(s.to_string());
            }
            true
        }
        // 3: ler string de stdin e gravar na memória apontada por a0
        3 => {
            let mut addr = cpu.read(10);
            if let Some(line) = console.read_line() {
                for b in line.as_bytes() {
                    mem.store8(addr, *b);
                    addr = addr.wrapping_add(1);
                }
                mem.store8(addr, 0); // NUL
                                     // Input has been consumed; stop requesting console input
                console.reading = false;
                true
            } else {
                console.reading = true;
                false
            }
        }
        _ => {
            console.push_error(format!("Unknown syscall code {code}"));
            false
        }
    }
}
