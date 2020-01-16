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
    
    let bytecode = generate_bytecode(&source)?;
    
    // optimization passes
    let bytecode = merge_operations(bytecode);
    let bytecode = reorder_pointer_moves(bytecode);
    let bytecode = remove_nops(bytecode);

    let assembly_filename = format!("{}.s", config.outfile);
    let object_filename = format!("{}.o", config.outfile);

    generate_assembly(&assembly_filename, &bytecode)?;

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

#[derive(Copy, Clone)]
enum ByteCode {
    NoOperation,
    MovePointer{ offset: i64 },
    AddToCell{ value: i64, offset: i64 },
    WriteByte{ offset: i64 },
    ReadByte{ offset: i64 },
    JumpIfZero{ offset: i64 },
    JumpIfNotZero {offset: i64 },
}

fn generate_bytecode(source: &str) -> Result<Vec<ByteCode>, &'static str> {
    let mut bytecode: Vec<ByteCode> = Vec::new();
    let mut counter = 0;

    for instruction in source.chars() {
        match instruction {
            '>' => bytecode.push(ByteCode::MovePointer{ offset: 1 }),
            '<' => bytecode.push(ByteCode::MovePointer{ offset: -1 }),
            '+' => bytecode.push(ByteCode::AddToCell{ value: 1, offset: 0 }),
            '-' => bytecode.push(ByteCode::AddToCell{ value: -1, offset: 0 }),
            '.' => bytecode.push(ByteCode::WriteByte{ offset: 0 }),
            ',' => bytecode.push(ByteCode::ReadByte{ offset: 0 }),
            '[' => {
                bytecode.push(ByteCode::JumpIfZero{ offset: 0 });
                counter += 1;
            },
            ']' => {
                bytecode.push(ByteCode::JumpIfNotZero{ offset: 0 });
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

    Ok(bytecode)
}

fn merge_operations(bytecode: Vec<ByteCode>) -> Vec<ByteCode> {
    let mut optimized: Vec<ByteCode> = Vec::new();

    for operation in bytecode {
        let previous = optimized.pop().unwrap_or_else(|| ByteCode::NoOperation);

        match (operation, previous) {
            (ByteCode::MovePointer{ offset: u }, 
                ByteCode::MovePointer{ offset: v }) => {
                optimized.push(ByteCode::MovePointer{ offset: u + v });
            },
            (ByteCode::AddToCell{ value: x, offset: u}, 
                ByteCode::AddToCell{ value: y, offset: v}) => {
                    if u == v {
                        optimized.push(ByteCode::AddToCell{
                            value: x + y,
                            offset: u,
                        });
                    }
                    else {
                        optimized.push(previous);
                        optimized.push(operation);
                    }
            },
            _ => {
                optimized.push(previous);
                optimized.push(operation);
            },
        }
    }

    optimized
}

fn reorder_pointer_moves(bytecode: Vec<ByteCode>) -> Vec<ByteCode> {
    let mut optimized: Vec<ByteCode> = Vec::new();

    let mut offset = 0;
    let mut stack: Vec<i64> = Vec::new();

    for operation in bytecode {
        match operation {
            ByteCode::MovePointer{ offset: u } => {
                offset += u;
            },
            ByteCode::AddToCell{ value: x, offset: u } => {
                optimized.push(ByteCode::AddToCell{
                    value: x,
                    offset: u + offset,
                });
            },
            ByteCode::WriteByte{ offset: u } => {
                optimized.push(ByteCode::WriteByte{ 
                    offset: u + offset
                });
            },
            ByteCode::ReadByte{ offset: u } => {
                optimized.push(ByteCode::ReadByte{ 
                    offset: u + offset
                });
            },
            ByteCode::JumpIfZero{ offset: u } => {
                stack.push(offset);
                optimized.push(ByteCode::JumpIfZero{ 
                    offset: u + offset
                });
            },
            ByteCode::JumpIfNotZero{ offset: u } => {
                let prev_offset = stack.pop().unwrap();
                optimized.push(ByteCode::MovePointer{ 
                    offset : offset - prev_offset,
                });
                offset = prev_offset;
                optimized.push(ByteCode::JumpIfNotZero{ 
                    offset: u + offset 
                });
            },
            _ => (),
        }
    }

    optimized
}

fn remove_nops(bytecode: Vec<ByteCode>) -> Vec<ByteCode> {
    let mut optimized: Vec<ByteCode> = Vec::new();

    for operation in bytecode {
        match operation {
            ByteCode::MovePointer{ offset: u } => {
                if u != 0 {
                    optimized.push(operation);
                }
            },
            ByteCode::AddToCell{ value: x, offset: _ } => {
                if x != 0 {
                    optimized.push(operation);
                }
            },
            _ => optimized.push(operation),
        }
    }

    optimized
}

fn generate_assembly(filename: &String, bytecode: &Vec<ByteCode>) -> Result<(), Box<dyn Error>> {
    let mut out = File::create(filename)?;

    let mut count = 0;
    let mut stack: Vec<i64> = Vec::new();

    writeln!(out, "section .text")?;
    writeln!(out, "global _start")?;
    writeln!(out, "_start:")?;
    writeln!(out, "sub rsp, 1")?;

    // set register in advance for system calls
    writeln!(out, "mov edx, 1")?;
    
    for instruction in bytecode {
        match instruction {
            ByteCode::NoOperation => (),
            ByteCode::MovePointer{ offset: u } => {
                writeln!(out, "sub rsp, {}", u)?;
            },
            ByteCode::AddToCell{ value: x, offset: u } => {
                writeln!(out, "add BYTE [rsp-{}], {}", u, x)?;
            },
            ByteCode::WriteByte{ offset: u } => {
                writeln!(out, "mov eax, 1")?;
                writeln!(out, "mov edi, 1")?;
                writeln!(out, "lea rsi, [rsp-{}]", u)?;
                writeln!(out, "syscall")?;
            },
            ByteCode::ReadByte{ offset: u } => {
                writeln!(out, "xor eax, eax")?;
                writeln!(out, "xor edi, edi")?;
                writeln!(out, "lea rsi, [rsp-{}]", u)?;
                writeln!(out, "syscall")?;
            },
            ByteCode::JumpIfZero{ offset: u } => {
                writeln!(out, "cmp BYTE [rsp-{}], 0", u)?;
                writeln!(out, "je L{}_", count)?;
                writeln!(out, "L{}:", count)?;
                stack.push(count);
                count += 1;
            },
            ByteCode::JumpIfNotZero{ offset: u } => {
                writeln!(out, "cmp BYTE [rsp-{}], 0", u)?;
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