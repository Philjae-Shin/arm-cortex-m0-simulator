/// Simple two-pass assembler that translates text assembly into `Vec<Instruction>`.
///
/// Syntax overview
/// ---------------
///   ; line comment
///   .data <base_addr>
///       <label>: .word v1, v2, ...
///   .text
///       <label>:
///       OPCODE operands        ; inline comment
///
/// Registers: R0-R12, SP, LR, PC
/// Immediates: #decimal  or  #0xHEX

use std::collections::HashMap;
use crate::isa::*;
use crate::memory::Memory;

/// Result of assembling a source file.
pub struct Program {
    pub instructions: Vec<Instruction>,
    pub name: String,
}

/// Assemble `source` text into a program image, optionally pre-loading data
/// into `mem`.
pub fn assemble(source: &str, mem: &mut Memory) -> Result<Program, String> {
    let lines = preprocess(source);

    // Pass 1 — collect labels and data directives
    let mut labels: HashMap<String, i32> = HashMap::new();
    let mut instr_addr: i32 = 0;
    let mut in_data_section = false;
    let mut data_base: u32 = 0x1000; // default data base address

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }

        // Section directives
        if trimmed.starts_with(".data") {
            in_data_section = true;
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() > 1 {
                data_base = parse_number(parts[1])
                    .map_err(|_| format!("Bad .data address: {}", parts[1]))? as u32;
            }
            continue;
        }
        if trimmed.starts_with(".text") {
            in_data_section = false;
            continue;
        }

        if in_data_section {
            // Check for label on data
            if let Some(colon_pos) = trimmed.find(':') {
                let lbl = trimmed[..colon_pos].trim().to_uppercase();
                labels.insert(lbl, data_base as i32);
                // Parse .word values to advance data_base
                let after = trimmed[colon_pos + 1..].trim();
                if after.starts_with(".word") {
                    let vals_str = after[5..].trim();
                    let count = vals_str.split(',').count();
                    data_base += (count as u32) * 4;
                }
            }
            continue;
        }

        // Text section - labels and instructions
        if let Some(colon_pos) = trimmed.find(':') {
            let lbl = trimmed[..colon_pos].trim().to_uppercase();
            labels.insert(lbl, instr_addr);
            let after = trimmed[colon_pos + 1..].trim();
            if !after.is_empty() {
                instr_addr += 1;
            }
        } else {
            instr_addr += 1;
        }
    }

    // Pass 2 — assemble instructions and data
    let mut instructions: Vec<Instruction> = Vec::new();
    in_data_section = false;
    data_base = 0x1000;
    let mut pc: usize = 0;

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }

        if trimmed.starts_with(".data") {
            in_data_section = true;
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() > 1 {
                data_base = parse_number(parts[1]).unwrap_or(0x1000) as u32;
            }
            continue;
        }
        if trimmed.starts_with(".text") {
            in_data_section = false;
            continue;
        }

        if in_data_section {
            // Process data line
            let content = if let Some(colon_pos) = trimmed.find(':') {
                trimmed[colon_pos + 1..].trim()
            } else {
                trimmed
            };
            if content.starts_with(".word") {
                let vals_str = content[5..].trim();
                let words: Vec<u32> = vals_str
                    .split(',')
                    .filter_map(|s| parse_number(s.trim()).ok().map(|v| v as u32))
                    .collect();
                mem.load_data(data_base, &words);
                data_base += (words.len() as u32) * 4;
            }
            continue;
        }

        // Text section
        let instr_text = if let Some(colon_pos) = trimmed.find(':') {
            let after = trimmed[colon_pos + 1..].trim();
            if after.is_empty() { continue; }
            after
        } else {
            trimmed
        };

        let instr = parse_instruction(instr_text, pc, &labels)?;
        instructions.push(instr);
        pc += 1;
    }

    Ok(Program {
        instructions,
        name: String::from("program"),
    })
}

// Pre-processing
fn preprocess(source: &str) -> Vec<String> {
    source
        .lines()
        .map(|l| {
            // Strip comments (everything after ';')
            if let Some(pos) = l.find(';') {
                l[..pos].to_string()
            } else {
                l.to_string()
            }
        })
        .collect()
}

// Instruction parsing
fn parse_instruction(
    text: &str,
    pc: usize,
    labels: &HashMap<String, i32>,
) -> Result<Instruction, String> {
    let tokens = tokenize(text);
    if tokens.is_empty() {
        return Err("Empty instruction line".into());
    }

    let mnemonic = tokens[0].to_uppercase();
    let ops = &tokens[1..];
    let mut instr = Instruction::new(Opcode::Nop, pc);
    instr.text = text.to_string();

    match mnemonic.as_str() {
        // Arithmetic (3-register)
        "ADD" => {
            instr.opcode = Opcode::Add;
            instr.rd = Some(parse_reg(ops.get(0).ok_or("ADD: missing Rd")?)?);
            instr.rn = Some(parse_reg(ops.get(1).ok_or("ADD: missing Rn")?)?);
            instr.rm = Some(parse_reg(ops.get(2).ok_or("ADD: missing Rm")?)?);
        }
        "ADDI" => {
            instr.opcode = Opcode::AddI;
            instr.rd = Some(parse_reg(ops.get(0).ok_or("ADDI: missing Rd")?)?);
            instr.rn = Some(parse_reg(ops.get(1).ok_or("ADDI: missing Rn")?)?);
            instr.imm = parse_imm(ops.get(2).ok_or("ADDI: missing imm")?)?;
        }
        "SUB" => {
            instr.opcode = Opcode::Sub;
            instr.rd = Some(parse_reg(ops.get(0).ok_or("SUB: missing Rd")?)?);
            instr.rn = Some(parse_reg(ops.get(1).ok_or("SUB: missing Rn")?)?);
            instr.rm = Some(parse_reg(ops.get(2).ok_or("SUB: missing Rm")?)?);
        }
        "SUBI" => {
            instr.opcode = Opcode::SubI;
            instr.rd = Some(parse_reg(ops.get(0).ok_or("SUBI: missing Rd")?)?);
            instr.rn = Some(parse_reg(ops.get(1).ok_or("SUBI: missing Rn")?)?);
            instr.imm = parse_imm(ops.get(2).ok_or("SUBI: missing imm")?)?;
        }
        "MUL" => {
            instr.opcode = Opcode::Mul;
            instr.rd = Some(parse_reg(ops.get(0).ok_or("MUL: missing Rd")?)?);
            instr.rn = Some(parse_reg(ops.get(1).ok_or("MUL: missing Rn")?)?);
            instr.rm = Some(parse_reg(ops.get(2).ok_or("MUL: missing Rm")?)?);
        }

        // Compare
        "CMP" => {
            instr.opcode = Opcode::Cmp;
            instr.rn = Some(parse_reg(ops.get(0).ok_or("CMP: missing Rn")?)?);
            instr.rm = Some(parse_reg(ops.get(1).ok_or("CMP: missing Rm")?)?);
        }
        "CMPI" => {
            instr.opcode = Opcode::CmpI;
            instr.rn = Some(parse_reg(ops.get(0).ok_or("CMPI: missing Rn")?)?);
            instr.imm = parse_imm(ops.get(1).ok_or("CMPI: missing imm")?)?;
        }

        // Logical
        "AND" => {
            instr.opcode = Opcode::And;
            instr.rd = Some(parse_reg(ops.get(0).ok_or("AND: missing Rd")?)?);
            instr.rn = Some(parse_reg(ops.get(1).ok_or("AND: missing Rn")?)?);
            instr.rm = Some(parse_reg(ops.get(2).ok_or("AND: missing Rm")?)?);
        }
        "ORR" => {
            instr.opcode = Opcode::Orr;
            instr.rd = Some(parse_reg(ops.get(0).ok_or("ORR: missing Rd")?)?);
            instr.rn = Some(parse_reg(ops.get(1).ok_or("ORR: missing Rn")?)?);
            instr.rm = Some(parse_reg(ops.get(2).ok_or("ORR: missing Rm")?)?);
        }
        "EOR" => {
            instr.opcode = Opcode::Eor;
            instr.rd = Some(parse_reg(ops.get(0).ok_or("EOR: missing Rd")?)?);
            instr.rn = Some(parse_reg(ops.get(1).ok_or("EOR: missing Rn")?)?);
            instr.rm = Some(parse_reg(ops.get(2).ok_or("EOR: missing Rm")?)?);
        }
        "MVN" => {
            instr.opcode = Opcode::Mvn;
            instr.rd = Some(parse_reg(ops.get(0).ok_or("MVN: missing Rd")?)?);
            instr.rn = Some(parse_reg(ops.get(1).ok_or("MVN: missing Rn")?)?);
        }

        // Shifts
        "LSL" => {
            instr.opcode = Opcode::Lsl;
            instr.rd = Some(parse_reg(ops.get(0).ok_or("LSL: missing Rd")?)?);
            instr.rn = Some(parse_reg(ops.get(1).ok_or("LSL: missing Rn")?)?);
            instr.imm = parse_imm(ops.get(2).ok_or("LSL: missing imm")?)?;
        }
        "LSR" => {
            instr.opcode = Opcode::Lsr;
            instr.rd = Some(parse_reg(ops.get(0).ok_or("LSR: missing Rd")?)?);
            instr.rn = Some(parse_reg(ops.get(1).ok_or("LSR: missing Rn")?)?);
            instr.imm = parse_imm(ops.get(2).ok_or("LSR: missing imm")?)?;
        }
        "ASR" => {
            instr.opcode = Opcode::Asr;
            instr.rd = Some(parse_reg(ops.get(0).ok_or("ASR: missing Rd")?)?);
            instr.rn = Some(parse_reg(ops.get(1).ok_or("ASR: missing Rn")?)?);
            instr.imm = parse_imm(ops.get(2).ok_or("ASR: missing imm")?)?;
        }

        // Data movement
        "MOV" => {
            instr.opcode = Opcode::Mov;
            instr.rd = Some(parse_reg(ops.get(0).ok_or("MOV: missing Rd")?)?);
            instr.rn = Some(parse_reg(ops.get(1).ok_or("MOV: missing Rm")?)?);
        }
        "MOVI" => {
            instr.opcode = Opcode::MovI;
            instr.rd = Some(parse_reg(ops.get(0).ok_or("MOVI: missing Rd")?)?);
            instr.imm = parse_imm(ops.get(1).ok_or("MOVI: missing imm")?)?;
        }

        // Memory access
        "LDR" => {
            instr.opcode = Opcode::Ldr;
            instr.rd = Some(parse_reg(ops.get(0).ok_or("LDR: missing Rd")?)?);
            let (base, offset) = parse_mem_operand(
                ops.get(1).map(|s| s.as_str()).unwrap_or(""),
                ops.get(2).map(|s| s.as_str()),
                labels,
            )?;
            instr.rn = Some(base);
            instr.imm = offset;
        }
        "LDRB" => {
            instr.opcode = Opcode::LdrB;
            instr.rd = Some(parse_reg(ops.get(0).ok_or("LDRB: missing Rd")?)?);
            let (base, offset) = parse_mem_operand(
                ops.get(1).map(|s| s.as_str()).unwrap_or(""),
                ops.get(2).map(|s| s.as_str()),
                labels,
            )?;
            instr.rn = Some(base);
            instr.imm = offset;
        }
        "STR" => {
            instr.opcode = Opcode::Str;
            instr.rm = Some(parse_reg(ops.get(0).ok_or("STR: missing Rm")?)?);
            let (base, offset) = parse_mem_operand(
                ops.get(1).map(|s| s.as_str()).unwrap_or(""),
                ops.get(2).map(|s| s.as_str()),
                labels,
            )?;
            instr.rn = Some(base);
            instr.imm = offset;
        }
        "STRB" => {
            instr.opcode = Opcode::StrB;
            instr.rm = Some(parse_reg(ops.get(0).ok_or("STRB: missing Rm")?)?);
            let (base, offset) = parse_mem_operand(
                ops.get(1).map(|s| s.as_str()).unwrap_or(""),
                ops.get(2).map(|s| s.as_str()),
                labels,
            )?;
            instr.rn = Some(base);
            instr.imm = offset;
        }

        // Post-indexed memory access
        // Syntax: LDRPOST Rd, Rn, #offset  →  Rd = Mem[Rn]; Rn += offset
        "LDRPOST" => {
            instr.opcode = Opcode::LdrPost;
            instr.rd = Some(parse_reg(ops.get(0).ok_or("LDRPOST: missing Rd")?)?);
            instr.rn = Some(parse_reg(ops.get(1).ok_or("LDRPOST: missing Rn")?)?);
            instr.imm = parse_imm(ops.get(2).ok_or("LDRPOST: missing offset")?)?;
        }
        // Syntax: STRPOST Rm, Rn, #offset  then go  Mem[Rn] = Rm; Rn += offset
        "STRPOST" => {
            instr.opcode = Opcode::StrPost;
            instr.rm = Some(parse_reg(ops.get(0).ok_or("STRPOST: missing Rm")?)?);
            instr.rn = Some(parse_reg(ops.get(1).ok_or("STRPOST: missing Rn")?)?);
            instr.imm = parse_imm(ops.get(2).ok_or("STRPOST: missing offset")?)?;
        }

        // Branches
        "B"   => { instr.opcode = Opcode::B;   instr.imm = resolve_branch(ops, pc, labels)?; }
        "BEQ" => { instr.opcode = Opcode::Beq; instr.imm = resolve_branch(ops, pc, labels)?; }
        "BNE" => { instr.opcode = Opcode::Bne; instr.imm = resolve_branch(ops, pc, labels)?; }
        "BLT" => { instr.opcode = Opcode::Blt; instr.imm = resolve_branch(ops, pc, labels)?; }
        "BGE" => { instr.opcode = Opcode::Bge; instr.imm = resolve_branch(ops, pc, labels)?; }
        "BGT" => { instr.opcode = Opcode::Bgt; instr.imm = resolve_branch(ops, pc, labels)?; }
        "BLE" => { instr.opcode = Opcode::Ble; instr.imm = resolve_branch(ops, pc, labels)?; }
        "BL"  => { instr.opcode = Opcode::Bl;  instr.imm = resolve_branch(ops, pc, labels)?; }
        "BX"  => {
            instr.opcode = Opcode::Bx;
            instr.rm = Some(parse_reg(ops.get(0).ok_or("BX: missing Rm")?)?);
        }

        // Stack
        "PUSH" => {
            instr.opcode = Opcode::Push;
            instr.reg_list = parse_reg_list(ops)?;
        }
        "POP" => {
            instr.opcode = Opcode::Pop;
            instr.reg_list = parse_reg_list(ops)?;
        }

        // Vector (extension) — uses separate vector register file V0-V7
        // VLDR Vd, Rn, #len — load len words from Mem[Rn..] into Vd
        "VLDR" => {
            instr.opcode = Opcode::VLdr;
            instr.vd = Some(parse_vreg(ops.get(0).ok_or("VLDR: missing Vd")?)?);
            instr.rn = Some(parse_reg(ops.get(1).ok_or("VLDR: missing Rn")?)?);
            instr.imm = parse_imm(ops.get(2).ok_or("VLDR: missing #len")?)?;
        }
        // VSTR Vs, Rn, #len — store len words from Vs to Mem[Rn..]
        "VSTR" => {
            instr.opcode = Opcode::VStr;
            instr.vd = Some(parse_vreg(ops.get(0).ok_or("VSTR: missing Vs")?)?);
            instr.rn = Some(parse_reg(ops.get(1).ok_or("VSTR: missing Rn")?)?);
            instr.imm = parse_imm(ops.get(2).ok_or("VSTR: missing #len")?)?;
        }
        // VADD Vd, Vn, Vm, #len — element-wise addition
        "VADD" => {
            instr.opcode = Opcode::VAdd;
            instr.vd = Some(parse_vreg(ops.get(0).ok_or("VADD: missing Vd")?)?);
            instr.vn = Some(parse_vreg(ops.get(1).ok_or("VADD: missing Vn")?)?);
            instr.vm = Some(parse_vreg(ops.get(2).ok_or("VADD: missing Vm")?)?);
            instr.imm = parse_imm(ops.get(3).ok_or("VADD: missing #len")?)?;
        }
        "VSUB" => {
            instr.opcode = Opcode::VSub;
            instr.vd = Some(parse_vreg(ops.get(0).ok_or("VSUB: missing Vd")?)?);
            instr.vn = Some(parse_vreg(ops.get(1).ok_or("VSUB: missing Vn")?)?);
            instr.vm = Some(parse_vreg(ops.get(2).ok_or("VSUB: missing Vm")?)?);
            instr.imm = parse_imm(ops.get(3).ok_or("VSUB: missing #len")?)?;
        }
        "VMUL" => {
            instr.opcode = Opcode::VMul;
            instr.vd = Some(parse_vreg(ops.get(0).ok_or("VMUL: missing Vd")?)?);
            instr.vn = Some(parse_vreg(ops.get(1).ok_or("VMUL: missing Vn")?)?);
            instr.vm = Some(parse_vreg(ops.get(2).ok_or("VMUL: missing Vm")?)?);
            instr.imm = parse_imm(ops.get(3).ok_or("VMUL: missing #len")?)?;
        }
        // VDOT Rd, Vn, Vm, #len — dot product, scalar result to GP register
        "VDOT" => {
            instr.opcode = Opcode::VDot;
            instr.rd = Some(parse_reg(ops.get(0).ok_or("VDOT: missing Rd")?)?);
            instr.vn = Some(parse_vreg(ops.get(1).ok_or("VDOT: missing Vn")?)?);
            instr.vm = Some(parse_vreg(ops.get(2).ok_or("VDOT: missing Vm")?)?);
            instr.imm = parse_imm(ops.get(3).ok_or("VDOT: missing #len")?)?;
        }

        // Special
        "NOP"  => { instr.opcode = Opcode::Nop; }
        "HALT" => { instr.opcode = Opcode::Halt; }

        _ => return Err(format!("Unknown mnemonic: {}", mnemonic)),
    }

    Ok(instr)
}

// Token helpers
/// Split an instruction line into tokens, stripping commas and brackets.
fn tokenize(text: &str) -> Vec<String> {
    text.replace(',', " ")
        .replace('[', " ")
        .replace(']', " ")
        .replace('{', " ")
        .replace('}', " ")
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

/// Parse a register name like "R3", "SP", "LR", "PC".
fn parse_reg(s: &str) -> Result<usize, String> {
    let upper = s.to_uppercase();
    match upper.as_str() {
        "SP" => Ok(SP),
        "LR" => Ok(LR),
        "PC" => Ok(PC),
        _ if upper.starts_with('R') => {
            upper[1..].parse::<usize>()
                .map_err(|_| format!("Bad register: {}", s))
                .and_then(|n| {
                    if n < NUM_REGS { Ok(n) }
                    else { Err(format!("Register out of range: {}", s)) }
                })
        }
        _ => Err(format!("Expected register, got: {}", s)),
    }
}

/// Parse a vector register name like "V0".."V7".
fn parse_vreg(s: &str) -> Result<usize, String> {
    let upper = s.to_uppercase();
    if upper.starts_with('V') {
        upper[1..].parse::<usize>()
            .map_err(|_| format!("Bad vector register: {}", s))
            .and_then(|n| {
                if n < crate::isa::NUM_VREGS { Ok(n) }
                else { Err(format!("Vector register out of range (V0-V7): {}", s)) }
            })
    } else {
        Err(format!("Expected vector register (V0-V7), got: {}", s))
    }
}

/// Parse an immediate value: `#10`, `#-5`, `#0xFF`.
fn parse_imm(s: &str) -> Result<i32, String> {
    let s = s.trim().trim_start_matches('#');
    parse_number(s)
}

/// Parse a signed/unsigned integer from decimal or hex string.
fn parse_number(s: &str) -> Result<i32, String> {
    let s = s.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        u32::from_str_radix(&s[2..], 16)
            .map(|v| v as i32)
            .map_err(|e| format!("Bad hex number '{}': {}", s, e))
    } else if s.starts_with('-') {
        s.parse::<i32>()
            .map_err(|e| format!("Bad number '{}': {}", s, e))
    } else {
        s.parse::<i32>()
            .map_err(|e| format!("Bad number '{}': {}", s, e))
    }
}

/// Parse a memory operand like `[R1, #4]` — tokens already split by tokenize.
/// `first` might be "R1" or a label; `second` might be "#4" or None.
fn parse_mem_operand(
    first: &str,
    second: Option<&str>,
    labels: &HashMap<String, i32>,
) -> Result<(usize, i32), String> {
    // Check if first token is a label (used as absolute address via R0 trick)
    let upper = first.to_uppercase();
    if labels.contains_key(&upper) {
        // Label-based: synthesise as base=R<whatever is holding addr>, offset=0
        // For simplicity, we treat it as base=any, offset=label_addr
        // Caller should have loaded the label address into a register first.
        // Actually, let's try parsing as register first.
    }

    let base = parse_reg(first)?;
    let offset = match second {
        Some(s) => parse_imm(s)?,
        None => 0,
    };
    Ok((base, offset))
}

/// Resolve a branch target label to an absolute instruction address.
fn resolve_branch(
    ops: &[String],
    _pc: usize,
    labels: &HashMap<String, i32>,
) -> Result<i32, String> {
    let target = ops.get(0).ok_or("Branch: missing target")?;
    let upper = target.to_uppercase();
    if let Some(&addr) = labels.get(&upper) {
        Ok(addr)
    } else {
        // Try as numeric offset
        parse_imm(target)
    }
}

/// Parse a register list like `{R0, R1, R4, LR}` — already tokenized.
fn parse_reg_list(ops: &[String]) -> Result<Vec<usize>, String> {
    let mut regs = Vec::new();
    for tok in ops {
        regs.push(parse_reg(tok)?);
    }
    regs.sort();
    Ok(regs)
}

// Load assembly from file
pub fn load_from_file(
    path: &str,
    mem: &mut Memory,
) -> Result<Program, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read '{}': {}", path, e))?;
    let mut prog = assemble(&source, mem)?;
    // Extract filename as program name
    if let Some(name) = std::path::Path::new(path).file_stem() {
        prog.name = name.to_string_lossy().to_string();
    }
    Ok(prog)
}
