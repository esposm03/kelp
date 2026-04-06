use std::{collections::HashMap, ffi::CStr, path::PathBuf};

use kelp_config::Config;
use kelp_format::{
    SymBind, SymType,
    elf::{self, Class, Endianness},
    section,
};

pub struct ElfIdent {
    pub class: Class,
    pub endianness: Endianness,
    pub os_abi: u8,
    pub abi_version: u8,
    pub machine: u16,
}

pub struct InputSection<'a> {
    pub name: &'a CStr,
    pub flags: section::Flags,
    pub addr: usize,
    pub align: usize,
    pub syms: Vec<InputSym<'a>>,
}

pub struct InputSym<'a> {
    pub name: &'a CStr,
    pub bind: SymBind,
    pub typ: SymType,
    pub other: u8,
    pub value: usize,
    pub size: usize,
}

pub struct InputFile<'a> {
    pub name: PathBuf,
    pub ident: ElfIdent,
    pub sections: Vec<InputSection<'a>>,
}

pub fn process_input_file<'a>(path: PathBuf, data: &'a [u8]) -> InputFile<'a> {
    let parsed = kelp_format::parse(data);
    let ehdr = parsed.header;
    assert_eq!(ehdr.typ, elf::Type::Relocatable, "Inputs must be .o files");

    // Extract the symtab for the section headers
    assert_ne!(ehdr.shstrndx, 0, "Files with no shstrtab are not supported");
    let shstrtab = &parsed.sections[ehdr.shstrndx as usize];
    assert_eq!(shstrtab.flags, section::Flags::empty());
    assert_eq!(shstrtab.typ, section::Type::Strtab);
    let shstrtab = &data[shstrtab.offset..][..shstrtab.size];

    // Extract the strtab for the symbol table
    assert_ne!(parsed.symtab_strtab, 0, "Found file with no symtab");
    let sym_strtab = &parsed.sections[parsed.symtab_strtab];
    assert_eq!(sym_strtab.flags, section::Flags::empty());
    assert_eq!(sym_strtab.typ, section::Type::Strtab);
    let sym_strtab = &data[sym_strtab.offset..][..sym_strtab.size];

    let mut sections = vec![];
    for (i, sec) in parsed.sections.iter().enumerate() {
        let name = CStr::from_bytes_until_nul(&shstrtab[sec.name..]).unwrap();
        if sec.typ != section::Type::Progbits {
            println!("[INFO] Skipping section {name:?} since it is not of type SHT_PROGBITS");
            continue;
        }

        // Collect symbols related to this section
        let mut syms = vec![];
        for sym in &parsed.syms {
            if sym.shndx as usize != i {
                continue;
            }
            syms.push(InputSym {
                name: CStr::from_bytes_until_nul(&sym_strtab[sym.name as usize..]).unwrap(),
                bind: sym.bind,
                typ: sym.typ,
                other: sym.other,
                value: sym.value,
                size: sym.size,
            });
        }

        sections.push(InputSection {
            name,
            flags: sec.flags,
            addr: sec.addr as usize,
            align: sec.align as usize,
            syms: vec![],
        });
    }

    InputFile {
        name: path,
        ident: ElfIdent {
            class: ehdr.class,
            endianness: ehdr.endianness,
            os_abi: ehdr.os_abi,
            abi_version: ehdr.abi_version,
            machine: ehdr.machine,
        },
        sections,
    }
}

#[derive(Default)]
struct OutputSection<'a> {
    sections: Vec<InputSection<'a>>,
}

pub fn merge<'a>(files: Vec<InputFile<'a>>, cfg: Config) {
    // Merge all sections that have same flags and "similar" name
    let mut sects = HashMap::<_, OutputSection>::new();

    for file in files {
        for sec in file.sections {
            if let Some(out) = cfg.output_section(sec.name) {
                let out = sects.entry(out).or_default();
                out.sections.push(sec);
            } else {
                println!("[WARN] Ignoring section {:?}", sec.name);
            }
        }
    }

    // Group them by flags, and assign virtual addresses and segments
}
