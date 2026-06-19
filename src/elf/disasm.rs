use iced_x86::{Decoder, DecoderOptions, Formatter, Instruction, IntelFormatter};

#[derive(Debug, Clone)]
pub struct DisasmInstruction {
    pub address: u64,
    pub offset: usize,
    pub length: usize,
    pub bytes: Vec<u8>,
    pub mnemonic: String,
    pub operands: String,
}

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub start_addr: u64,
    pub end_addr: u64,
    pub instructions: Vec<DisasmInstruction>,
}

#[derive(Debug, Clone)]
pub struct DisasmResult {
    pub functions: Vec<FunctionInfo>,
    pub all_instructions: Vec<DisasmInstruction>,
    pub bitness: u32,
}

pub fn disassemble_section(data: &[u8], base_addr: u64) -> DisasmResult {
    let bitness = 64;
    let mut decoder = Decoder::with_ip(bitness, data, base_addr, DecoderOptions::NONE);

    let mut all_instructions = Vec::new();
    let mut formatter = IntelFormatter::new();

    let mut offset = 0usize;
    while decoder.can_decode() {
        let mut instruction = Instruction::default();
        decoder.decode_out(&mut instruction);

        let mut output = String::new();
        formatter.format_mnemonic(&instruction, &mut output);
        let mnemonic = output.trim().to_string();

        output.clear();
        formatter.format_all_operands(&instruction, &mut output);
        let operands = output.trim().to_string();

        let len = instruction.len();
        let bytes = data[offset..offset + len].to_vec();

        all_instructions.push(DisasmInstruction {
            address: instruction.ip(),
            offset,
            length: len,
            bytes,
            mnemonic,
            operands,
        });

        offset += len;
    }

    let functions = identify_functions(&all_instructions);

    DisasmResult {
        functions,
        all_instructions,
        bitness,
    }
}

fn identify_functions(instructions: &[DisasmInstruction]) -> Vec<FunctionInfo> {
    if instructions.is_empty() {
        return Vec::new();
    }

    let mut functions = Vec::new();
    let mut func_start = instructions[0].address;
    let mut func_insns = Vec::new();

    for insn in instructions {
        let is_call = insn.mnemonic == "call";
        let is_ret = insn.mnemonic == "ret"
            || insn.mnemonic == "retf"
            || insn.mnemonic == "iret"
            || insn.mnemonic == "iretd";

        func_insns.push(insn.clone());

        if is_ret {
            functions.push(FunctionInfo {
                name: format!("sub_{:x}", func_start),
                start_addr: func_start,
                end_addr: insn.address + insn.length as u64,
                instructions: func_insns.clone(),
            });
            func_insns.clear();
            func_start = insn.address + insn.length as u64;
        }
    }

    if !func_insns.is_empty() {
        functions.push(FunctionInfo {
            name: format!("sub_{:x}", func_start),
            start_addr: func_start,
            end_addr: func_insns.last().unwrap().address + func_insns.last().unwrap().length as u64,
            instructions: func_insns,
        });
    }

    functions
}

pub fn merge_symbols_with_functions(
    symbols: &[crate::elf::parser::SymbolInfo],
    mut functions: Vec<FunctionInfo>,
) -> Vec<FunctionInfo> {
    for sym in symbols {
        if !matches!(sym.ty, crate::elf::parser::SymbolType::Function) {
            continue;
        }

        let mut found = false;
        for func in &mut functions {
            if func.start_addr == sym.addr || func.name == sym.name {
                func.name = sym.name.clone();
                found = true;
                break;
            }
        }

        if found {
            continue;
        }

        let insns = functions
            .iter()
            .flat_map(|f| f.instructions.iter())
            .filter(|i| i.address >= sym.addr)
            .cloned()
            .collect::<Vec<_>>();

        if !insns.is_empty() {
            functions.push(FunctionInfo {
                name: sym.name.clone(),
                start_addr: sym.addr,
                end_addr: insns.last().unwrap().address + insns.last().unwrap().length as u64,
                instructions: insns,
            });
        }
    }

    functions.sort_by_key(|f| f.start_addr);
    functions
}