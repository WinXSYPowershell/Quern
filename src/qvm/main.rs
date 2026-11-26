use std::env;
use std::fs;
use std::process;

mod errors;
mod instructions;
mod qvm;
mod parser;
mod gui_engine;

use qvm::QVM;
use parser::Parser;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 3 {
        eprintln!("Usage: {} [--Check] [--Verbose] --Run <filename.qb> [-- <program_args...>]", args[0]);
        process::exit(1);
    }

    let mut check_mode = false;
    let mut verbose_mode = false;
    let mut filename = String::new();
    let mut run_flag_found = false;
    let mut program_args: Vec<String> = Vec::new();

    // Simple argument parser
    let mut i = 1;
    let mut passed_double_dash = false;
    while i < args.len() {
        // Once we see "--", everything after is program arguments
        if passed_double_dash {
            program_args.push(args[i].clone());
        } else {
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
                "--" => {
                    passed_double_dash = true;
                }
                _ => {
                    // Ignore unknown flags or treat as error depending on strictness
                }
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
                let mut vm = QVM::new();
                vm.set_program_args(program_args);
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
