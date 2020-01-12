use std::error::Error;
use std::fmt;
use std::fs;

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

    let ir = compile_to_ir(&source);
    
    println!("{:?}", ir);
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

impl fmt::Debug for ByteCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ByteCode::IncrementPointer => write!(f, ">"),
            ByteCode::DecrementPointer => write!(f, "<"),
            ByteCode::IncrementCell => write!(f, "+"),
            ByteCode::DecrementCell => write!(f, "-"),
            ByteCode::WriteByte => write!(f, "."),
            ByteCode::ReadByte => write!(f, ","),
            ByteCode::JumpIfZero => write!(f, "["),
            ByteCode::JumpIfNotZero => write!(f, "]"),
        }
        
    }
}

fn compile_to_ir(source: &str) -> Vec<ByteCode> {
    let mut ir: Vec<ByteCode> = Vec::new();

    for instruction in source.chars() {
        match instruction {
            '>' => ir.push(ByteCode::IncrementPointer),
            '<' => ir.push(ByteCode::DecrementPointer),
            '+' => ir.push(ByteCode::IncrementCell),
            '-' => ir.push(ByteCode::DecrementCell),
            '.' => ir.push(ByteCode::WriteByte),
            ',' => ir.push(ByteCode::ReadByte),
            '[' => ir.push(ByteCode::JumpIfZero),
            ']' => ir.push(ByteCode::JumpIfNotZero),
            _ => (),
        }
    }

    ir
}