use std::{
    collections::{HashMap, HashSet},
    ffi::CStr,
    path::PathBuf,
};

use kelp_config::Config;
use kelp_format::{
    SymBind, SymType,
    elf::{self, Class, Endianness},
    section::{self, Flags as Shf},
};
use log::{info, warn};

pub struct ElfIdent {
    pub class: Class,
    pub endianness: Endianness,
    pub os_abi: u8,
    pub abi_version: u8,
    pub machine: u16,
}

#[derive(Debug)]
pub struct InputSection<'a> {
    pub name: &'a CStr,
    pub flags: section::Flags,
    pub addr: usize,
    pub align: usize,
    pub syms: Vec<InputSym<'a>>,
}

#[derive(Debug)]
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
    assert_eq!(
        parsed.header.typ,
        elf::Type::Relocatable,
        "Inputs must be .o files"
    );

    let shstrtab = extract_strtab(data, &parsed, parsed.header.shstrndx as usize);
    let sym_strtab = extract_strtab(data, &parsed, parsed.symtab_strtab);

    let mut sections = vec![];
    for (i, sec) in parsed.sections.iter().enumerate() {
        assert!(
            !sec.flags.contains(section::Flags::LinkOrder),
            "SHF_LINKORDER is not yet implemented"
        );

        let name = CStr::from_bytes_until_nul(&shstrtab[sec.name..]).unwrap();
        if sec.typ != section::Type::Progbits {
            info!("Skipping section {name:?} since it is not of type SHT_PROGBITS");
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
            class: parsed.header.class,
            endianness: parsed.header.endianness,
            os_abi: parsed.header.os_abi,
            abi_version: parsed.header.abi_version,
            machine: parsed.header.machine,
        },
        sections,
    }
}

#[track_caller]
fn extract_strtab<'a>(data: &'a [u8], parsed: &kelp_format::ElfFile<'_>, idx: usize) -> &'a [u8] {
    assert_ne!(idx, 0, "Files with no shstrtab are not supported");
    let shstrtab = &parsed.sections[idx];
    assert_eq!(shstrtab.flags, section::Flags::empty());
    assert_eq!(shstrtab.typ, section::Type::Strtab);
    &data[shstrtab.offset..][..shstrtab.size]
}

pub struct OutputSection<'a> {
    name: &'a str,
    sections: Vec<InputSection<'a>>,
    flags: section::Flags,
}

pub fn merge_sections<'a>(files: Vec<InputFile<'a>>, cfg: &'a Config) -> Vec<OutputSection<'a>> {
    let mut sects = HashMap::<_, OutputSection>::new();

    for file in files {
        for sec in file.sections {
            if let Some(out) = cfg.output_section(sec.name) {
                let out = sects.entry(out).or_insert(OutputSection {
                    name: out,
                    sections: vec![],
                    flags: Shf::empty(),
                });
                out.flags |= sec.flags;
                out.sections.push(sec);
            } else {
                warn!("Ignoring section {:?}", sec.name);
            }
        }
    }

    sects.into_values().collect()
}

pub fn alloc_segments<'a>(sections: &mut [OutputSection<'a>]) {
    let mut output_flags = HashSet::new();
    for v in sections {
        println!("{} {:?}: ", v.name, v.flags);
        v.sections.sort_by_key(|sec| sec.align as isize * -1);

        for x in &v.sections {
            println!("{x:?}");
            output_flags.insert(x.flags & (Shf::Alloc | Shf::Write | Shf::ExecInstr));
        }
    }
    println!("Segments: {output_flags:?}");
}
