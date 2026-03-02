# FALCON-ASM — Roadmap de Compatibilidade RISC-V

Este arquivo documenta as lacunas entre o simulador atual e o padrão RISC-V
completo, ordenadas por valor acadêmico. Use como guia para decidir o que
implementar.

---

## Status atual (após v1.9.0)

| Recurso | Status |
|---|---|
| RV32I base (40 instruções) | ✅ Completo |
| M extension (mul/div) | ✅ Completo |
| `fence` (base RV32I) | ✅ Nop no simulador |
| `li` full 32-bit | ✅ lui+addi automático |
| Pseudos: bgt/ble/neg/not/seqz/snez etc. | ✅ Implementado |
| Pseudos: randomByte/randomBytes | ✅ Implementado |
| Zicsr (csrrs/csrrw/rdcycle/rdinstret) | ❌ |
| `.word label` (endereço em .data) | ❌ |
| RV32F (ponto flutuante) | ❌ |
| A extension (atomics) | ❌ |
| C extension (compressed) | ❌ |
| Níveis de privilégio M/S/U | ❌ |

---

## 1. `.word label` — endereços em seção .data

**Valor:** Alto — necessário para jump tables, vtables, ponteiros de função.

**Exemplo desejado:**
```asm
.data
  dispatch: .word handler_a, handler_b, handler_c
.text
handler_a:  ...
handler_b:  ...
```

**Por que não funciona hoje:** o assembler monta dados na 1ª passagem mas
labels de `.text` só são resolvidos depois. É um problema de forward reference.

**Como implementar:**
- Adicionar `DataItem::WordLabel(String)` junto com os itens de `.data`
- Após a 1ª passagem (quando todos os labels estão disponíveis), resolver esses
  itens e preencher os bytes com o endereço em little-endian 32-bit
- Alternativa mais invasiva: mover a geração de `data_bytes` para a 2ª passagem

**Esforço:** Médio (~150 linhas de mudança em `assembler.rs`)

---

## 2. Zicsr — registradores de controle e leitura de contadores

**Valor:** Alto — toda referência acadêmica que mede CPI usa `rdcycle`/`rdinstret`.
Sem isso o programa não tem acesso aos contadores que o simulador já computa.

**Instruções necessárias (spec Zicsr — 6 instruções):**
| Instrução | Operação |
|---|---|
| `csrrs rd, csr, rs1` | rd = CSR; CSR |= rs1 |
| `csrrw rd, csr, rs1` | rd = CSR; CSR = rs1 |
| `csrrc rd, csr, rs1` | rd = CSR; CSR &= ~rs1 |
| `csrrsi rd, csr, imm5` | variante imediata de csrrs |
| `csrrwi rd, csr, imm5` | variante imediata de csrrw |
| `csrrci rd, csr, imm5` | variante imediata de csrrc |

**Pseudos que se expandem a Zicsr:**
- `rdcycle rd`   → `csrrs rd, 0xC00, x0`
- `rdinstret rd` → `csrrs rd, 0xC02, x0`
- `rdtime rd`    → `csrrs rd, 0xC01, x0`
- `csrr rd, csr` → `csrrs rd, csr, x0`
- `csrw csr, rs` → `csrrw x0, csr, rs`

**CSRs mínimos a implementar (read-only user-level):**
| Endereço | Nome | Mapeamento no FALCON |
|---|---|---|
| 0xC00 | `cycle` | `mem.total_program_cycles()` |
| 0xC02 | `instret` | `mem.instruction_count` |
| 0xC01 | `time` | mesmo que `cycle` (simulador não tem relógio real) |

**Encoding:** opcode 0x73, funct3 0x1–0x3 e 0x5–0x7 (mesmo OPC_SYSTEM)

**Como implementar:**
1. Adicionar `Instruction::Csrrs/Csrrw/Csrrc` (+ variantes imediatas) a `instruction.rs`
2. Encoder: formato I-type, rs1=csr address nos bits 31:20
3. Decoder: em `itype::decode_system`, distinguir funct3 != 0 → CSR instructions
4. Exec: switch no endereço CSR, ler de `mem.total_program_cycles()` / `instruction_count`
5. Assembler: adicionar `csrrs`/`rdcycle`/etc. a `parse_instr`
6. `parse_instr` precisa aceitar nomes de CSR (`cycle`, `instret`) e converter para endereços

**Esforço:** Médio (~200 linhas, mas muito mecânico)

---

## 3. RV32F — extensão de ponto flutuante

**Valor:** Muito alto para uso acadêmico em álgebra linear, DSP, ML.
**Esforço:** Grande — maior trabalho individual do roadmap.

**O que precisa ser adicionado:**

### Registradores
- 32 registradores float `f0`–`f31` na struct `Cpu` (como `f: [f32; 32]`)
- `fcsr` (CSR 0x001/0x002/0x003): bits de exceção FP + modo de arredondamento
- ABI names: `ft0`–`ft11` (temporários), `fa0`–`fa7` (args), `fs0`–`fs11` (saved)

### Instruções (~30)
```
flw  rd, imm(rs1)     — load float
fsw  rs2, imm(rs1)    — store float
fadd.s/fsub.s/fmul.s/fdiv.s  rd, rs1, rs2
fsqrt.s   rd, rs1
fmin.s/fmax.s  rd, rs1, rs2
feq.s/flt.s/fle.s  rd, rs1, rs2   — comparação → int
fcvt.w.s / fcvt.wu.s  rd, rs1     — float → int
fcvt.s.w / fcvt.s.wu  rd, rs1     — int → float
fmv.x.w / fmv.w.x    rd, rs1      — move bits
fmadd.s / fmsub.s / fnmadd.s / fnmsub.s  (R4-type)
```

### Novo formato de encoding
- R4-type: `fmadd.s rd, rs1, rs2, rs3, rm` — usa bits 26:25 para rs3

### Detalhes de implementação
- Usar `f32` do Rust nativamente (IEEE 754 single — RISC-V usa o mesmo padrão)
- Tratar `NaN`, `±Inf`, underflow conforme a spec (Rust já faz isso)
- `fcsr.FRM` (rounding mode): para simplificar, implementar apenas `RNE` (round to nearest even, o padrão)
- Opcodes novos: 0x07 (FLW), 0x27 (FSW), 0x43–0x4F (fmadd/fmsub/fnmadd/fnmsub/FP-OP)

### O que NÃO precisa agora
- RV32D (double) — pode ser ignorado
- Tratamento de exceções FP via trap
- Todos os 5 modos de arredondamento (só RNE é necessário para 95% dos usos)

**Esforço:** Grande (~500–700 linhas em instruction.rs + encoder + decoder + exec + assembler + UI)

---

## 4. A extension — operações atômicas

**Valor:** Médio — necessário para cursos de sistemas concorrentes.
**Relevante apenas se o simulador ganhar múltiplos harts (threads de hardware).**

**Instruções:**
```
lr.w  rd, (rs1)              — load-reserved
sc.w  rd, rs2, (rs1)         — store-conditional
amoswap.w / amoadd.w / amoor.w / amoxor.w / amoand.w
amomin.w  / amomax.w  / amominu.w / amomaxu.w  rd, rs2, (rs1)
```

**Implementação em single-core (simplificada):**
- `lr.w`: load normal + marcar endereço como reservado (`reservation: Option<u32>` na CPU)
- `sc.w`: se reserva válida → store + rd=0; senão → rd=1 (falha)
- AMOs: read-modify-write atômico (em single-core é sempre atômico)

**Encoding:** opcode 0x2F (47)

**Esforço:** Pequeno para single-core (~100 linhas)

---

## 5. Níveis de privilégio (M-mode mínimo)

**Valor:** Necessário para cursos de SO / kernels bare-metal.

**O que precisaria:**
- Campo `mode: PrivMode` na CPU (`Machine | Supervisor | User`)
- CSRs de máquina: `mstatus`, `mtvec`, `mepc`, `mcause`, `mscratch`, `mie`, `mip`
- Instrução `mret` (retorno de trap)
- Mecanismo de trap: exceções (instrução ilegal, acesso inválido) + interrupções de timer
- Lógica de delegação M→S

**Esforço:** Grande (~400 linhas + redesenho do exec loop)

---

## 6. C extension — instruções comprimidas (16-bit)

**Valor:** Baixo para simulação didática. Alto para análise de densidade de código.

Cada instrução comprimida mapeia 1:1 para uma instrução base de 32 bits.
O desafio é o decoder: misturar palavras de 16 e 32 bits na mesma stream de instruções
(detectado via bits[1:0] — se `11`, 32-bit; caso contrário, 16-bit).

**Esforço:** Médio (~300 linhas no decoder/encoder) mas baixo valor educacional

---

## Sugestão de ordem de implementação

```
Próximo passo recomendado:
  1. .word label         — corrige bug irritante, esforço baixo
  2. Zicsr mínimo        — habilita benchmarking no próprio código Assembly
  3. RV32F               — maior salto de valor acadêmico
  4. A extension         — para labs de concorrência (opcional)
  5. Privilege M-mode    — para cursos de SO (opcional, grande esforço)
```

---

## Referências

- RISC-V Unprivileged Spec: https://riscv.org/technical/specifications/
- RISC-V Privileged Spec: https://riscv.org/technical/specifications/
- ABI calling conventions: https://github.com/riscv-non-isa/riscv-elf-psabi-doc
- Simuladores de referência: Spike (oficial), QEMU, Venus (web), RARS (educational)
