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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disassemble_empty_data() {
        let result = disassemble_section(&[], 0x1000);
        assert!(result.all_instructions.is_empty());
        assert!(result.functions.is_empty());
    }

    #[test]
    fn test_disassemble_ret_only() {
        // c3 = ret (x86-64)
        let data = [0xc3u8];
        let result = disassemble_section(&data, 0x1000);
        assert_eq!(result.all_instructions.len(), 1);
        assert_eq!(result.all_instructions[0].mnemonic, "ret");
        assert_eq!(result.all_instructions[0].address, 0x1000);
        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].name, "sub_1000");
    }

    #[test]
    fn test_disassemble_nop_ret() {
        // 90 = nop, c3 = ret
        let data = [0x90u8, 0xc3];
        let result = disassemble_section(&data, 0x1000);
        assert_eq!(result.all_instructions.len(), 2);
        assert_eq!(result.all_instructions[0].mnemonic, "nop");
        assert_eq!(result.all_instructions[1].mnemonic, "ret");
        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].start_addr, 0x1000);
        assert_eq!(result.functions[0].instructions.len(), 2);
    }

    #[test]
    fn test_identify_multiple_functions() {
        // nop; ret; nop; nop; ret
        let data = [0x90u8, 0xc3, 0x90, 0x90, 0xc3];
        let result = disassemble_section(&data, 0x1000);
        assert_eq!(result.functions.len(), 2);
        assert_eq!(result.functions[0].name, "sub_1000");
        assert_eq!(result.functions[1].name, "sub_1002");
    }

    #[test]
    fn test_disassemble_instructions_have_bytes() {
        let data = [0x55u8, 0x48, 0x89, 0xe5, 0xc3];
        let result = disassemble_section(&data, 0x1000);
        for insn in &result.all_instructions {
            assert!(!insn.bytes.is_empty());
            assert!(insn.length > 0);
            assert!(!insn.mnemonic.is_empty());
        }
    }

    #[test]
    fn test_merge_symbols_empty_functions() {
        use crate::elf::parser::SymbolInfo;
        use crate::elf::parser::SymbolType;
        let symbols = vec![SymbolInfo {
            name: "main".into(),
            addr: 0x1000,
            size: 0,
            ty: SymbolType::Function,
            bind: "GLOBAL".into(),
            vis: "DEFAULT".into(),
            shndx: 0,
        }];
        let merged = merge_symbols_with_functions(&symbols, vec![]);
        assert!(merged.is_empty());
    }

    #[test]
    fn test_merge_symbols_renames_function() {
        use crate::elf::parser::SymbolInfo;
        use crate::elf::parser::SymbolType;
        let func = FunctionInfo {
            name: "sub_1000".into(),
            start_addr: 0x1000,
            end_addr: 0x1005,
            instructions: vec![],
        };
        let symbols = vec![SymbolInfo {
            name: "main".into(),
            addr: 0x1000,
            size: 0,
            ty: SymbolType::Function,
            bind: "GLOBAL".into(),
            vis: "DEFAULT".into(),
            shndx: 0,
        }];
        let merged = merge_symbols_with_functions(&symbols, vec![func]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name, "main");
    }
}