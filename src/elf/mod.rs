pub mod disasm;
pub mod parser;

#[cfg(test)]
pub(crate) mod fixtures {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Tests run in parallel within one process, so each fixture needs its
    /// own file name or they clobber each other.
    static FIXTURE_SEQ: AtomicUsize = AtomicUsize::new(0);

    fn write_temp_elf(tag: &str, bytes: &[u8]) -> PathBuf {
        let seq = FIXTURE_SEQ.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "elf-insight-{}-{}-{}.elf",
            tag,
            std::process::id(),
            seq
        ));
        fs::write(&path, bytes).expect("failed to write ELF fixture");
        path
    }

    /// `/bin/ls` with section 1's `sh_offset` set past `u64::MAX - sh_size`,
    /// so `sh_offset + sh_size` overflows.
    pub(crate) fn overflow_section_fixture() -> PathBuf {
        let mut bytes = fs::read("/bin/ls").expect("failed to read /bin/ls");
        assert_eq!(bytes[4], 2, "/bin/ls should be ELF64");

        let shoff = u64::from_le_bytes(bytes[0x28..0x30].try_into().unwrap()) as usize;
        let shentsize = u16::from_le_bytes(bytes[0x3A..0x3C].try_into().unwrap()) as usize;
        let shnum = u16::from_le_bytes(bytes[0x3C..0x3E].try_into().unwrap()) as usize;
        assert!(
            shnum > 1,
            "/bin/ls should have more than one section header"
        );

        let sh = shoff + shentsize;
        let sh_type = u32::from_le_bytes(bytes[sh + 4..sh + 8].try_into().unwrap());
        assert_ne!(
            sh_type,
            goblin::elf::section_header::SHT_NOBITS,
            "section 1 must carry data for this fixture to exercise the slice"
        );
        bytes[sh + 24..sh + 32].copy_from_slice(&0xFFFF_FFFF_FFFF_FFF0u64.to_le_bytes());
        bytes[sh + 32..sh + 40].copy_from_slice(&0x20u64.to_le_bytes());

        write_temp_elf("overflow", &bytes)
    }

    /// Minimal synthetic ELF32 (i386) with `.shstrtab`, a two-entry `.dynamic`
    /// (DT_STRTAB, DT_NULL) and a two-byte `.text` holding `inc eax; ret`.
    pub(crate) fn elf32_fixture() -> PathBuf {
        const EHDR_SIZE: usize = 52;
        const SHDR_SIZE: usize = 40;

        // name indices: 1 = ".shstrtab", 11 = ".dynamic", 20 = ".text"
        let shstrtab: &[u8] = b"\0.shstrtab\0.dynamic\0.text\0";

        let shstrtab_off = EHDR_SIZE;
        let dynamic_off = shstrtab_off + shstrtab.len();
        let text_off = dynamic_off + 16;
        let shoff = text_off + 2;

        let mut bytes = Vec::with_capacity(shoff + 4 * SHDR_SIZE);

        // ELF32 header
        bytes.extend_from_slice(&[0x7f, b'E', b'L', b'F']);
        bytes.extend_from_slice(&[1, 1, 1, 0, 0]); // class32, LE, version 1, SysV
        bytes.extend_from_slice(&[0; 7]);
        bytes.extend_from_slice(&3u16.to_le_bytes()); // ET_DYN
        bytes.extend_from_slice(&3u16.to_le_bytes()); // EM_386
        bytes.extend_from_slice(&1u32.to_le_bytes()); // e_version
        bytes.extend_from_slice(&0u32.to_le_bytes()); // e_entry
        bytes.extend_from_slice(&0u32.to_le_bytes()); // e_phoff
        bytes.extend_from_slice(&(shoff as u32).to_le_bytes()); // e_shoff
        bytes.extend_from_slice(&0u32.to_le_bytes()); // e_flags
        bytes.extend_from_slice(&(EHDR_SIZE as u16).to_le_bytes()); // e_ehsize
        bytes.extend_from_slice(&32u16.to_le_bytes()); // e_phentsize
        bytes.extend_from_slice(&0u16.to_le_bytes()); // e_phnum
        bytes.extend_from_slice(&(SHDR_SIZE as u16).to_le_bytes()); // e_shentsize
        bytes.extend_from_slice(&4u16.to_le_bytes()); // e_shnum
        bytes.extend_from_slice(&1u16.to_le_bytes()); // e_shstrndx
        assert_eq!(bytes.len(), EHDR_SIZE);

        bytes.extend_from_slice(shstrtab);
        bytes.extend_from_slice(&5u32.to_le_bytes()); // DT_STRTAB
        bytes.extend_from_slice(&0x1234u32.to_le_bytes());
        bytes.extend_from_slice(&[0; 8]); // DT_NULL
        bytes.extend_from_slice(&[0x40, 0xc3]); // inc eax; ret
        assert_eq!(bytes.len(), shoff);

        let mut push_shdr = |name: u32,
                             ty: u32,
                             flags: u32,
                             addr: u32,
                             off: u32,
                             size: u32,
                             align: u32,
                             entsize: u32| {
            bytes.extend_from_slice(&name.to_le_bytes());
            bytes.extend_from_slice(&ty.to_le_bytes());
            bytes.extend_from_slice(&flags.to_le_bytes());
            bytes.extend_from_slice(&addr.to_le_bytes());
            bytes.extend_from_slice(&off.to_le_bytes());
            bytes.extend_from_slice(&size.to_le_bytes());
            bytes.extend_from_slice(&0u32.to_le_bytes()); // sh_link
            bytes.extend_from_slice(&0u32.to_le_bytes()); // sh_info
            bytes.extend_from_slice(&align.to_le_bytes());
            bytes.extend_from_slice(&entsize.to_le_bytes());
        };

        push_shdr(0, 0, 0, 0, 0, 0, 0, 0); // SHT_NULL
        push_shdr(1, 3, 0, 0, shstrtab_off as u32, shstrtab.len() as u32, 1, 0);
        push_shdr(11, 6, 0, 0, dynamic_off as u32, 16, 4, 8); // SHT_DYNAMIC
        push_shdr(20, 1, 0x6, 0x0804_8000, text_off as u32, 2, 1, 0); // PROGBITS, A|X

        write_temp_elf("elf32", &bytes)
    }
}
