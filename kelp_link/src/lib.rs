use std::{ffi::CStr, path::PathBuf};

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
    pub shndx: usize,
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
    assert_eq!(ehdr.typ, elf::Type::Shared, "Inputs must be .o files");

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
    for sec in parsed.sections {
        sections.push(InputSection {
            name: CStr::from_bytes_until_nul(&shstrtab[sec.name..]).unwrap(),
            flags: sec.flags,
            addr: sec.addr as usize,
            align: sec.align as usize,
            syms: vec![],
        });
    }

    for sym in parsed.syms {
        sections[sym.shndx as usize].syms.push(InputSym {
            name: CStr::from_bytes_until_nul(&sym_strtab[sym.name as usize..]).unwrap(),
            shndx: sym.shndx as usize,
            bind: sym.bind,
            typ: sym.typ,
            other: sym.other,
            value: sym.value,
            size: sym.size,
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

pub fn merge<'a>(_: Vec<InputFile<'a>>) {
    // Merge all sections that have same flags and "similar" name
    // TBD what similar means

    // Group them by flags, and assign virtual addresses and segments
}
