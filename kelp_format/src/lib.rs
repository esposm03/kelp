use num_derive::FromPrimitive;

pub mod section {
    use num_derive::FromPrimitive;

    bitflags::bitflags! {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub struct Flags: u32 {
            const Write	           = 1 << 0;	/* Writable */
            const Alloc	           = 1 << 1;	/* Occupies memory during execution */
            const ExecInstr	       = 1 << 2;	/* Executable */
            const Merge	           = 1 << 4;	/* Might be merged */
            const Strings	       = 1 << 5;	/* Contains nul-terminated strings */
            const InfoLink	       = 1 << 6;	/* `sh_info' contains SHT index */
            const LinkOrder	       = 1 << 7;	/* Preserve order after combining */
            const OsNonconforming  = 1 << 8;	/* Non-standard OS specific handling */
        }
    }

    #[repr(u32)]
    #[derive(Clone, Copy, Debug, FromPrimitive, PartialEq, Eq, Hash)]
    pub enum Type {
        Null = 0,                   /* Section header table entry unused */
        Progbits = 1,               /* Program data */
        Symtab = 2,                 /* Symbol table */
        Strtab = 3,                 /* String table */
        Rela = 4,                   /* Relocation entries with addends */
        Hash = 5,                   /* Symbol hash table */
        Dynamic = 6,                /* Dynamic linking information */
        Note = 7,                   /* Notes */
        Nobits = 8,                 /* Program space with no data (bss) */
        Rel = 9,                    /* Relocation entries, no addends */
        Shlib = 10,                 /* Reserved */
        Dynsym = 11,                /* Dynamic linker symbol table */
        InitArray = 14,             /* Array of constructors */
        FiniArray = 15,             /* Array of destructors */
        PreinitArray = 16,          /* Array of pre-constructors */
        Group = 17,                 /* Section group */
        SymtabShndx = 18,           /* Extended section indices */
        Relr = 19,                  /* RELR relative relocations */
        Num = 20,                   /* Number of defined types.  */
        LlvmAddrSig = 0x6fff4c03,   /* LLVM Address Signatures */
        GnuAttributes = 0x6ffffff5, /* Object attributes.  */
        GnuHash = 0x6ffffff6,       /* GNU-style hash table.  */
        GnuLiblist = 0x6ffffff7,    /* Prelink library list */
        Checksum = 0x6ffffff8,      /* Checksum for DSO content.  */
        SunwMove = 0x6ffffffa,
        SunwComdat = 0x6ffffffb,
        SunwSyminfo = 0x6ffffffc,
        GnuVerdef = 0x6ffffffd,  /* Version definition section.  */
        GnuVerneed = 0x6ffffffe, /* Version needs section.  */
        GnuVersym = 0x6fffffff,  /* Version symbol table.  */
        X8664Unwind = 0x70000001,
    }

    #[derive(Debug, PartialEq, Eq)]
    pub struct Header {
        pub name: usize,
        pub typ: Type,
        pub flags: Flags,
        pub addr: u64,
        pub offset: usize,
        pub size: usize,
        pub link: u32,
        pub info: u32,
        pub align: u64,
        pub entry_size: u64,
    }
}

pub mod segment {
    #[repr(u32)]
    #[derive(Clone, Copy, Debug, num_derive::FromPrimitive, PartialEq, Eq, Hash)]
    pub enum Type {
        Null = 0,                 /* Program header table entry unused */
        Load = 1,                 /* Loadable program segment */
        Dynamic = 2,              /* Dynamic linking information */
        Interp = 3,               /* Program interpreter */
        Note = 4,                 /* Auxiliary information */
        Shlib = 5,                /* Reserved */
        Phdr = 6,                 /* Entry for header table itself */
        Tls = 7,                  /* Thread-local storage segment */
        Num = 8,                  /* Number of defined types */
        GnuEhFrame = 0x6474e550,  /* GCC .eh_frame_hdr segment */
        GnuStack = 0x6474e551,    /* Indicates stack executability */
        GnuRelro = 0x6474e552,    /* Read-only after relocation */
        GnuProperty = 0x6474e553, /* GNU property */
        GnuSframe = 0x6474e554,   /* SFrame segment.  */
        SunwBss = 0x6ffffffa,     /* Sun Specific segment */
        SunWstack = 0x6ffffffb,   /* Stack segment */
    }

    bitflags::bitflags! {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub struct Flags: u32 {
            const Read             = 1 << 2;
            const Write            = 1 << 1;
            const Exec             = 1 << 0;
        }
    }

    #[derive(Debug)]
    pub struct Header {
        pub segment_type: Type,
        pub flags: Flags,
        pub offset: u64,
        pub virtual_addr: u64,
        pub physical_addr: u64,
        pub file_size: u64,
        pub mem_size: u64,
        pub align: u64,
    }
}

pub mod elf {
    #[derive(Clone, Copy, Debug, num_derive::FromPrimitive, PartialEq, Eq)]
    pub enum Class {
        Class32 = 1,
        Class64 = 2,
    }

    #[derive(Clone, Copy, Debug, num_derive::FromPrimitive, PartialEq, Eq)]
    pub enum Endianness {
        Little = 1,
        Big = 2,
    }

    #[derive(Clone, Copy, Debug, num_derive::FromPrimitive, PartialEq, Eq)]
    pub enum Type {
        Relocatable = 1, // ET_REL
        Executable = 2,  // ET_EXEC
        Shared = 3,      // ET_DYN
        Core = 4,        // ET_CORE
    }

    #[derive(Debug)]
    pub struct Header {
        pub class: Class,
        pub endianness: Endianness,
        pub os_abi: u8,
        pub abi_version: u8,
        pub typ: Type,
        pub machine: u16,
        pub entry: usize,
        pub phoff: usize,
        pub shoff: usize,
        pub flags: u32,
        pub header_size: u16,
        pub phentsize: u16,
        pub phnum: u16,
        pub shentsize: u16,
        pub shnum: u16,
        pub shstrndx: u16,
    }
}

#[derive(Debug)]
pub struct ElfFile<'a> {
    pub header: elf::Header,
    pub sections: Vec<section::Header>,
    pub segments: Vec<segment::Header>,
    pub syms: Vec<Sym>,
    /// The index of the strtab to be used for looking up symbol names.
    pub symtab_strtab: usize,

    pub data: &'a [u8],
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, FromPrimitive)]
pub enum SymBind {
    Local = 0,
    Global = 1,
    Weak = 2,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, FromPrimitive)]
pub enum SymType {
    NoType = 0,
    Object = 1,
    Func = 2,
    Section = 3,
    File = 4,
    Common = 5,
    Tls = 6,
}

#[derive(Clone, Copy, Debug)]
pub struct Sym {
    pub name: u32,
    pub bind: SymBind,
    pub typ: SymType,
    pub other: u8,
    pub shndx: u16,
    pub value: usize,
    pub size: usize,
}

mod parser;
pub use parser::elf as parse;

mod writer;
pub use writer::Writer;
