use std::env;
use std::fs;
use std::process;

mod errors;
mod instructions;
mod vm;
mod parser;

use errors::SyntaxError;
use instructions::{Program, Instruction, ArithmeticOp, ComparisonOp};
use vm::VM;
use parser::Parser;

    let args: Vec<String> = env::args().collect();
    
    if args.len() < 3 {
        eprintln!("Usage: {} [--Check] [--Verbose] --Run <filename.qb>", args[0]);
        process::exit(1);
    }

    let mut check_mode = false;
    let mut verbose_mode = false;
    let mut filename = String::new();
    let mut run_flag_found = false;

    // Simple argument parser
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--Check" => check_mode = true,
            "--Verbose" => verbose_mode = true,
            "--Run" => {
                run_flag_found = true;
                if i + 1 < args.len() {
                    filename = args[i+1].clone();
                    i += 1; // Skip next arg as it is the filename
                } else {
                    eprintln!("Error: --Run requires a filename");
                    process::exit(1);
                }
            }
            _ => {
                // Ignore unknown flags or treat as error depending on strictness
            }
        }
        i += 1;
    }

    if !run_flag_found || filename.is_empty() {
        eprintln!("Error: Missing --Run <filename>");
        process::exit(1);
    }

    let content = match fs::read_to_string(&filename) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read file '{}': {}", filename, e);
            process::exit(1);
        }
    };

    let parser = Parser::new(&content);
    
    match parser.parse() {
        Ok(program) => {
            if check_mode {
                println!("Syntax Check Passed: No errors detected in {}", filename);
            } else {
                let mut vm = VM::new();
                vm.execute(&program);
            }
        }
        Err(err) => {
            if verbose_mode {
                eprintln!("{}", err.format_verbose(&filename));
            } else {
                eprintln!("{}", err.format_normal(&filename));
            }
            process::exit(1);
        }
    }
}
