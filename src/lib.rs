use std::error::Error;
use std::fs::{self, File};
use std::io::prelude::Write;
use std::process::Command;

pub struct Config {
    pub infile: String,
    pub outfile: String,
}

impl Config {
    pub fn new(mut args: std::env::Args) -> Result<Config, &'static str> {
        args.next();

        let infile = match args.next() {
            Some(arg) => arg,
            None => return Err("no input file given"),
        };

        let outfile = match args.next() {
            Some(arg) => arg,
            None => return Err("no output file given"),
        };

        Ok(Config { infile, outfile })
    }
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let source = fs::read_to_string(&config.infile)?;
    let intermediate_code = generate_intermediate_code(&source)?;
    // optimization intermediate code

    let assembly_filename = format!("{}.s", config.outfile);
    let object_filename = format!("{}.o", config.outfile);

    generate_assembly(&assembly_filename, &intermediate_code)?;

    // assemble
    Command::new("nasm")
            .arg("-felf64")
            .arg(&assembly_filename)
            .arg("-o")
            .arg(&object_filename)
            .output()
            .expect("nasm failed to start");

    // link
    Command::new("ld")
            .arg(&object_filename)
            .arg("-o")
            .arg(&config.outfile)
            .output()
            .expect("ld failed to start");

    Ok(())
}

enum ByteCode {
    MovePointer(i64),
    AddToCell(i64),
    WriteByte,
    ReadByte,
    JumpIfZero,
    JumpIfNotZero,
}

fn generate_intermediate_code(source: &str) -> Result<Vec<ByteCode>, &'static str> {
    let mut intermediate: Vec<ByteCode> = Vec::new();

    let mut counter = 0;

    for instruction in source.chars() {
        match instruction {
            '>' => intermediate.push(ByteCode::MovePointer(1)),
            '<' => intermediate.push(ByteCode::MovePointer(-1)),
            '+' => intermediate.push(ByteCode::AddToCell(1)),
            '-' => intermediate.push(ByteCode::AddToCell(-1)),
            '.' => intermediate.push(ByteCode::WriteByte),
            ',' => intermediate.push(ByteCode::ReadByte),
            '[' => {
                intermediate.push(ByteCode::JumpIfZero);
                counter += 1;
            },
            ']' => {
                intermediate.push(ByteCode::JumpIfNotZero);
                if counter == 0 {
                    return Err("syntax error, missing  opening bracket");
                }
                counter -= 1;
            }
            _ => (),
        }
    }

    if counter != 0 {
        return Err("syntax error, missing closing bracket");
    }

    Ok(intermediate)
}

fn generate_assembly(filename: &String, intermediate: &Vec<ByteCode>) -> Result<(), Box<dyn Error>> {
    let mut out = File::create(filename)?;

    let mut count = 0;
    let mut stack: Vec<i64> = Vec::new();

    writeln!(out, "section .text")?;
    writeln!(out, "global _start")?;
    writeln!(out, "_start:")?;
    writeln!(out, "sub rsp, 0x8000")?;
    writeln!(out, "mov rsi, rsp")?;
    
    for instruction in intermediate {
        match instruction {
            ByteCode::MovePointer(x) => writeln!(out, "add rsi, {}", x)?,
            ByteCode::AddToCell(x) => writeln!(out, "add BYTE [rsi], {}", x)?,
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
                writeln!(out, "cmp BYTE [rsi], 0")?;
                writeln!(out, "je L{}_", count)?;
                writeln!(out, "L{}:", count)?;
                stack.push(count);
                count += 1;
            },
            ByteCode::JumpIfNotZero => {
                writeln!(out, "cmp BYTE [rsi], 0")?;
                
                let count = stack.pop().unwrap();
                writeln!(out, "jne L{}", count)?;
                writeln!(out, "L{}_:", count)?;
            },
        }
    }

    writeln!(out, "mov eax, 0x3c")?;
    writeln!(out, "xor edi, edi")?;
    writeln!(out, "syscall")?;
    
    Ok(())
}