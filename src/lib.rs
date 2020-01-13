use std::error::Error;
use std::fs::{self, File};
use std::io::prelude::Write;

pub struct Config {
    pub filename: String,
}

impl Config {
    pub fn new(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 2 {
            return Err("not enough arguments")
        }

        let filename = args[1].clone();

        Ok(Config { filename })
    }
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let source = fs::read_to_string(config.filename)?;
    let intermediate = generate_intermediate_code(&source);
    // optimization
    let target_code = generate_target_code(&intermediate);
    write_elf(&target_code, "a.out")?;
    Ok(())
}

enum ByteCode {
    IncrementPointer,
    DecrementPointer,
    IncrementCell,
    DecrementCell,
    WriteByte,
    ReadByte,
    JumpIfZero,
    JumpIfNotZero,
}

fn generate_intermediate_code(source: &str) -> Vec<ByteCode> {
    let mut intermediate: Vec<ByteCode> = Vec::new();

    for instruction in source.chars() {
        match instruction {
            '>' => intermediate.push(ByteCode::IncrementPointer),
            '<' => intermediate.push(ByteCode::DecrementPointer),
            '+' => intermediate.push(ByteCode::IncrementCell),
            '-' => intermediate.push(ByteCode::DecrementCell),
            '.' => intermediate.push(ByteCode::WriteByte),
            ',' => intermediate.push(ByteCode::ReadByte),
            '[' => intermediate.push(ByteCode::JumpIfZero),
            ']' => intermediate.push(ByteCode::JumpIfNotZero),
            _ => (),
        }
    }

    intermediate
}

fn generate_target_code(intermediate: &Vec<ByteCode>) -> Vec<u8> {
    let mut target: Vec<u8> = Vec::new();

    // for computing jumps
    let mut stack: Vec<usize> = Vec::new();

    // notes
    // syscalls: rax, rdi, rsi, rdx
    // pointer: rsi

    // TODO allocate the memory without brk

    // sub rsp, 0x8000
    target.push(0x48);
    target.push(0x81);
    target.push(0xec);
    emit_dword(&mut target, 0x8000);
    // mov rsi, rsp
    target.push(0x48);
    target.push(0x89);
    target.push(0xe6);

    // // mov eax, 0xc
    // target.push(0xb8);
    // emit_dword(&mut target, 0xc);
    // // mov edi, 0x8000
    // target.push(0xbf);
    // emit_dword(&mut target, 0x8000);
    // // syscall
    // target.push(0x0f);
    // target.push(0x05);
    
    // mov esi, eax
    // target.push(0x89);
    // target.push(0xc6);
    
    for instruction in intermediate {
        match instruction {
            ByteCode::IncrementPointer => {
                // add rsi, imm8
                target.push(0x48);
                target.push(0x83);
                target.push(0xc6);
                target.push(0x01);
            },
            ByteCode::DecrementPointer => {
                // sub rsi, imm8
                target.push(0x48);
                target.push(0x83);
                target.push(0xee);
                target.push(0x01);
            },
            ByteCode::IncrementCell => {
                // add BYTE PTR [rsi], imm8
                target.push(0x80);
                target.push(0x06);
                target.push(0x01);
            },
            ByteCode::DecrementCell => {
                // add BYTE PTR [rsi], imm8
                target.push(0x80);
                target.push(0x2e);
                target.push(0x01);
            },
            ByteCode::WriteByte => {
                // mov eax, 0x1
                target.push(0xb8);
                emit_dword(&mut target, 0x01);
                // mov edi, 0x1
                target.push(0xbf);
                emit_dword(&mut target, 0x01);
                // mov edx, 0x1
                target.push(0xba);
                emit_dword(&mut target, 0x01);
                // syscall
                target.push(0x0f);
                target.push(0x05);

            },
            ByteCode::ReadByte => {
                // xor eax, eax
                target.push(0x31);
                target.push(0xc0);
                // xor edi, edi
                target.push(0x31);
                target.push(0xff);
                // mov edx, 0x1
                target.push(0xba);
                emit_dword(&mut target, 0x01);
                // syscall
                target.push(0x0f);
                target.push(0x05);
            },
            ByteCode::JumpIfZero => {
                // cmp BYTE PTR [rdx], 0x0
                target.push(0x80);
                target.push(0x3e);
                target.push(0x00);
                // je rel32
                target.push(0x0f);
                target.push(0x84);
                emit_dword(&mut target, 0x00);
                
                // save the target address
                stack.push(target.len());
            },
            ByteCode::JumpIfNotZero => {
                // cmp BYTE PTR [rdx], 0x0
                target.push(0x80);
                target.push(0x3e);
                target.push(0x00);
                // jne rel32
                target.push(0x0f);
                target.push(0x85);

                let address: usize = stack.pop().unwrap();
                let offset: i32 = (target.len() + 4 - address) as i32;
                emit_dword(&mut target, -offset as u32);

                target[address - 4] = offset as u8;
                target[address - 3] = (offset >> 8) as u8;
                target[address - 2] = (offset >> 16) as u8;
                target[address - 1] = (offset >> 24) as u8;
            },
        }
    }

    if stack.len() != 0 {
        panic!("Missing ]");
    }

    // mov eax, 0x3c
    target.push(0xb8);
    emit_dword(&mut target, 0x3c);
    // xor edi, edi
    target.push(0x31);
    target.push(0xff);
    // syscall
    target.push(0x0f);
    target.push(0x05);
    
    target
}

fn emit_dword(target: &mut Vec<u8>, value: u32) {
    let mut value = value;
    for _ in 0..4 {
        target.push(value as u8);
        value = value >> 8;
    }
}

#[repr(C)]
struct ElfHeader {
    e_ident: [u8; 16],  // ELF identification
    e_type: u16,        // Object file type
    e_machine: u16,     // Machine type
    e_version: u32,     // Object file version
    e_entry: u64,       // Entry point address
    e_phoff: u64,       // Program header offset
    e_shoff: u64,       // Section header offset
    e_flags: u32,       // Processor-specific flags
    e_ehsize: u16,      // ELF header size
    e_phentsize: u16,   //
    e_phnum: u16,       //
    e_shentsize: u16,   // Size of program header entries
    e_shnum: u16,       // Number of section header entries
    e_shstrndx: u16,    // Section name string table index
}

#[repr(C)]
struct ProgramHeader {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

// #[repr(C)]
// struct SectionHeader {
//     sh_name: u32,
//     sh_type: u32,
//     sh_flags: u64,
//     sh_addr: u64,
//     sh_offset: u64,
//     sh_size: u64,
//     sh_link: u32,
//     sh_info: u32,
//     sh_addralign: u64,
//     sh_entsize: u64,
// }

fn write_elf(target_code: &Vec<u8>, filename: &str) -> Result<(), Box<dyn Error>> {
    let mut elf = File::create(filename)?;

    let ehsize: u16 = 64;
    let phentsize: u16 = 56;

    let elf_header = ElfHeader {
        e_ident: [0x7f, 0x45, 0x4c, 0x46, 
            2, 1, 1, 0,
            0, 0, 0, 0, // padding
            0, 0, 0, 0],
        e_type: 2,
        e_machine: 0x3e,
        e_version: 1,
        e_entry: 0x400000 + (ehsize + phentsize) as u64,
        e_phoff: ehsize as u64,
        e_shoff: 0, // section header not needed
        e_flags: 0, // ignored for x86
        e_ehsize: ehsize,
        e_phentsize: phentsize,
        e_phnum: 1,
        e_shentsize: 0,
        e_shnum: 0,
        e_shstrndx: 0,
    };

    let elf_bytes: [u8; std::mem::size_of::<ElfHeader>()] = unsafe { 
        std::mem::transmute(elf_header) 
    };
    elf.write_all(&elf_bytes)?;

    let program_header = ProgramHeader {
        p_type: 1, // loadable segment
        p_flags: 5, // read and execute
        p_offset: 0,
        p_vaddr: 0x400000, // virtual address
        p_paddr: 0x400000, // reserved for systems with physical addressing
        p_filesz: (ehsize + phentsize) as u64 + target_code.len() as u64,
        p_memsz: (ehsize + phentsize) as u64 + target_code.len() as u64,
        p_align: 0x1000,
    };

    let ph_bytes: [u8; std::mem::size_of::<ProgramHeader>()] = unsafe {
        std::mem::transmute(program_header)
    };
    elf.write_all(&ph_bytes)?;


    elf.write_all(target_code)?;
    
    Ok(())
}