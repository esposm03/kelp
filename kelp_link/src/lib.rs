use std::{cmp::max, collections::HashMap, ffi::CStr, path::PathBuf};

use indexmap::IndexMap;
use kelp_config::Config;
use kelp_format::{
    SymBind, SymType,
    elf::{self, Class, Endianness},
    section::{self, Flags as Shf},
    segment::Flags as Phf,
};
use log::{info, warn};

pub struct ElfIdent {
    pub class: Class,
    pub endianness: Endianness,
    pub os_abi: u8,
    pub abi_version: u8,
    pub machine: u16,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct InputSection<'a> {
    pub name: &'a CStr,
    pub flags: section::Flags,
    pub addr: usize,
    pub align: usize,
    pub size: usize,
    pub syms: Vec<InputSym<'a>>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
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
            size: sec.size as usize,
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutputSection<'a> {
    pub name: &'a str,
    pub input_sections: Vec<InputSection<'a>>,
    pub size: usize,
    pub align: usize,
    pub flags: section::Flags,
}

pub fn merge_sections<'a>(files: Vec<InputFile<'a>>, cfg: &'a Config) -> Vec<OutputSection<'a>> {
    let mut sects = IndexMap::<_, OutputSection>::new();

    for file in files {
        for sec in file.sections {
            if let Some((i, out)) = cfg.output_section(sec.name) {
                let out = sects.entry(i).or_insert(OutputSection {
                    name: out,
                    input_sections: vec![],
                    flags: Shf::empty(),

                    size: 0,
                    align: 0,
                });
                out.flags |= sec.flags;
                out.align = max(out.align, sec.align);
                out.size = align(out.size, sec.align);
                out.size += sec.size;
                out.input_sections.push(sec);
            } else {
                warn!("Ignoring section {:?}", sec.name);
            }
        }
    }

    // Sort sections like in the configuration
    sects
        .sorted_unstable_by(|k1, _, k2, _| k1.cmp(k2))
        .map(|(_, v)| v)
        .collect()
}

pub struct OutputSegment<'a> {
    pub sections: Vec<OutputSection<'a>>,
    pub flags: Phf,

    pub size: usize,
    pub virtaddr: usize,
}

impl<'a> OutputSegment<'a> {
    pub fn new(flags: Phf) -> Self {
        Self {
            sections: vec![],
            flags,
            size: 0,
            virtaddr: 0,
        }
    }
}

pub fn alloc_segments<'a>(sections: Vec<OutputSection<'a>>) -> Vec<OutputSegment<'a>> {
    let mut output_flags = HashMap::new();

    for mut sec in sections {
        let flags = shf_to_phf(sec.flags);
        sec.input_sections
            .sort_by_key(|sec| sec.align as isize * -1);

        let entry = output_flags
            .entry(flags)
            .or_insert(OutputSegment::new(flags));

        entry.size = align(entry.size, sec.align);
        entry.size += sec.size;
        entry.sections.push(sec);
    }

    let mut virtaddr = 0x201000;
    let mut res: Vec<_> = output_flags.into_values().collect();
    res.sort_unstable_by_key(|seg| seg.flags);

    for seg in &mut res {
        virtaddr = align(virtaddr, 0x1000); // TODO: this should be an architecture-specific constant
        // TODO: here we also assume that the sections have an alignment <= 0x1000
        seg.virtaddr = virtaddr;
    }

    res
}

fn shf_to_phf(shf: Shf) -> Phf {
    let mut res = Phf::empty();

    if shf.contains(Shf::Alloc) {
        res |= Phf::Read;
    }
    if shf.contains(Shf::Write) {
        res |= Phf::Write;
    }
    if shf.contains(Shf::ExecInstr) {
        res |= Phf::Exec;
    }

    res
}

fn align(what: usize, align_to: usize) -> usize {
    let align_to = align_to.max(1);
    let aligned = what + (align_to - 1) & !(align_to - 1);
    aligned
}
