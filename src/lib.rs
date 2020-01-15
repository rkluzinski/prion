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
    let intermediate_code = generate_intermediate_code(&source);
    // optimization
    generate_asm(&intermediate_code)?;
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

    // this section should validate code as well

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

fn generate_asm(intermediate: &Vec<ByteCode>) -> Result<(), Box<dyn Error>> {
    let mut out = File::create("out.asm")?;

    writeln!(out, "_start:")?;
    writeln!(out, "sub rsp, 0x8000")?;
    writeln!(out, "mov rsi, rsp")?;
    
    for instruction in intermediate {
        match instruction {
            ByteCode::IncrementPointer => writeln!(out, "add rsi, 1")?,
            ByteCode::DecrementPointer => writeln!(out, "sub rsi, 1")?,
            ByteCode::IncrementCell => writeln!(out, "add BYTE [rsi], 1")?,
            ByteCode::DecrementCell => writeln!(out, "sub BYTE [rsi], 1")?,
            ByteCode::WriteByte => {
                writeln!(out, "mov eax, 1")?;
                writeln!(out, "mov edi, 1")?;
                writeln!(out, "mov edx, 1")?;
                writeln!(out, "syscall")?;
            },
            ByteCode::ReadByte => {
                writeln!(out, "xor eax, eax")?;
                writeln!(out, "xor edi, edi")?;
                writeln!(out, "mov edx, 1")?;
                writeln!(out, "syscall")?;
            },
            ByteCode::JumpIfZero => {
                writeln!(out, "cmp BYTE [rdx], 0")?;
                writeln!(out, "je L")?;
                writeln!(out, "L{}:", 0)?;
            },
            ByteCode::JumpIfNotZero => {
                writeln!(out, "cmp BYTE [rdx], 0")?;
                writeln!(out, "jne L")?;
                writeln!(out, "L{}:", 0)?;
            },
        }
    }

    writeln!(out, "mov eax, 0x3c")?;
    writeln!(out, "xor edi, edi")?;
    writeln!(out, "syscall")?;
    
    Ok(())
}