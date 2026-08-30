
use atoy::parser::{ParseError};
use atoy::{lexer::Lexer, parser::Parser, vm::VM, parser::Compiler };
use std::rc::Rc;
use std::{fs::read_to_string, println};
use std::{io, io::{Write}, print};

fn repl() -> anyhow::Result<()> {
    let mut buffer = String::new();
    let opcodes = Vec::new();
    let mut vm = VM::new(opcodes);
    vm.add_builtin("exit", Rc::new(|_args| {
        std::process::exit(0);
    }));
    loop {
        if buffer.len() == 0 {
            print!(">>> ");
        } else {
            print!("     ");
        }
        let _ = io::stdout().flush();
        io::stdin().read_line(&mut buffer)?;
        let mut lexer = Lexer::new(buffer.clone());
        let res = lexer.tokenize();
        if let Ok(tokens) = res {
            let mut parser = Parser::new(tokens);
            let parse_result = parser.parse();
            match parse_result {
                Ok(tree) => {
                    let mut compiler = Compiler::new();
                    vm.replace_code(compiler.compile(&tree));
                    vm.peek_code();
                    let res = vm.run();
                    match res {
                        None => {},
                        Some(val) => println!("{}", val)
                    }
                    buffer.clear()
                },
                Err(err) => {
                    match err {
                        ParseError::UnexpectedEof => continue,
                        ParseError::ExpectedToken(_e, a) => {
                            if a == "EOF" {
                                continue;
                            } else {
                                println!("Syntax Error: ");
                                buffer.clear();
                                continue;
                            }
                        },
                        _ => {
                            println!("Syntax Error: {:}", err);
                            buffer.clear();
                            continue;
                        }
                    }
                }
            }
        } else {
            println!("Syntax Error: {:?}", res)
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    println!("{:?}", args);
    if args.len() == 1 {
        let _ = repl(); // 实际上是Never
        return anyhow::Ok(())
    }
    let path = &args[1];
    let code = read_to_string(path)?;

    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let expr = parser.parse()?;
    let mut compiler = Compiler::new();
    let opcodes = compiler.compile(&expr);
    let mut vm = VM::new(opcodes);
    let res = vm.run();
    if let Some(val) = res {
        println!("Program exited with result: {}", val);
    } else {
        println!("Program exited with no result");
    }
    anyhow::Ok(())
}
