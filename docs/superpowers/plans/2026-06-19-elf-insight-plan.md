# ELF Insight Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a TUI ELF file viewer with tree navigation, hexdump, disassembly, and search capabilities.

**Architecture:** Ratatui app with left-right split layout. Left panel is a tree widget (Overview, Sections, Segments, Symbols). Right panel switches views based on selection. Core logic (ELF parsing via goblin, disassembly via iced-x86) is decoupled from UI rendering.

**Tech Stack:** Rust, Ratatui 0.29, Crossterm 0.28, goblin 0.9, iced-x86 1.21

---

## File Structure

```
elf-insight/
├── Cargo.toml
├── src/
│   ├── main.rs           # Entry, CLI arg parsing, error handling
│   ├── app.rs            # App state machine, event dispatch, view routing
│   ├── elf/
│   │   ├── mod.rs        # Re-exports
│   │   ├── parser.rs     # goblin wrapper, ElfData internal structs
│   │   └── disasm.rs     # iced-x86 wrapper, DisasmResult + function detection
│   └── ui/
│       ├── mod.rs        # Main layout render (left/right split)
│       ├── tree.rs       # Left panel tree navigation
│       ├── overview.rs   # Overview view (readelf -WSlh style)
│       ├── info.rs       # Structured info view (field tables)
│       ├── hexdump.rs    # Hexdump view with cursor
│       ├── disasm.rs     # Disassembly view with function list
│       ├── strings.rs    # String table view
│       └── search.rs     # Search bar widget + search logic
```

---

### Task 1: Project Scaffolding

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/elf/mod.rs`
- Create: `src/ui/mod.rs`
- Create: `src/app.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "elf-insight"
version = "0.1.0"
edition = "2021"

[dependencies]
ratatui = "0.29"
crossterm = "0.28"
goblin = "0.9"
iced-x86 = "1.21"
```

- [ ] **Step 2: Create minimal src/main.rs**

```rust
mod app;
mod elf;
mod ui;

use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: elf-insight <elf-file>");
        process::exit(1);
    }
    let file_path = &args[1];

    let data = elf::parser::parse_elf(file_path)
        .unwrap_or_else(|e| {
            eprintln!("Failed to parse ELF file: {}", e);
            process::exit(1);
        });

    app::run_app(data).unwrap_or_else(|e| {
        eprintln!("Application error: {}", e);
        process::exit(1);
    });
}
```

- [ ] **Step 3: Create placeholder module files**

`src/elf/mod.rs`:
```rust
pub mod parser;
pub mod disasm;
```

`src/ui/mod.rs`:
```rust
pub mod tree;
pub mod overview;
pub mod info;
pub mod hexdump;
pub mod disasm;
pub mod strings;
pub mod search;
```

`src/app.rs`:
```rust
use crate::elf::parser::ElfData;

pub fn run_app(_data: ElfData) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
```

- [ ] **Step 4: Verify it compiles (with placeholder types)**

Run: `cargo check`
Expected: FAIL — `ElfData` not defined yet. This is expected; we'll fix in Task 2.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: scaffold project with Cargo.toml and module structure"
```

---

### Task 2: ELF Parser

**Files:**
- Create: `src/elf/parser.rs`

- [ ] **Step 1: Write ElfData struct and parse_elf function**

```rust
use goblin::elf::Elf;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct SectionInfo {
    pub name: String,
    pub index: usize,
    pub addr: u64,
    pub offset: u64,
    pub size: u64,
    pub ty: String,
    pub flags: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct SegmentInfo {
    pub index: usize,
    pub ty: String,
    pub offset: u64,
    pub vaddr: u64,
    pub paddr: u64,
    pub filesz: u64,
    pub memsz: u64,
    pub flags: String,
    pub align: u64,
}

#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub addr: u64,
    pub size: u64,
    pub ty: SymbolType,
    pub bind: String,
    pub vis: String,
    pub shndx: u16,
}

#[derive(Debug, Clone)]
pub enum SymbolType {
    Function,
    Object,
    Section,
    File,
    Other(u8),
}

#[derive(Debug, Clone)]
pub struct ElfData {
    pub file_path: String,
    pub raw_bytes: Vec<u8>,
    pub class: u8,       // 1 = 32-bit, 2 = 64-bit
    pub data: u8,        // 1 = little, 2 = big
    pub os_abi: String,
    pub abi_version: u8,
    pub elf_type: String,
    pub machine: String,
    pub version: u32,
    pub entry: u64,
    pub phoff: u64,
    pub shoff: u64,
    pub flags: u32,
    pub ehsize: u16,
    pub phentsize: u16,
    pub phnum: u16,
    pub shentsize: u16,
    pub shnum: u16,
    pub shstrndx: u16,
    pub sections: Vec<SectionInfo>,
    pub segments: Vec<SegmentInfo>,
    pub symbols: Vec<SymbolInfo>,
}

pub fn parse_elf<P: AsRef<Path>>(path: P) -> Result<ElfData, Box<dyn std::error::Error>> {
    let path = path.as_ref();
    let raw_bytes = fs::read(path)?;
    let elf = Elf::parse(&raw_bytes)?;

    let shstrtab_offset = elf.header.e_shstrndx as usize;

    let sections: Vec<SectionInfo> = elf
        .section_headers
        .iter()
        .enumerate()
        .map(|(i, sh)| {
            let name = elf
                .shdr_strtab
                .get_at(sh.sh_name)
                .unwrap_or("")
                .to_string();
            let data = if sh.sh_type == goblin::elf::section_header::SHT_NOBITS {
                Vec::new()
            } else {
                let start = sh.sh_offset as usize;
                let end = start + sh.sh_size as usize;
                if end <= raw_bytes.len() {
                    raw_bytes[start..end].to_vec()
                } else {
                    Vec::new()
                }
            };
            SectionInfo {
                name,
                index: i,
                addr: sh.sh_addr,
                offset: sh.sh_offset,
                size: sh.sh_size,
                ty: section_type_to_str(sh.sh_type),
                flags: section_flags_to_str(sh.sh_flags),
                data,
            }
        })
        .collect();

    let segments: Vec<SegmentInfo> = elf
        .program_headers
        .iter()
        .enumerate()
        .map(|(i, ph)| SegmentInfo {
            index: i,
            ty: segment_type_to_str(ph.p_type),
            offset: ph.p_offset,
            vaddr: ph.p_vaddr,
            paddr: ph.p_paddr,
            filesz: ph.p_filesz,
            memsz: ph.p_memsz,
            flags: segment_flags_to_str(ph.p_flags),
            align: ph.p_align,
        })
        .collect();

    let symbols: Vec<SymbolInfo> = elf
        .syms
        .iter()
        .filter_map(|sym| {
            let name = elf.strtab.get_at(sym.st_name).unwrap_or("").to_string();
            if name.is_empty() {
                return None;
            }
            let ty = match goblin::elf::sym::st_type(sym.st_info) {
                goblin::elf::sym::STT_FUNC => SymbolType::Function,
                goblin::elf::sym::STT_OBJECT => SymbolType::Object,
                goblin::elf::sym::STT_SECTION => SymbolType::Section,
                goblin::elf::sym::STT_FILE => SymbolType::File,
                t => SymbolType::Other(t),
            };
            let bind = sym_bind_to_str(goblin::elf::sym::st_bind(sym.st_info));
            let vis = sym_vis_to_str(goblin::elf::sym::st_visibility(sym.st_other));
            Some(SymbolInfo {
                name,
                addr: sym.st_value,
                size: sym.st_size,
                ty,
                bind,
                vis,
                shndx: sym.st_shndx,
            })
        })
        .collect();

    Ok(ElfData {
        file_path: path.to_string_lossy().to_string(),
        raw_bytes,
        class: elf.header.e_ident[goblin::elf::header::EI_CLASS],
        data: elf.header.e_ident[goblin::elf::header::EI_DATA],
        os_abi: os_abi_to_str(elf.header.e_ident[goblin::elf::header::EI_OSABI]),
        abi_version: elf.header.e_ident[goblin::elf::header::EI_ABIVERSION],
        elf_type: elf_type_to_str(elf.header.e_type),
        machine: machine_to_str(elf.header.e_machine),
        version: elf.header.e_version,
        entry: elf.header.e_entry,
        phoff: elf.header.e_phoff,
        shoff: elf.header.e_shoff,
        flags: elf.header.e_flags,
        ehsize: elf.header.e_ehsize,
        phentsize: elf.header.e_phentsize,
        phnum: elf.header.e_phnum,
        shentsize: elf.header.e_shentsize,
        shnum: elf.header.e_shnum,
        shstrndx: elf.header.e_shstrndx,
        sections,
        segments,
        symbols,
    })
}

fn elf_type_to_str(et: u16) -> String {
    match et {
        goblin::elf::header::ET_NONE => "NONE".into(),
        goblin::elf::header::ET_REL => "REL (Relocatable)".into(),
        goblin::elf::header::ET_EXEC => "EXEC (Executable)".into(),
        goblin::elf::header::ET_DYN => "DYN (Shared object)".into(),
        goblin::elf::header::ET_CORE => "CORE".into(),
        _ => format!("0x{:x}", et),
    }
}

fn machine_to_str(m: u16) -> String {
    match m {
        goblin::elf::header::EM_386 => "Intel 80386".into(),
        goblin::elf::header::EM_X86_64 => "AMD x86-64".into(),
        goblin::elf::header::EM_ARM => "ARM".into(),
        goblin::elf::header::EM_AARCH64 => "AArch64".into(),
        _ => format!("0x{:x}", m),
    }
}

fn os_abi_to_str(abi: u8) -> String {
    match abi {
        goblin::elf::header::ELFOSABI_NONE => "UNIX - System V".into(),
        goblin::elf::header::ELFOSABI_LINUX => "UNIX - Linux".into(),
        goblin::elf::header::ELFOSABI_SOLARIS => "UNIX - Solaris".into(),
        goblin::elf::header::ELFOSABI_FREEBSD => "UNIX - FreeBSD".into(),
        _ => format!("0x{:x}", abi),
    }
}

fn section_type_to_str(t: u32) -> String {
    match t {
        goblin::elf::section_header::SHT_NULL => "NULL".into(),
        goblin::elf::section_header::SHT_PROGBITS => "PROGBITS".into(),
        goblin::elf::section_header::SHT_SYMTAB => "SYMTAB".into(),
        goblin::elf::section_header::SHT_STRTAB => "STRTAB".into(),
        goblin::elf::section_header::SHT_RELA => "RELA".into(),
        goblin::elf::section_header::SHT_HASH => "HASH".into(),
        goblin::elf::section_header::SHT_DYNAMIC => "DYNAMIC".into(),
        goblin::elf::section_header::SHT_NOTE => "NOTE".into(),
        goblin::elf::section_header::SHT_NOBITS => "NOBITS".into(),
        goblin::elf::section_header::SHT_REL => "REL".into(),
        goblin::elf::section_header::SHT_DYNSYM => "DYNSYM".into(),
        goblin::elf::section_header::SHT_GNU_HASH => "GNU_HASH".into(),
        goblin::elf::section_header::SHT_GNU_VERSYM => "VERSYM".into(),
        goblin::elf::section_header::SHT_GNU_VERNEED => "VERNEED".into(),
        goblin::elf::section_header::SHT_GNU_VERDEF => "VERDEF".into(),
        goblin::elf::section_header::SHT_INIT_ARRAY => "INIT_ARRAY".into(),
        goblin::elf::section_header::SHT_FINI_ARRAY => "FINI_ARRAY".into(),
        _ => format!("0x{:x}", t),
    }
}

fn section_flags_to_str(flags: u64) -> String {
    let mut parts = Vec::new();
    if flags & goblin::elf::section_header::SHF_WRITE as u64 != 0 {
        parts.push("W");
    }
    if flags & goblin::elf::section_header::SHF_ALLOC as u64 != 0 {
        parts.push("A");
    }
    if flags & goblin::elf::section_header::SHF_EXECINSTR as u64 != 0 {
        parts.push("X");
    }
    if parts.is_empty() {
        "-".into()
    } else {
        parts.join("")
    }
}

fn segment_type_to_str(t: u32) -> String {
    match t {
        goblin::elf::program_header::PT_NULL => "NULL".into(),
        goblin::elf::program_header::PT_LOAD => "LOAD".into(),
        goblin::elf::program_header::PT_DYNAMIC => "DYNAMIC".into(),
        goblin::elf::program_header::PT_INTERP => "INTERP".into(),
        goblin::elf::program_header::PT_NOTE => "NOTE".into(),
        goblin::elf::program_header::PT_PHDR => "PHDR".into(),
        goblin::elf::program_header::PT_TLS => "TLS".into(),
        goblin::elf::program_header::PT_GNU_EH_FRAME => "GNU_EH_FRAME".into(),
        goblin::elf::program_header::PT_GNU_STACK => "GNU_STACK".into(),
        goblin::elf::program_header::PT_GNU_RELRO => "GNU_RELRO".into(),
        _ => format!("0x{:x}", t),
    }
}

fn segment_flags_to_str(flags: u32) -> String {
    let mut parts = Vec::new();
    if flags & goblin::elf::program_header::PF_R != 0 {
        parts.push("R");
    }
    if flags & goblin::elf::program_header::PF_W != 0 {
        parts.push("W");
    }
    if flags & goblin::elf::program_header::PF_X != 0 {
        parts.push("E");
    }
    parts.join("")
}

fn sym_bind_to_str(bind: u8) -> String {
    match bind {
        goblin::elf::sym::STB_LOCAL => "LOCAL".into(),
        goblin::elf::sym::STB_GLOBAL => "GLOBAL".into(),
        goblin::elf::sym::STB_WEAK => "WEAK".into(),
        goblin::elf::sym::STB_GNU_UNIQUE => "GNU_UNIQUE".into(),
        _ => format!("0x{:x}", bind),
    }
}

fn sym_vis_to_str(vis: u8) -> String {
    match vis {
        goblin::elf::sym::STV_DEFAULT => "DEFAULT".into(),
        goblin::elf::sym::STV_INTERNAL => "INTERNAL".into(),
        goblin::elf::sym::STV_HIDDEN => "HIDDEN".into(),
        goblin::elf::sym::STV_PROTECTED => "PROTECTED".into(),
        _ => format!("0x{:x}", vis),
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: PASS (no errors)

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: add ELF parser with goblin wrapper"
```

---

### Task 3: Disassembler

**Files:**
- Create: `src/elf/disasm.rs`

- [ ] **Step 1: Write disassembler module**

```rust
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
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: add disassembler with iced-x86 wrapper"
```

---

### Task 4: App State and Event Loop

**Files:**
- Overwrite: `src/app.rs`
- Create: `src/ui/tree.rs`
- Create: `src/ui/overview.rs`
- Create: `src/ui/info.rs`
- Create: `src/ui/hexdump.rs`
- Create: `src/ui/disasm.rs`
- Create: `src/ui/strings.rs`
- Create: `src/ui/search.rs`

- [ ] **Step 1: Write the tree node type and state**

`src/ui/tree.rs`:
```rust
use ratatui::widgets::ListState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeNodeType {
    Overview,
    ElfHeader,
    SectionsGroup,
    SectionHeader { index: usize },
    SectionBody { index: usize },
    SegmentsGroup,
    Segment { index: usize },
    SymbolsGroup,
    Symbol { index: usize },
}

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub label: String,
    pub node_type: TreeNodeType,
    pub depth: u8,
    pub children: Vec<TreeNode>,
    pub expanded: bool,
}

#[derive(Debug, Clone)]
pub struct TreeState {
    pub nodes: Vec<TreeNode>,
    pub flat_list: Vec<usize>,
    pub list_state: ListState,
    pub selected_node: Option<TreeNodeType>,
}

impl TreeState {
    pub fn new(nodes: Vec<TreeNode>) -> Self {
        let mut state = TreeState {
            nodes,
            flat_list: Vec::new(),
            list_state: ListState::default(),
            selected_node: None,
        };
        state.rebuild_flat_list();
        state.list_state.select(Some(0));
        state.selected_node = state.node_at_index(0);
        state
    }

    pub fn rebuild_flat_list(&mut self) {
        self.flat_list.clear();
        Self::flatten(&self.nodes, &mut self.flat_list);
    }

    fn flatten(nodes: &[TreeNode], result: &mut Vec<usize>) {
        for (i, node) in nodes.iter().enumerate() {
            result.push(i);
            if node.expanded {
                Self::flatten(&node.children, result);
            }
        }
    }

    pub fn node_at_index(&self, idx: usize) -> Option<TreeNodeType> {
        let mut count = 0;
        Self::find_node(&self.nodes, idx, &mut count)
    }

    fn find_node(nodes: &[TreeNode], target: usize, count: &mut usize) -> Option<TreeNodeType> {
        for node in nodes {
            if *count == target {
                return Some(node.node_type.clone());
            }
            *count += 1;
            if node.expanded {
                if let Some(t) = Self::find_node(&node.children, target, count) {
                    return Some(t);
                }
            }
        }
        None
    }

    pub fn move_up(&mut self) {
        let idx = self.list_state.selected().unwrap_or(0);
        if idx > 0 {
            self.list_state.select(Some(idx - 1));
            self.selected_node = self.node_at_index(idx - 1);
        }
    }

    pub fn move_down(&mut self) {
        let idx = self.list_state.selected().unwrap_or(0);
        if idx + 1 < self.flat_list.len() {
            self.list_state.select(Some(idx + 1));
            self.selected_node = self.node_at_index(idx + 1);
        }
    }

    pub fn toggle_expand(&mut self) {
        let idx = self.list_state.selected().unwrap_or(0);
        if let Some(node) = self.node_at_flat_index(idx) {
            if !node.children.is_empty() {
                node.expanded = !node.expanded;
                self.rebuild_flat_list();
                self.list_state.select(Some(idx));
                self.selected_node = self.node_at_index(idx);
            }
        }
    }

    fn node_at_flat_index(&mut self, target: usize) -> Option<&mut TreeNode> {
        let mut count = 0;
        Self::find_node_mut(&mut self.nodes, target, &mut count)
    }

    fn find_node_mut<'a>(
        nodes: &'a mut [TreeNode],
        target: usize,
        count: &mut usize,
    ) -> Option<&'a mut TreeNode> {
        for node in nodes.iter_mut() {
            if *count == target {
                return Some(node);
            }
            *count += 1;
            if node.expanded {
                if let Some(n) = Self::find_node_mut(&mut node.children, target, count) {
                    return Some(n.try_into().ok());
                }
            }
        }
        None
    }
}
```

- [ ] **Step 2: Write placeholder UI modules**

`src/ui/overview.rs`:
```rust
pub struct OverviewState {
    pub scroll: usize,
}

impl OverviewState {
    pub fn new() -> Self {
        OverviewState { scroll: 0 }
    }
}
```

`src/ui/info.rs`:
```rust
pub struct InfoState {
    pub scroll: usize,
}

impl InfoState {
    pub fn new() -> Self {
        InfoState { scroll: 0 }
    }
}
```

`src/ui/hexdump.rs`:
```rust
pub enum HexCursor {
    Hex,
    Ascii,
}

pub struct HexdumpState {
    pub scroll: usize,
    pub cursor_offset: usize,
    pub cursor_mode: HexCursor,
    pub goto_input: String,
    pub goto_mode: bool,
}

impl HexdumpState {
    pub fn new() -> Self {
        HexdumpState {
            scroll: 0,
            cursor_offset: 0,
            cursor_mode: HexCursor::Hex,
            goto_input: String::new(),
            goto_mode: false,
        }
    }
}
```

`src/ui/disasm.rs`:
```rust
pub struct DisasmState {
    pub selected_function: usize,
    pub scroll: usize,
}

impl DisasmState {
    pub fn new() -> Self {
        DisasmState {
            selected_function: 0,
            scroll: 0,
        }
    }
}
```

`src/ui/strings.rs`:
```rust
pub struct StringsState {
    pub scroll: usize,
}

impl StringsState {
    pub fn new() -> Self {
        StringsState { scroll: 0 }
    }
}
```

`src/ui/search.rs`:
```rust
pub struct SearchState {
    pub active: bool,
    pub input: String,
    pub results: Vec<usize>,
    pub current_result: usize,
    pub no_matches_timer: u8,
}

impl SearchState {
    pub fn new() -> Self {
        SearchState {
            active: false,
            input: String::new(),
            results: Vec::new(),
            current_result: 0,
            no_matches_timer: 0,
        }
    }
}
```

- [ ] **Step 3: Write the App state and event loop**

`src/app.rs`:
```rust
use crate::elf::parser::ElfData;
use crate::ui::tree::{TreeNode, TreeNodeType, TreeState};
use crate::ui::overview::OverviewState;
use crate::ui::info::InfoState;
use crate::ui::hexdump::HexdumpState;
use crate::ui::disasm::DisasmState;
use crate::ui::strings::StringsState;
use crate::ui::search::SearchState;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders},
};
use std::io;

pub enum DetailView {
    Overview,
    StructuredInfo,
    Hexdump,
    Disassembly,
    Strings,
}

pub struct App {
    pub data: ElfData,
    pub tree: TreeState,
    pub overview: OverviewState,
    pub info: InfoState,
    pub hexdump: HexdumpState,
    pub disasm: DisasmState,
    pub strings: StringsState,
    pub search: SearchState,
    pub current_view: DetailView,
    pub focus: Focus,
    pub should_quit: bool,
}

#[derive(PartialEq, Eq)]
pub enum Focus {
    Tree,
    Detail,
    Search,
}

impl App {
    pub fn new(data: ElfData) -> Self {
        let tree = build_tree(&data);
        App {
            data,
            tree: TreeState::new(tree),
            overview: OverviewState::new(),
            info: InfoState::new(),
            hexdump: HexdumpState::new(),
            disasm: DisasmState::new(),
            strings: StringsState::new(),
            search: SearchState::new(),
            current_view: DetailView::Overview,
            focus: Focus::Tree,
            should_quit: false,
        }
    }
}

fn build_tree(data: &ElfData) -> Vec<TreeNode> {
    let mut nodes = Vec::new();

    nodes.push(TreeNode {
        label: "Overview".into(),
        node_type: TreeNodeType::Overview,
        depth: 0,
        children: vec![],
        expanded: true,
    });

    nodes.push(TreeNode {
        label: "ELF Header".into(),
        node_type: TreeNodeType::ElfHeader,
        depth: 0,
        children: vec![],
        expanded: true,
    });

    let mut section_children: Vec<TreeNode> = data
        .sections
        .iter()
        .map(|s| TreeNode {
            label: format!("[{}] {}", s.index, s.name),
            node_type: TreeNodeType::SectionBody {
                index: s.index,
            },
            depth: 1,
            children: vec![],
            expanded: true,
        })
        .collect();

    nodes.push(TreeNode {
        label: format!("Sections ({})", data.sections.len()),
        node_type: TreeNodeType::SectionsGroup,
        depth: 0,
        children: section_children,
        expanded: true,
    });

    let segment_children: Vec<TreeNode> = data
        .segments
        .iter()
        .map(|s| TreeNode {
            label: format!("[{}] {} (0x{:x}-0x{:x})", s.index, s.ty, s.vaddr, s.vaddr + s.memsz),
            node_type: TreeNodeType::Segment { index: s.index },
            depth: 1,
            children: vec![],
            expanded: true,
        })
        .collect();

    nodes.push(TreeNode {
        label: format!("Segments ({})", data.segments.len()),
        node_type: TreeNodeType::SegmentsGroup,
        depth: 0,
        children: segment_children,
        expanded: true,
    });

    let symbol_children: Vec<TreeNode> = data
        .symbols
        .iter()
        .enumerate()
        .map(|(i, sym)| {
            let prefix = match sym.ty {
                crate::elf::parser::SymbolType::Function => "[F]",
                crate::elf::parser::SymbolType::Object => "[O]",
                _ => "[?]",
            };
            TreeNode {
                label: format!("{} {}", prefix, sym.name),
                node_type: TreeNodeType::Symbol { index: i },
                depth: 1,
                children: vec![],
                expanded: true,
            }
        })
        .collect();

    nodes.push(TreeNode {
        label: format!("Symbols ({})", data.symbols.len()),
        node_type: TreeNodeType::SymbolsGroup,
        depth: 0,
        children: symbol_children,
        expanded: true,
    });

    nodes
}

pub fn run_app(data: ElfData) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(data);
    let res = run_event_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    while !app.should_quit {
        terminal.draw(|f| render(f, app))?;
        handle_events(app)?;
    }
    Ok(())
}

fn render(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(f.area());

    crate::ui::tree::render(f, app, chunks[0]);
    crate::ui::mod::render_detail(f, app, chunks[1]);
    crate::ui::search::render(f, app, f.area());
}

fn handle_events(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    if event::poll(std::time::Duration::from_millis(16))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                handle_key(app, key.code);
            }
        }
    }
    Ok(())
}

fn handle_key(app: &mut App, key: KeyCode) {
    if app.search.active {
        match key {
            KeyCode::Esc => {
                app.search.active = false;
                app.search.input.clear();
                app.search.results.clear();
                app.focus = Focus::Tree;
            }
            KeyCode::Enter => {
                crate::ui::search::do_search(app);
                app.search.active = false;
                app.focus = Focus::Detail;
            }
            KeyCode::Backspace => {
                app.search.input.pop();
            }
            KeyCode::Char(c) => {
                app.search.input.push(c);
            }
            _ => {}
        }
        return;
    }

    match key {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('/') => {
            app.search.active = true;
            app.search.input.clear();
            app.focus = Focus::Search;
        }
        KeyCode::Char('n') => {
            if app.focus == Focus::Detail {
                crate::ui::search::next_result(app);
            }
        }
        KeyCode::Char('N') => {
            if app.focus == Focus::Detail {
                crate::ui::search::prev_result(app);
            }
        }
        KeyCode::Tab => {
            app.focus = match app.focus {
                Focus::Tree => Focus::Detail,
                Focus::Detail => Focus::Tree,
                Focus::Search => Focus::Search,
            };
        }
        KeyCode::Up => {
            if app.focus == Focus::Tree {
                app.tree.move_up();
                update_view(app);
            } else if app.focus == Focus::Detail {
                match app.current_view {
                    DetailView::Overview => app.overview.scroll = app.overview.scroll.saturating_sub(1),
                    DetailView::Hexdump => app.hexdump.scroll = app.hexdump.scroll.saturating_sub(1),
                    DetailView::Disassembly => app.disasm.scroll = app.disasm.scroll.saturating_sub(1),
                    DetailView::Strings => app.strings.scroll = app.strings.scroll.saturating_sub(1),
                    DetailView::StructuredInfo => app.info.scroll = app.info.scroll.saturating_sub(1),
                }
            }
        }
        KeyCode::Down => {
            if app.focus == Focus::Tree {
                app.tree.move_down();
                update_view(app);
            } else if app.focus == Focus::Detail {
                match app.current_view {
                    DetailView::Overview => app.overview.scroll += 1,
                    DetailView::Hexdump => app.hexdump.scroll += 1,
                    DetailView::Disassembly => app.disasm.scroll += 1,
                    DetailView::Strings => app.strings.scroll += 1,
                    DetailView::StructuredInfo => app.info.scroll += 1,
                }
            }
        }
        KeyCode::Right | KeyCode::Enter => {
            if app.focus == Focus::Tree {
                app.tree.toggle_expand();
            }
        }
        KeyCode::Left => {
            if app.focus == Focus::Tree {
                app.tree.toggle_expand();
            }
        }
        _ => {}
    }
}

fn update_view(app: &mut App) {
    if let Some(ref node_type) = app.tree.selected_node {
        app.current_view = match node_type {
            TreeNodeType::Overview => DetailView::Overview,
            TreeNodeType::ElfHeader => DetailView::StructuredInfo,
            TreeNodeType::SectionsGroup => DetailView::Overview,
            TreeNodeType::SectionHeader { .. } => DetailView::StructuredInfo,
            TreeNodeType::SectionBody { .. } => DetailView::Hexdump,
            TreeNodeType::SegmentsGroup => DetailView::Overview,
            TreeNodeType::Segment { .. } => DetailView::StructuredInfo,
            TreeNodeType::SymbolsGroup => DetailView::Overview,
            TreeNodeType::Symbol { .. } => DetailView::Disassembly,
        };
    }
}
```

- [ ] **Step 4: Update ui/mod.rs with render_detail dispatcher**

Overwrite `src/ui/mod.rs`:
```rust
pub mod tree;
pub mod overview;
pub mod info;
pub mod hexdump;
pub mod disasm;
pub mod strings;
pub mod search;

use crate::app::{App, DetailView};
use ratatui::prelude::*;

pub fn render_detail(f: &mut Frame, app: &mut App, area: Rect) {
    match app.current_view {
        DetailView::Overview => overview::render(f, app, area),
        DetailView::StructuredInfo => info::render(f, app, area),
        DetailView::Hexdump => hexdump::render(f, app, area),
        DetailView::Disassembly => disasm::render(f, app, area),
        DetailView::Strings => strings::render(f, app, area),
    }
}
```

- [ ] **Step 5: Add minimal render functions to each UI module**

Add to `src/ui/tree.rs`:
```rust
use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem};

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .tree
        .flat_list
        .iter()
        .map(|_| {
            let (node, depth) = get_flat_node(&app.tree.nodes, app.tree.flat_list.as_slice());
            let indent = "  ".repeat(depth as usize);
            ListItem::new(format!("{}{}", indent, node.label))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Navigation"))
        .highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(list, area, &mut app.tree.list_state);
}

fn get_flat_node(nodes: &[TreeNode], flat_indices: &[usize]) -> (&TreeNode, u8) {
    // Simplified: just return the first node and depth 0
    // Will be properly implemented in the tree rendering task
    (&nodes[0], 0)
}
```

Add to `src/ui/overview.rs`:
```rust
use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let text = "Overview - loading...";
    let p = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Overview"));
    f.render_widget(p, area);
}
```

Add to `src/ui/info.rs`:
```rust
use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let text = "Structured Info - loading...";
    let p = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Info"));
    f.render_widget(p, area);
}
```

Add to `src/ui/hexdump.rs`:
```rust
use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let text = "Hexdump - loading...";
    let p = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Hexdump"));
    f.render_widget(p, area);
}
```

Add to `src/ui/disasm.rs`:
```rust
use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let text = "Disassembly - loading...";
    let p = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Disassembly"));
    f.render_widget(p, area);
}
```

Add to `src/ui/strings.rs`:
```rust
use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let text = "Strings - loading...";
    let p = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Strings"));
    f.render_widget(p, area);
}
```

Add to `src/ui/search.rs`:
```rust
use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    if !app.search.active {
        return;
    }
    let search_area = Rect {
        y: area.height.saturating_sub(3),
        height: 3,
        ..area
    };
    let p = Paragraph::new(format!("/{}", app.search.input))
        .block(Block::default().borders(Borders::ALL).title("Search"));
    f.render_widget(p, search_area);
}

pub fn do_search(_app: &mut App) {}

pub fn next_result(_app: &mut App) {}

pub fn prev_result(_app: &mut App) {}
```

- [ ] **Step 6: Verify it compiles**

Run: `cargo check`
Expected: PASS

- [ ] **Step 7: Test with a real ELF file**

Run: `cargo run -- /bin/ls`
Expected: TUI launches with navigation tree visible, press `q` to quit

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat: add app state machine, event loop, and placeholder UI views"
```

---

### Task 5: Tree Navigation Render

**Files:**
- Overwrite: `src/ui/tree.rs`

- [ ] **Step 1: Fix the tree flattening and rendering**

Rewrite `src/ui/tree.rs` (replace the `get_flat_node` function and `render` function):

The file should be overwritten with the complete implementation that properly handles depth-based indentation and node access from the flat list:

```rust
use crate::app::{App, Focus};
use crate::elf::parser::SymbolType;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeNodeType {
    Overview,
    ElfHeader,
    SectionsGroup,
    SectionHeader { index: usize },
    SectionBody { index: usize },
    SegmentsGroup,
    Segment { index: usize },
    SymbolsGroup,
    Symbol { index: usize },
}

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub label: String,
    pub node_type: TreeNodeType,
    pub depth: u8,
    pub children: Vec<TreeNode>,
    pub expanded: bool,
}

#[derive(Debug, Clone)]
pub struct TreeState {
    pub nodes: Vec<TreeNode>,
    pub flat_list: Vec<usize>,
    pub list_state: ListState,
    pub selected_node: Option<TreeNodeType>,
}

impl TreeState {
    pub fn new(nodes: Vec<TreeNode>) -> Self {
        let mut state = TreeState {
            nodes,
            flat_list: Vec::new(),
            list_state: ListState::default(),
            selected_node: None,
        };
        state.rebuild_flat_list();
        state.list_state.select(Some(0));
        state.selected_node = state.node_at_index(0);
        state
    }

    pub fn rebuild_flat_list(&mut self) {
        self.flat_list.clear();
        Self::flatten(&self.nodes, &mut self.flat_list);
    }

    fn flatten(nodes: &[TreeNode], result: &mut Vec<usize>) {
        for (i, node) in nodes.iter().enumerate() {
            result.push(i);
            if node.expanded {
                Self::flatten(&node.children, result);
            }
        }
    }

    fn collect_flat(&self) -> Vec<(&TreeNode, u8, Vec<usize>)> {
        let mut result = Vec::new();
        Self::collect(&self.nodes, 0, &mut vec![], &mut result);
        result
    }

    fn collect<'a>(nodes: &'a [TreeNode], depth: u8, path: &Vec<usize>, result: &mut Vec<(&'a TreeNode, u8, Vec<usize>)>) {
        for (i, node) in nodes.iter().enumerate() {
            let mut node_path = path.clone();
            node_path.push(i);
            result.push((node, depth, node_path.clone()));
            if node.expanded {
                Self::collect(&node.children, depth + 1, &node_path, result);
            }
        }
    }

    pub fn node_at_index(&self, idx: usize) -> Option<TreeNodeType> {
        let flat = self.collect_flat();
        flat.get(idx).map(|(n, _, _)| n.node_type.clone())
    }

    pub fn move_up(&mut self) {
        let idx = self.list_state.selected().unwrap_or(0);
        if idx > 0 {
            self.list_state.select(Some(idx - 1));
            self.selected_node = self.node_at_index(idx - 1);
        }
    }

    pub fn move_down(&mut self) {
        let idx = self.list_state.selected().unwrap_or(0);
        let flat = self.collect_flat();
        if idx + 1 < flat.len() {
            self.list_state.select(Some(idx + 1));
            self.selected_node = self.node_at_index(idx + 1);
        }
    }

    pub fn toggle_expand(&mut self) {
        let flat = self.collect_flat();
        let idx = self.list_state.selected().unwrap_or(0);
        if let Some((node, _, _)) = flat.get(idx) {
            if !node.children.is_empty() {
                let node_type = node.node_type.clone();
                let node_label = node.label.clone();
                // Search top-level nodes
                for top_node in &mut self.nodes {
                    if top_node.node_type == node_type && top_node.label == node_label {
                        top_node.expanded = !top_node.expanded;
                        self.rebuild_flat_list();
                        self.selected_node = self.node_at_index(idx);
                        return;
                    }
                    // Search children
                    for child in &mut top_node.children {
                        if child.node_type == node_type && child.label == node_label {
                            child.expanded = !child.expanded;
                            self.rebuild_flat_list();
                            self.selected_node = self.node_at_index(idx);
                            return;
                        }
                    }
                }
            }
        }
    }
}

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let flat = app.tree.collect_flat();

    let items: Vec<ListItem> = flat
        .iter()
        .map(|(node, depth, _path)| {
            let indent = "  ".repeat(*depth as usize);
            let prefix = if !node.children.is_empty() {
                if node.expanded { "▼ " } else { "▶ " }
            } else {
                "  "
            };
            ListItem::new(format!("{}{}{}", indent, prefix, node.label))
        })
        .collect();

    let border_style = if app.focus == Focus::Tree {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Navigation").border_style(border_style))
        .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White));

    f.render_stateful_widget(list, area, &mut app.tree.list_state);
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: PASS

- [ ] **Step 3: Test tree navigation**

Run: `cargo run -- /bin/ls`
Expected: Tree shows with expand/collapse arrows, arrow keys navigate, Enter toggles

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: implement tree navigation with expand/collapse"
```

---

### Task 6: Overview View

**Files:**
- Overwrite: `src/ui/overview.rs`
- Modify: `src/app.rs` — update `update_view` for SectionBody to route to correct view based on section type

- [ ] **Step 1: Implement overview render**

Rewrite `src/ui/overview.rs`:
```rust
use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let data = &app.data;

    let mut lines = Vec::new();

    // ELF Header
    lines.push(format!("ELF Header"));
    lines.push(format!("  Magic:   {:02x?}", &data.raw_bytes[..16]));
    lines.push(format!(
        "  Class:   {}",
        match data.class {
            1 => "ELF32",
            2 => "ELF64",
            _ => "Unknown",
        }
    ));
    lines.push(format!(
        "  Data:    {}",
        match data.data {
            1 => "2's complement, little endian",
            2 => "2's complement, big endian",
            _ => "Unknown",
        }
    ));
    lines.push(format!("  Version: {} (current)", data.version));
    lines.push(format!("  OS/ABI:  {}", data.os_abi));
    lines.push(format!("  Type:    {}", data.elf_type));
    lines.push(format!("  Machine: {}", data.machine));
    lines.push(format!("  Entry:   0x{:x}", data.entry));
    lines.push(format!(
        "  PH off:  0x{:x} ({} entries, {} bytes each)",
        data.phoff, data.phnum, data.phentsize
    ));
    lines.push(format!(
        "  SH off:  0x{:x} ({} entries, {} bytes each)",
        data.shoff, data.shnum, data.shentsize
    ));
    lines.push(format!("  Flags:   0x{:x}", data.flags));
    lines.push(String::new());

    // Section Headers
    lines.push(format!(
        "Section Headers: [Nr] Name                 Type        Address  Offset   Size     Flags"
    ));
    for s in &data.sections {
        let name = if s.name.len() > 20 {
            format!("{}...", &s.name[..17])
        } else {
            format!("{:20}", s.name)
        };
        lines.push(format!(
            "  [{:2}] {} {:10} 0x{:08x} 0x{:06x} 0x{:06x} {:3}",
            s.index, name, s.ty, s.addr, s.offset, s.size, s.flags
        ));
    }
    lines.push(String::new());

    // Program Headers
    lines.push(format!(
        "Program Headers:  Type       Offset   VirtAddr  PhysAddr  FileSiz  MemSiz   Flg Align"
    ));
    for s in &data.segments {
        lines.push(format!(
            "  {:14} 0x{:06x} 0x{:08x} 0x{:08x} 0x{:06x} 0x{:06x} {:3} 0x{:x}",
            s.ty, s.offset, s.vaddr, s.paddr, s.filesz, s.memsz, s.flags, s.align
        ));
    }

    let text = lines.join("\n");
    let total_lines = lines.len() as u16;
    let area_height = area.height.saturating_sub(2);

    let max_scroll = total_lines.saturating_sub(area_height) as usize;
    if app.overview.scroll > max_scroll {
        app.overview.scroll = max_scroll;
    }

    let p = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title(format!("Overview - {}", app.data.file_path)))
        .scroll((app.overview.scroll as u16, 0));

    f.render_widget(p, area);

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    let mut scrollbar_state = ScrollbarState::new(max_scroll)
        .position(app.overview.scroll);
    f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: PASS

- [ ] **Step 3: Test overview display**

Run: `cargo run -- /bin/ls`
Expected: Overview shows ELF Header, Section table, and Segment table like `readelf -WSlh`

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: implement overview view (readelf -WSlh style)"
```

---

### Task 7: Structured Info View

**Files:**
- Overwrite: `src/ui/info.rs`

- [ ] **Step 1: Implement structured info view for ELF Header, Section Headers, and Program Headers**

Rewrite `src/ui/info.rs`:
```rust
use crate::app::App;
use crate::ui::tree::TreeNodeType;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let node_type = app.tree.selected_node.clone();
    let lines = match node_type {
        Some(TreeNodeType::ElfHeader) => render_elf_header(app),
        Some(TreeNodeType::SectionHeader { index }) => render_section_header(app, index),
        Some(TreeNodeType::SectionBody { index }) => render_section_body_info(app, index),
        Some(TreeNodeType::Segment { index }) => render_segment(app, index),
        Some(TreeNodeType::Symbol { index }) => render_symbol(app, index),
        _ => vec!["Select a node to view details".into()],
    };

    let text = lines.join("\n");
    let p = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Details"));
    f.render_widget(p, area);
}

fn render_elf_header(app: &App) -> Vec<String> {
    let d = &app.data;
    vec![
        format!("Magic:         {:02x?}", &d.raw_bytes[..16]),
        format!("Class:         {}", if d.class == 2 { "ELF64" } else { "ELF32" }),
        format!("Data:          {}", if d.data == 1 { "2's complement, little endian" } else { "2's complement, big endian" }),
        format!("Version:       {} (current)", d.version),
        format!("OS/ABI:        {}", d.os_abi),
        format!("ABI Version:   {}", d.abi_version),
        format!("Type:          {}", d.elf_type),
        format!("Machine:       {}", d.machine),
        format!("Version:       0x{:x}", d.version),
        format!("Entry point:   0x{:x}", d.entry),
        format!("PH offset:     0x{:x} ({} entries, {} bytes each)", d.phoff, d.phnum, d.phentsize),
        format!("SH offset:     0x{:x} ({} entries, {} bytes each)", d.shoff, d.shnum, d.shentsize),
        format!("Flags:         0x{:x}", d.flags),
        format!("EH size:       {} bytes", d.ehsize),
        format!("SH strndx:     {}", d.shstrndx),
    ]
}

fn render_section_header(app: &App, index: usize) -> Vec<String> {
    if index >= app.data.sections.len() {
        return vec!["Section not found".into()];
    }
    let s = &app.data.sections[index];
    vec![
        format!("Name:      {}", s.name),
        format!("Type:      {}", s.ty),
        format!("Flags:     {}", s.flags),
        format!("Address:   0x{:016x}", s.addr),
        format!("Offset:    0x{:x}", s.offset),
        format!("Size:      0x{:x} ({} bytes)", s.size, s.size),
        format!("Link:      {}", s.index),
        format!("Info:      0x{:x}", s.index),
        format!("Addr align: 0x{:x}", s.index),
        format!("Ent size:  0x{:x}", s.index),
    ]
}

fn render_segment(app: &App, index: usize) -> Vec<String> {
    if index >= app.data.segments.len() {
        return vec!["Segment not found".into()];
    }
    let s = &app.data.segments[index];
    vec![
        format!("Type:       {}", s.ty),
        format!("Flags:      {}", s.flags),
        format!("Offset:     0x{:x}", s.offset),
        format!("VirtAddr:   0x{:016x}", s.vaddr),
        format!("PhysAddr:   0x{:016x}", s.paddr),
        format!("FileSiz:    0x{:x} ({} bytes)", s.filesz, s.filesz),
        format!("MemSiz:     0x{:x} ({} bytes)", s.memsz, s.memsz),
        format!("Align:      0x{:x}", s.align),
    ]
}

fn render_section_body_info(app: &App, index: usize) -> Vec<String> {
    if index >= app.data.sections.len() {
        return vec!["Section not found".into()];
    }
    let s = &app.data.sections[index];
    let data_size = s.data.len();
    vec![
        format!("Name:      {}", s.name),
        format!("Type:      {}", s.ty),
        format!("Flags:     {}", s.flags),
        format!("Address:   0x{:016x}", s.addr),
        format!("Offset:    0x{:x}", s.offset),
        format!("Size:      0x{:x} ({} bytes)", s.size, s.size),
        format!("Data size: {} bytes", data_size),
    ]
}

fn render_symbol(app: &App, index: usize) -> Vec<String> {
    if index >= app.data.symbols.len() {
        return vec!["Symbol not found".into()];
    }
    let sym = &app.data.symbols[index];
    let type_str = match sym.ty {
        crate::elf::parser::SymbolType::Function => "FUNC",
        crate::elf::parser::SymbolType::Object => "OBJECT",
        crate::elf::parser::SymbolType::Section => "SECTION",
        crate::elf::parser::SymbolType::File => "FILE",
        crate::elf::parser::SymbolType::Other(_) => "OTHER",
    };
    vec![
        format!("Name:      {}", sym.name),
        format!("Type:      {}", type_str),
        format!("Bind:      {}", sym.bind),
        format!("Vis:       {}", sym.vis),
        format!("Value:     0x{:016x}", sym.addr),
        format!("Size:      {} bytes", sym.size),
        format!("Shndx:     {}", sym.shndx),
    ]
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: implement structured info view for ELF header, sections, segments"
```

---

### Task 8: Hexdump View

**Files:**
- Overwrite: `src/ui/hexdump.rs`
- Modify: `src/app.rs` — add hexdump-specific key handling

- [ ] **Step 1: Implement hexdump view with cursor**

Rewrite `src/ui/hexdump.rs`:
```rust
use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

const BYTES_PER_ROW: usize = 16;

pub enum HexCursor {
    Hex,
    Ascii,
}

pub struct HexdumpState {
    pub scroll: usize,
    pub cursor_offset: usize,
    pub cursor_mode: HexCursor,
    pub goto_input: String,
    pub goto_mode: bool,
}

impl HexdumpState {
    pub fn new() -> Self {
        HexdumpState {
            scroll: 0,
            cursor_offset: 0,
            cursor_mode: HexCursor::Hex,
            goto_input: String::new(),
            goto_mode: false,
        }
    }
}

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let section_index = match &app.tree.selected_node {
        Some(crate::ui::tree::TreeNodeType::SectionBody { index }) => *index,
        _ => {
            let p = Paragraph::new("No section selected")
                .block(Block::default().borders(Borders::ALL).title("Hexdump"));
            f.render_widget(p, area);
            return;
        }
    };

    let section = &app.data.sections[section_index];
    let data = &section.data;

    if data.is_empty() {
        let p = Paragraph::new(format!("Section {} has no data (NOBITS)", section.name))
            .block(Block::default().borders(Borders::ALL).title("Hexdump"));
        f.render_widget(p, area);
        return;
    }

    let total_rows = data.len().div_ceil(BYTES_PER_ROW);
    let visible_rows = area.height.saturating_sub(3) as usize;
    let max_scroll = total_rows.saturating_sub(visible_rows);

    if app.hexdump.scroll > max_scroll {
        app.hexdump.scroll = max_scroll;
    }

    let mut lines = Vec::new();

    // Header
    lines.push(format!(
        "{:10} │ {:47} │ {}",
        "Offset", "00 01 02 03 04 05 06 07  08 09 0A 0B 0C 0D 0E 0F", "ASCII"
    ));
    lines.push(format!("{}", "─".repeat(area.width as usize - 2)));

    let base_addr = section.addr;
    let start_row = app.hexdump.scroll;

    for row in start_row..(start_row + visible_rows).min(total_rows) {
        let offset = row * BYTES_PER_ROW;
        let end = (offset + BYTES_PER_ROW).min(data.len());
        let row_data = &data[offset..end];

        // Hex part
        let hex_str: Vec<String> = row_data.iter().enumerate().map(|(i, b)| {
            let s = format!("{:02x}", b);
            if offset + i == app.hexdump.cursor_offset {
                format!("[{}]", s)
            } else {
                s
            }
        }).collect();
        let hex_line = hex_str.join(" ");
        let hex_padded = format!("{:47}", hex_line);

        // ASCII part
        let ascii_str: String = row_data.iter().enumerate().map(|(i, &b)| {
            if b.is_ascii_graphic() || b == b' ' {
                b as char
            } else {
                '·'
            }
        }).collect();

        lines.push(format!(
            "0x{:08x} │ {} │ {}",
            base_addr + offset as u64,
            hex_padded,
            ascii_str
        ));
    }

    let text = lines.join("\n");
    let title = format!(
        "{} - 0x{:x}-0x{:x}",
        section.name,
        section.addr,
        section.addr + section.size
    );

    let p = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(p, area);
}
```

- [ ] **Step 2: Add hexdump key handling in app.rs**

In `src/app.rs`, in the `handle_key` function, replace the `DetailView::Hexdump` arm in the `Up`/`Down` match with:

```rust
DetailView::Hexdump => {
    app.hexdump.scroll = app.hexdump.scroll.saturating_sub(1);
}
```

And for Down:
```rust
DetailView::Hexdump => {
    app.hexdump.scroll += 1;
}
```

Add new key handlers before the `_ => {}` catch-all in `handle_key`:

```rust
KeyCode::Char('g') => {
    if app.focus == Focus::Detail && matches!(app.current_view, DetailView::Hexdump) {
        app.hexdump.goto_mode = true;
        app.hexdump.goto_input.clear();
        app.focus = Focus::Search;
    }
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: PASS

- [ ] **Step 4: Test hexdump**

Run: `cargo run -- /bin/ls`
Expected: Navigate to a section with data (e.g., .rodata), see hexdump with offset, hex, and ASCII columns

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: implement hexdump view with cursor support"
```

---

### Task 9: Disassembly View

**Files:**
- Overwrite: `src/ui/disasm.rs`
- Modify: `src/app.rs` — add disassembly data to App, initialize on section selection

- [ ] **Step 1: Add disassembly cache to App**

In `src/app.rs`, add to the imports:
```rust
use crate::elf::disasm::{DisasmResult, FunctionInfo, disassemble_section, merge_symbols_with_functions};
```

Add fields to the `App` struct:
```rust
pub disasm_cache: Option<DisasmResult>,
pub current_disasm_section: Option<usize>,
```

Update `App::new`:
```rust
pub fn new(data: ElfData) -> Self {
    let tree = build_tree(&data);
    App {
        data,
        tree: TreeState::new(tree),
        overview: OverviewState::new(),
        info: InfoState::new(),
        hexdump: HexdumpState::new(),
        disasm: DisasmState::new(),
        strings: StringsState::new(),
        search: SearchState::new(),
        current_view: DetailView::Overview,
        focus: Focus::Tree,
        should_quit: false,
        disasm_cache: None,
        current_disasm_section: None,
    }
}
```

Update `update_view` to trigger disassembly:
```rust
fn update_view(app: &mut App) {
    if let Some(ref node_type) = app.tree.selected_node {
        app.current_view = match node_type {
            TreeNodeType::Overview => DetailView::Overview,
            TreeNodeType::ElfHeader => DetailView::StructuredInfo,
            TreeNodeType::SectionsGroup => DetailView::Overview,
            TreeNodeType::SectionHeader { .. } => DetailView::StructuredInfo,
            TreeNodeType::SectionBody { index } => {
                let section = &app.data.sections[*index];
                if section.name == ".text" || section.flags.contains('X') {
                    if app.current_disasm_section != Some(*index) {
                        let disasm = disassemble_section(&section.data, section.addr);
                        let merged = merge_symbols_with_functions(&app.data.symbols, disasm.functions);
                        app.disasm_cache = Some(DisasmResult {
                            functions: merged,
                            all_instructions: disasm.all_instructions,
                            bitness: disasm.bitness,
                        });
                        app.current_disasm_section = Some(*index);
                        app.disasm.selected_function = 0;
                        app.disasm.scroll = 0;
                    }
                    DetailView::Disassembly
                } else if section.name.contains("str") {
                    DetailView::Strings
                } else {
                    DetailView::Hexdump
                }
            }
            TreeNodeType::SegmentsGroup => DetailView::Overview,
            TreeNodeType::Segment { .. } => DetailView::StructuredInfo,
            TreeNodeType::SymbolsGroup => DetailView::Overview,
            TreeNodeType::Symbol { .. } => DetailView::Disassembly,
        };
    }
}
```

- [ ] **Step 2: Implement disassembly view**

Rewrite `src/ui/disasm.rs`:
```rust
use crate::app::App;
use crate::elf::disasm::DisasmInstruction;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

pub struct DisasmState {
    pub selected_function: usize,
    pub scroll: usize,
}

impl DisasmState {
    pub fn new() -> Self {
        DisasmState {
            selected_function: 0,
            scroll: 0,
        }
    }
}

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let disasm = match &app.disasm_cache {
        Some(d) => d,
        None => {
            let p = Paragraph::new("No disassembly available")
                .block(Block::default().borders(Borders::ALL).title("Disassembly"));
            f.render_widget(p, area);
            return;
        }
    };

    let mut lines = Vec::new();

    // Function list
    let func_names: Vec<String> = disasm
        .functions
        .iter()
        .map(|f| {
            if disasm.functions.iter().position(|x| x.start_addr == f.start_addr) == Some(app.disasm.selected_function) {
                format!("[{}]", f.name)
            } else {
                f.name.clone()
            }
        })
        .collect();
    lines.push(format!("Functions: {}", func_names.join(" | ")));
    lines.push(format!("{}", "─".repeat(area.width as usize - 2)));

    // Instructions for selected function
    if let Some(func) = disasm.functions.get(app.disasm.selected_function) {
        let visible_rows = area.height.saturating_sub(4) as usize;
        let total_insns = func.instructions.len();
        let max_scroll = total_insns.saturating_sub(visible_rows);

        if app.disasm.scroll > max_scroll {
            app.disasm.scroll = max_scroll;
        }

        let start = app.disasm.scroll;
        let end = (start + visible_rows).min(total_insns);

        for insn in &func.instructions[start..end] {
            let bytes_str: String = insn
                .bytes
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(" ");
            lines.push(format!(
                "  0x{:08x}: {:20}  {} {}",
                insn.address, bytes_str, insn.mnemonic, insn.operands
            ));
        }

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        let mut scrollbar_state = ScrollbarState::new(max_scroll).position(app.disasm.scroll);
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }

    let title = if let Some(func) = disasm.functions.get(app.disasm.selected_function) {
        format!("Disassembly - {} (0x{:x}-0x{:x})", func.name, func.start_addr, func.end_addr)
    } else {
        "Disassembly".into()
    };

    let text = lines.join("\n");
    let p = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(p, area);
}
```

- [ ] **Step 3: Add Left/Right key handling for function switching in app.rs**

In `handle_key`, add before the `_ => {}` catch-all:
```rust
KeyCode::Right => {
    if app.focus == Focus::Detail && matches!(app.current_view, DetailView::Disassembly) {
        if let Some(ref disasm) = app.disasm_cache {
            if app.disasm.selected_function + 1 < disasm.functions.len() {
                app.disasm.selected_function += 1;
                app.disasm.scroll = 0;
            }
        }
    } else if app.focus == Focus::Tree {
        app.tree.toggle_expand();
    }
}
KeyCode::Left => {
    if app.focus == Focus::Detail && matches!(app.current_view, DetailView::Disassembly) {
        if app.disasm.selected_function > 0 {
            app.disasm.selected_function -= 1;
            app.disasm.scroll = 0;
        }
    } else if app.focus == Focus::Tree {
        app.tree.toggle_expand();
    }
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`
Expected: PASS

- [ ] **Step 5: Test disassembly**

Run: `cargo run -- /bin/ls`
Expected: Navigate to .text section, see function list and disassembly. Left/Right to switch functions.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: implement disassembly view with function navigation"
```

---

### Task 10: Strings View

**Files:**
- Overwrite: `src/ui/strings.rs`

- [ ] **Step 1: Implement strings view**

Rewrite `src/ui/strings.rs`:
```rust
use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

pub struct StringsState {
    pub scroll: usize,
}

impl StringsState {
    pub fn new() -> Self {
        StringsState { scroll: 0 }
    }
}

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let section_index = match &app.tree.selected_node {
        Some(crate::ui::tree::TreeNodeType::SectionBody { index }) => *index,
        _ => {
            let p = Paragraph::new("No string section selected")
                .block(Block::default().borders(Borders::ALL).title("Strings"));
            f.render_widget(p, area);
            return;
        }
    };

    let section = &app.data.sections[section_index];
    let data = &section.data;

    let strings: Vec<(usize, String)> = extract_strings(data);

    let mut lines = Vec::new();
    for (offset, s) in &strings {
        lines.push(format!("  0x{:08x}  {}", section.addr + *offset as u64, s));
    }

    let total = lines.len();
    let visible = area.height.saturating_sub(2) as usize;
    let max_scroll = total.saturating_sub(visible);

    if app.strings.scroll > max_scroll {
        app.strings.scroll = max_scroll;
    }

    let visible_lines: Vec<&str> = lines
        .iter()
        .skip(app.strings.scroll)
        .take(visible)
        .map(|s| s.as_str())
        .collect();

    let text = visible_lines.join("\n");
    let title = format!("{} - {} strings", section.name, strings.len());

    let p = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(p, area);

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    let mut scrollbar_state = ScrollbarState::new(max_scroll).position(app.strings.scroll);
    f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
}

fn extract_strings(data: &[u8]) -> Vec<(usize, String)> {
    let mut results = Vec::new();
    let mut start = None;

    for (i, &byte) in data.iter().enumerate() {
        if byte.is_ascii_graphic() || byte == b' ' {
            if start.is_none() {
                start = Some(i);
            }
        } else if byte == 0 {
            if let Some(s) = start {
                let len = i - s;
                if len >= 2 {
                    let string = String::from_utf8_lossy(&data[s..i]).to_string();
                    results.push((s, string));
                }
                start = None;
            }
        } else {
            start = None;
        }
    }

    results
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: PASS

- [ ] **Step 3: Test strings view**

Run: `cargo run -- /bin/ls`
Expected: Navigate to .dynstr or .strtab, see string list

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: implement strings view for string table sections"
```

---

### Task 11: Search Functionality

**Files:**
- Overwrite: `src/ui/search.rs`
- Modify: `src/app.rs` — wire up search

- [ ] **Step 1: Implement search logic**

Rewrite `src/ui/search.rs`:
```rust
use crate::app::{App, DetailView, Focus};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

pub struct SearchState {
    pub active: bool,
    pub input: String,
    pub results: Vec<usize>,
    pub current_result: usize,
    pub no_matches_timer: u8,
}

impl SearchState {
    pub fn new() -> Self {
        SearchState {
            active: false,
            input: String::new(),
            results: Vec::new(),
            current_result: 0,
            no_matches_timer: 0,
        }
    }
}

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    if !app.search.active {
        return;
    }
    let search_area = Rect {
        y: area.height.saturating_sub(3),
        height: 3,
        width: area.width.min(40),
        x: area.width.saturating_sub(40),
    };
    let display = if app.search.no_matches_timer > 0 {
        format!("/{}  [No matches]", app.search.input)
    } else {
        format!("/{}_", app.search.input)
    };
    let p = Paragraph::new(display)
        .block(Block::default().borders(Borders::ALL).title("Search"));
    f.render_widget(p, search_area);
}

pub fn do_search(app: &mut App) {
    let query = app.search.input.clone();
    if query.is_empty() {
        return;
    }

    app.search.results.clear();

    if query.starts_with("0x") || query.starts_with("0X") {
        if let Ok(addr) = u64::from_str_radix(&query[2..], 16) {
            match app.current_view {
                DetailView::Hexdump => {
                    if let Some(section) = get_current_section(app) {
                        let offset = addr.saturating_sub(section.addr);
                        if (offset as u64) < section.size {
                            app.search.results.push(offset as usize);
                        }
                    }
                }
                DetailView::Disassembly => {
                    if let Some(disasm) = &app.disasm_cache {
                        for (i, insn) in disasm.all_instructions.iter().enumerate() {
                            if insn.address == addr {
                                app.search.results.push(i);
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    } else {
        match app.current_view {
            DetailView::Overview => {
                for (i, line) in build_overview_lines(app).iter().enumerate() {
                    if line.to_lowercase().contains(&query.to_lowercase()) {
                        app.search.results.push(i);
                    }
                }
            }
            DetailView::Disassembly => {
                if let Some(disasm) = &app.disasm_cache {
                    for (i, insn) in disasm.all_instructions.iter().enumerate() {
                        if insn.mnemonic.contains(&query)
                            || insn.operands.contains(&query)
                        {
                            app.search.results.push(i);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if app.search.results.is_empty() {
        app.search.no_matches_timer = 120;
    } else {
        app.search.current_result = 0;
        apply_search_result(app);
    }
}

pub fn next_result(app: &mut App) {
    if app.search.results.is_empty() {
        return;
    }
    app.search.current_result = (app.search.current_result + 1) % app.search.results.len();
    apply_search_result(app);
}

pub fn prev_result(app: &mut App) {
    if app.search.results.is_empty() {
        return;
    }
    app.search.current_result = if app.search.current_result == 0 {
        app.search.results.len() - 1
    } else {
        app.search.current_result - 1
    };
    apply_search_result(app);
}

fn apply_search_result(app: &mut App) {
    if let Some(&pos) = app.search.results.get(app.search.current_result) {
        match app.current_view {
            DetailView::Overview => {
                app.overview.scroll = pos;
            }
            DetailView::Hexdump => {
                app.hexdump.scroll = pos / 16;
            }
            DetailView::Disassembly => {
                app.disasm.scroll = pos;
            }
            _ => {}
        }
    }
}

fn get_current_section(app: &App) -> Option<&crate::elf::parser::SectionInfo> {
    match &app.tree.selected_node {
        Some(crate::ui::tree::TreeNodeType::SectionBody { index }) => {
            Some(&app.data.sections[*index])
        }
        _ => None,
    }
}

fn build_overview_lines(app: &App) -> Vec<String> {
    let data = &app.data;
    let mut lines = Vec::new();
    lines.push(format!("ELF Header Magic: {:02x?}", &data.raw_bytes[..16]));
    lines.push(format!("{} {} {}", data.elf_type, data.machine, data.os_abi));
    lines.push(format!("Entry: 0x{:x}", data.entry));
    for s in &data.sections {
        lines.push(format!("{} {} {:?}", s.name, s.ty, s.addr));
    }
    for s in &data.segments {
        lines.push(format!("{} {:?}", s.ty, s.vaddr));
    }
    lines
}
```

- [ ] **Step 2: Update app.rs to handle search timer**

In `src/app.rs`, in the `run_event_loop` function, add search timer decrement:

```rust
fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    while !app.should_quit {
        terminal.draw(|f| render(f, app))?;
        handle_events(app)?;

        if app.search.no_matches_timer > 0 {
            app.search.no_matches_timer -= 1;
        }
    }
    Ok(())
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: PASS

- [ ] **Step 4: Test search**

Run: `cargo run -- /bin/ls`
Expected: Press `/` to open search, type a query, press Enter to search, `n`/`N` to navigate results

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: implement search functionality with n/N navigation"
```

---

### Task 12: Polish and Final Integration

**Files:**
- Modify: `src/main.rs` — add error context
- Modify: `src/app.rs` — improve key handling, add `Home`/`End` keys, `PgUp`/`PgDn` support

- [ ] **Step 1: Add Home/End/PgUp/PgDn key handling in app.rs**

In `handle_key`, add before the catch-all:
```rust
KeyCode::PageUp => {
    if app.focus == Focus::Detail {
        let visible = 10;
        match app.current_view {
            DetailView::Overview => app.overview.scroll = app.overview.scroll.saturating_sub(visible),
            DetailView::Hexdump => app.hexdump.scroll = app.hexdump.scroll.saturating_sub(visible),
            DetailView::Disassembly => app.disasm.scroll = app.disasm.scroll.saturating_sub(visible),
            DetailView::Strings => app.strings.scroll = app.strings.scroll.saturating_sub(visible),
            DetailView::StructuredInfo => app.info.scroll = app.info.scroll.saturating_sub(visible),
        }
    }
}
KeyCode::PageDown => {
    if app.focus == Focus::Detail {
        let visible = 10;
        match app.current_view {
            DetailView::Overview => app.overview.scroll += visible,
            DetailView::Hexdump => app.hexdump.scroll += visible,
            DetailView::Disassembly => app.disasm.scroll += visible,
            DetailView::Strings => app.strings.scroll += visible,
            DetailView::StructuredInfo => app.info.scroll += visible,
        }
    }
}
KeyCode::Home => {
    if app.focus == Focus::Detail {
        match app.current_view {
            DetailView::Overview => app.overview.scroll = 0,
            DetailView::Hexdump => app.hexdump.scroll = 0,
            DetailView::Disassembly => app.disasm.scroll = 0,
            DetailView::Strings => app.strings.scroll = 0,
            DetailView::StructuredInfo => app.info.scroll = 0,
        }
    }
}
KeyCode::End => {
    if app.focus == Focus::Detail {
        match app.current_view {
            DetailView::Overview => app.overview.scroll = usize::MAX,
            DetailView::Hexdump => app.hexdump.scroll = usize::MAX,
            DetailView::Disassembly => app.disasm.scroll = usize::MAX,
            DetailView::Strings => app.strings.scroll = usize::MAX,
            DetailView::StructuredInfo => app.info.scroll = usize::MAX,
        }
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: PASS

- [ ] **Step 3: Final end-to-end test**

Run: `cargo run -- /bin/ls`
Expected:
- Overview with ELF Header, Section table, Program Headers
- Tree navigation: expand/collapse sections, switch views
- Hexdump: select .rodata, see hex + ASCII
- Disassembly: select .text, see function list and instructions
- Strings: select .dynstr, see string list
- Search: `/` to search, `n`/`N` to navigate
- `q` to quit
- `PgUp`/`PgDn`/`Home`/`End` for scrolling
- `Tab` to switch focus between tree and detail

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: add Home/End/PgUp/PgDn navigation and final polish"
```

---

### Task 13: Release Build

- [ ] **Step 1: Build release binary**

Run: `cargo build --release`
Expected: PASS

- [ ] **Step 2: Test release binary**

Run: `./target/release/elf-insight /bin/ls`
Expected: Same behavior as debug

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "chore: release build verified"
```