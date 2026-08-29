use crate::parser::{OpCode, Value};

pub struct VM {
    stack: Vec<Value>,
    code: Vec<OpCode>,
    ip: usize,
    globals: Vec<Value>,
}

macro_rules! gen_binop_closure {
    ($op:tt) => {
        |a, b| match (a, b) {
            (Value::Integer(a), Value::Integer(b)) => Value::Integer(a $op b),
            (Value::Float(a), Value::Float(b)) => Value::Float(a $op b),
            (Value::Integer(a), Value::Float(b)) => Value::Float((a as f64) $op b),
            (Value::Float(a), Value::Integer(b)) => Value::Float(a $op (b as f64)),
            _ => panic!("Unsupport types")
        }
    };
}

macro_rules! gen_cmpop_closure {
    ($op:tt) => {
        |a, b| match (a, b) {
            (Value::Integer(a), Value::Integer(b)) => Value::Bool(a $op b),
            (Value::Float(a), Value::Float(b)) => Value::Bool(a $op b),
            (Value::Integer(a), Value::Float(b)) => Value::Bool((a as f64) $op b),
            (Value::Float(a), Value::Integer(b)) => Value::Bool(a $op (b as f64)),
            _ => panic!("Unsupport types")
        }
    };
}

impl VM {
    pub fn new(code: Vec<OpCode>) -> Self {
        Self {
            stack: Vec::new(),
            code,
            ip: 0,
            globals: Vec::new(),
        }
    }
    pub fn run(&mut self) -> Value {
        while self.ip < self.code.len() {
            let op = &self.code[self.ip];
            self.ip += 1;
            match op {
                OpCode::Push(value) => self.stack.push(value.clone()),
                OpCode::Add => self.binary_op(gen_binop_closure!(+)),
                OpCode::Sub => self.binary_op(gen_binop_closure!(-)),
                OpCode::Mul => self.binary_op(gen_binop_closure!(*)),
                OpCode::Div => self.binary_op(gen_binop_closure!(/)),
                OpCode::Neg => {
                    let v = self.stack.pop().expect("Stack underflow");
                    self.stack.push(match v {
                        Value::Integer(n) => Value::Integer(-n),
                        Value::Float(n) => Value::Float(-n),
                        _ => panic!("Unsupport"),
                    })
                }
                OpCode::Eq => self.binary_op(gen_cmpop_closure!(==)),
                OpCode::NEq => self.binary_op(gen_cmpop_closure!(!=)),
                OpCode::Gt => self.binary_op(gen_cmpop_closure!(>)),
                OpCode::Lt => self.binary_op(gen_cmpop_closure!(<)),
                OpCode::Gte => self.binary_op(gen_cmpop_closure!(>=)),
                OpCode::Lte => self.binary_op(gen_cmpop_closure!(<=)),
                OpCode::LoadGlobal(idx) => {
                    let value = self.globals.get(*idx).unwrap_or(&Value::None).clone();
                    self.stack.push(value);
                }
                OpCode::StoreGlobal(idx) => {
                    let value = self.stack.pop().expect("Stack underflow");
                    if *idx >= self.globals.len() {
                        self.globals.resize(*idx + 1, Value::None);
                    }
                    self.globals[*idx] = value;
                }
                OpCode::Jmp(usize) => self.ip = *usize,
                OpCode::JmpIfNot(usize) => {
                    let value = self.stack.pop().expect("Stack overflow");
                    if let Value::Bool(value) = value {
                        if !value {
                            self.ip = *usize
                        }
                    }
                },
                OpCode::JmpIf(usize) => {
                    let value = self.stack.pop().expect("Stack overflow");
                    if let Value::Bool(value) = value {
                        if value {
                            self.ip = *usize
                        }
                    }
                }
                _ => panic!("{op:?}"),
            }
        }
        // Value::None
        self.stack.pop().expect("Stack underflow")
    }

    fn binary_op<F>(&mut self, op: F)
    where
        F: FnOnce(Value, Value) -> Value,
    {
        let b = self.stack.pop().expect("Stack underflow");
        let a = self.stack.pop().expect("Stack underflow");
        self.stack.push(op(a, b));
    }
}

#[cfg(test)]
mod vm_test {
    use super::*;
    use crate::{
        lexer::Lexer,
        parser::{Compiler, Parser},
    };
    #[test]
    fn vm_test() -> Result<(), Box<dyn std::error::Error>> {
        let mut lexer = Lexer::new("let a = 0; let b = 1; let tmp = 0; let count = 0; while count < 91 {tmp = b; b = a + b; a = tmp; count = count + 1; } a;".to_owned());
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        let expr = parser.parse()?;
        let opcodes = Compiler::compile_program(&expr);
        let mut vm = VM::new(opcodes);
        let res = vm.run();
        println!("{:#?}", res);
        Ok(())
    }
}
