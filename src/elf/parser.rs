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
    pub is_pie: bool,
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

    let (sections, segments, symbols, header_fields, is_pie) = {
        let elf = Elf::parse(&raw_bytes)?;

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
                    shndx: sym.st_shndx as u16,
                })
            })
            .collect();

        let h = &elf.header;
        let is_pie = elf.header.e_type == goblin::elf::header::ET_DYN
            && elf.program_headers.iter().any(|ph| ph.p_type == goblin::elf::program_header::PT_INTERP);
        let header_fields = (
            h.e_ident[goblin::elf::header::EI_CLASS],
            h.e_ident[goblin::elf::header::EI_DATA],
            os_abi_to_str(h.e_ident[goblin::elf::header::EI_OSABI]),
            h.e_ident[goblin::elf::header::EI_ABIVERSION],
            elf_type_to_str(h.e_type),
            machine_to_str(h.e_machine),
            h.e_version,
            h.e_entry,
            h.e_phoff,
            h.e_shoff,
            h.e_flags,
            h.e_ehsize,
            h.e_phentsize,
            h.e_phnum,
            h.e_shentsize,
            h.e_shnum,
            h.e_shstrndx,
        );
        (sections, segments, symbols, header_fields, is_pie)
    };

    Ok(ElfData {
        file_path: path.to_string_lossy().to_string(),
        raw_bytes,
        class: header_fields.0,
        data: header_fields.1,
        os_abi: header_fields.2,
        abi_version: header_fields.3,
        elf_type: header_fields.4,
        is_pie,
            machine: header_fields.5,
        version: header_fields.6,
        entry: header_fields.7,
        phoff: header_fields.8,
        shoff: header_fields.9,
        flags: header_fields.10,
        ehsize: header_fields.11,
        phentsize: header_fields.12,
        phnum: header_fields.13,
        shentsize: header_fields.14,
        shnum: header_fields.15,
        shstrndx: header_fields.16,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_elf_on_real_binary() {
        let data = parse_elf("/bin/ls").expect("Failed to parse /bin/ls");
        assert_eq!(&data.raw_bytes[..4], &[0x7f, b'E', b'L', b'F']);
        assert_eq!(data.class, 2, "Should be ELF64");
        assert_eq!(data.data, 1, "Should be little endian");
        assert!(!data.elf_type.is_empty());
        assert!(!data.machine.is_empty());
        assert!(!data.sections.is_empty());
        assert!(!data.segments.is_empty());
        assert!(data.sections.iter().any(|s| s.name == ".text"));
        assert!(data.sections.iter().any(|s| s.name == ".rodata"));
    }

    #[test]
    fn test_parse_elf_nonexistent_file() {
        let result = parse_elf("/nonexistent/file");
        assert!(result.is_err());
    }

    #[test]
    fn test_elf_type_to_str() {
        assert_eq!(elf_type_to_str(goblin::elf::header::ET_NONE), "NONE");
        assert_eq!(elf_type_to_str(goblin::elf::header::ET_REL), "REL (Relocatable)");
        assert_eq!(elf_type_to_str(goblin::elf::header::ET_EXEC), "EXEC (Executable)");
        assert_eq!(elf_type_to_str(goblin::elf::header::ET_DYN), "DYN (Shared object)");
        assert_eq!(elf_type_to_str(goblin::elf::header::ET_CORE), "CORE");
        assert_eq!(elf_type_to_str(0xFFFF), "0xffff");
    }

    #[test]
    fn test_machine_to_str() {
        assert_eq!(machine_to_str(goblin::elf::header::EM_X86_64), "AMD x86-64");
        assert_eq!(machine_to_str(goblin::elf::header::EM_386), "Intel 80386");
        assert_eq!(machine_to_str(goblin::elf::header::EM_ARM), "ARM");
        assert_eq!(machine_to_str(goblin::elf::header::EM_AARCH64), "AArch64");
        assert_eq!(machine_to_str(0xFFFF), "0xffff");
    }

    #[test]
    fn test_os_abi_to_str() {
        assert_eq!(os_abi_to_str(goblin::elf::header::ELFOSABI_NONE), "UNIX - System V");
        assert_eq!(os_abi_to_str(goblin::elf::header::ELFOSABI_LINUX), "UNIX - Linux");
        assert_eq!(os_abi_to_str(0xFF), "0xff");
    }

    #[test]
    fn test_section_type_to_str() {
        assert_eq!(section_type_to_str(goblin::elf::section_header::SHT_PROGBITS), "PROGBITS");
        assert_eq!(section_type_to_str(goblin::elf::section_header::SHT_NOBITS), "NOBITS");
        assert_eq!(section_type_to_str(0xDEAD), "0xdead");
    }

    #[test]
    fn test_section_flags_to_str() {
        let w = goblin::elf::section_header::SHF_WRITE as u64;
        let a = goblin::elf::section_header::SHF_ALLOC as u64;
        let x = goblin::elf::section_header::SHF_EXECINSTR as u64;
        assert_eq!(section_flags_to_str(0), "-");
        assert_eq!(section_flags_to_str(w), "W");
        assert_eq!(section_flags_to_str(a), "A");
        assert_eq!(section_flags_to_str(x), "X");
        assert_eq!(section_flags_to_str(w | a), "WA");
        assert_eq!(section_flags_to_str(w | a | x), "WAX");
    }

    #[test]
    fn test_segment_type_to_str() {
        assert_eq!(segment_type_to_str(goblin::elf::program_header::PT_LOAD), "LOAD");
        assert_eq!(segment_type_to_str(goblin::elf::program_header::PT_DYNAMIC), "DYNAMIC");
        assert_eq!(segment_type_to_str(goblin::elf::program_header::PT_NOTE), "NOTE");
        assert_eq!(segment_type_to_str(0xFFFF), "0xffff");
    }

    #[test]
    fn test_segment_flags_to_str() {
        let r = goblin::elf::program_header::PF_R;
        let w = goblin::elf::program_header::PF_W;
        let x = goblin::elf::program_header::PF_X;
        assert_eq!(segment_flags_to_str(0), "");
        assert_eq!(segment_flags_to_str(r), "R");
        assert_eq!(segment_flags_to_str(w), "W");
        assert_eq!(segment_flags_to_str(x), "E");
        assert_eq!(segment_flags_to_str(r | w | x), "RWE");
    }

    #[test]
    fn test_sym_bind_to_str() {
        assert_eq!(sym_bind_to_str(goblin::elf::sym::STB_LOCAL), "LOCAL");
        assert_eq!(sym_bind_to_str(goblin::elf::sym::STB_GLOBAL), "GLOBAL");
        assert_eq!(sym_bind_to_str(goblin::elf::sym::STB_WEAK), "WEAK");
        assert_eq!(sym_bind_to_str(0xFF), "0xff");
    }

    #[test]
    fn test_sym_vis_to_str() {
        assert_eq!(sym_vis_to_str(goblin::elf::sym::STV_DEFAULT), "DEFAULT");
        assert_eq!(sym_vis_to_str(goblin::elf::sym::STV_HIDDEN), "HIDDEN");
        assert_eq!(sym_vis_to_str(0xFF), "0xff");
    }
}