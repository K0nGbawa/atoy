use atoy::parser::ParseError;
use atoy::{lexer::Lexer, parser::Compiler, parser::Parser, vm::VM};
use std::rc::Rc;
use std::{fs::read_to_string, println};
use std::{io, io::Write, print};

fn repl() -> anyhow::Result<()> {
    println!(
        "Atoy v{} on {} {}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    println!("Type exit() to exit the REPL.");
    let mut buffer = String::new();
    let opcodes = Vec::new();
    let mut vm = VM::new(opcodes);
    vm.register_func(
        "exit",
        Rc::new(|_args| {
            std::process::exit(0);
        }),
    );
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
        match res {
            Ok(tokens) => {

                let mut parser = Parser::new(tokens);
                let parse_result = parser.parse();
                println!("{:?}", parse_result);
                match parse_result {
                    Ok(tree) => {
                        let mut compiler = Compiler::new();
                        vm.replace_code(compiler.compile(&tree));
                        vm.peek_code();
                        let res = vm.run(None);
                        match res {
                            None => {}
                            Some(val) => println!("{}", val),
                        }
                        buffer.clear()
                    }
                    Err(err) => match &err {
                        ParseError::UnexpectedEof => continue,
                        ParseError::ExpectedToken(_e, a) => {
                            if a == "EOF" {
                                continue;
                            } else {
                                println!("Syntax Error: {}", err);
                                buffer.clear();
                                continue;
                            }
                        }
                        _ => {
                            println!("Syntax Error: {:}", err);
                            buffer.clear();
                            continue;
                        }
                    },
                }
            }
            Err(e) => {
                println!("Syntax Error: {:}", e);
                buffer.clear();
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    println!("{:?}", args);
    if args.len() == 1 {
        let _ = repl(); // 实际上是Never
        return anyhow::Ok(());
    }
    let path = &args[1];
    let code = read_to_string(path)?;

    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let expr = parser.parse()?;
    let mut compiler = Compiler::new();
    let opcodes = compiler.compile(&expr);
    println!("{:?}", opcodes);
    let mut vm = VM::new(opcodes);
    let res = vm.run(None);
    if let Some(val) = res {
        println!("Program exited with result: {}", val);
    } else {
        println!("Program exited with no result");
    }
    anyhow::Ok(())
}
