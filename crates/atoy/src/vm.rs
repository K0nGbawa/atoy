use crate::{
    builtin,
    parser::{Func, OpCode, Value},
};
use std::{cell::RefCell, collections::hash_map::HashMap, panic, println, rc::Rc};

pub trait FromAtoyValue: Sized {
    fn from_value(v: &Value) -> Result<Self, String>;
}

pub trait IntoAtoyValue: Sized {
    fn into_value(self) -> Value;
}

impl FromAtoyValue for i32 {
    fn from_value(v: &Value) -> Result<Self, String> {
        match v {
            Value::Integer(n) => Ok(*n as i32),
            other => Err(format!("expect int, found {other}")),
        }
    }
}

impl IntoAtoyValue for i32 {
    fn into_value(self) -> Value {
        Value::Integer(self as i64)
    }
}

pub struct Args {
    pub values: Vec<Value>,
    pub pos: usize,
}

impl Args {
    pub fn new(values: Vec<Value>) -> Self {
        Self { values, pos: 0 }
    }

    pub fn take<T: FromAtoyValue>(&mut self) -> Result<T, String> {
        let v = match self.values.get(self.pos) {
            Some(v) => v,
            None => return Err("param less".to_owned()),
        };
        self.pos += 1;
        T::from_value(v)
    }
}

pub struct VM {
    stack: Vec<Value>,
    code: Vec<OpCode>,
    ip: usize,
    globals: HashMap<String, Value>,
    // 可能被闭包函数捕获
    locals: Vec<Rc<RefCell<Env>>>,
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

#[derive(Debug)]
pub struct Env {
    vars: Vec<Value>,
}

impl Env {
    fn new() -> Self {
        Self { vars: Vec::new() }
    }
    fn get_or_new_var(&mut self, idx: usize) -> &mut Value {
        self.vars.resize(idx + 1, Value::None);
        self.vars.get_mut(idx).unwrap()
    }
}

impl VM {
    pub fn new(code: Vec<OpCode>) -> Self {
        let mut instance = Self {
            stack: Vec::new(),
            code,
            ip: 0,
            globals: HashMap::new(),
            locals: Vec::new(),
        };
        builtin::__atoy_register_println(&mut instance);
        return instance;
    }
    pub fn register_func(&mut self, name: &str, func: Rc<dyn Fn(Args) -> Result<Value, String>>) {
        self.globals
            .insert(name.to_owned(), Value::BuiltInFunc(func));
    }
    pub fn run(&mut self, codes: Option<&Vec<OpCode>>) -> Option<Value> {
        let mut ip = 0;
        while ip < codes.unwrap_or(&self.code).len() {
            let op = &codes.unwrap_or(&self.code)[ip];
            ip += 1;
            // println!("{} {:?}", ip, op);
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
                OpCode::And(idx) => {
                    let v = self.stack.last().expect("Stack underflow");
                    if v.is_truthy() {
                        self.stack.pop();
                    } else {
                        ip = *idx;
                    }
                }
                OpCode::Or(idx) => {
                    let v = self.stack.last().expect("Stack underflow");
                    if v.is_truthy() {
                        ip = *idx;
                    } else {
                        self.stack.pop();
                    }
                }
                OpCode::Not => {
                    let v = self.stack.pop().expect("Stack underflow");
                    self.stack.push(Value::Bool(v.is_truthy()));
                }
                OpCode::LoadGlobal(ident) => {
                    let value = self.globals.get(ident).unwrap_or(&Value::None).clone();
                    self.stack.push(value);
                }
                OpCode::StoreGlobal(ident) => {
                    let value = self.stack.pop().expect("Stack underflow");
                    self.globals.insert(ident.clone(), value);
                }
                OpCode::Jmp(usize) => ip = *usize,
                OpCode::JmpIfNot(usize) => {
                    let value = self.stack.pop().expect("Stack overflow");
                    if let Value::Bool(value) = value {
                        if !value {
                            ip = *usize
                        }
                    }
                }
                OpCode::JmpIf(usize) => {
                    let value = self.stack.pop().expect("Stack overflow");
                    if let Value::Bool(value) = value {
                        if value {
                            ip = *usize
                        }
                    }
                }
                OpCode::Call(arg_count) => {
                    let args = self.stack.split_off(self.stack.len() - arg_count);
                    let callee = self.stack.pop().expect("stack underflow");
                    match callee {
                        Value::BuiltInFunc(func) => {
                            let ret = match func(Args::new(args)) {
                                Ok(ret) => ret,
                                Err(e) => panic!("{}", e),
                            };
                            self.stack.push(ret);
                        }
                        Value::Func(func) => {
                            let param_count = func.param_count;
                            if param_count != *arg_count {
                                panic!("Arg count does not match")
                            }
                            let original_level = self.locals.len();
                            self.locals.extend(func.env.clone());
                            let mut new_env = Env::new();
                            for i in 0..*arg_count {
                                *new_env.get_or_new_var(i) = args[i].clone();
                            }
                            self.locals.push(Rc::new(RefCell::new(new_env)));
                            let tmp = std::mem::take(&mut self.stack);
                            let ret_val = self.run(Some(&func.code)).unwrap_or(Value::None);
                            self.locals.truncate(original_level);
                            self.stack = tmp;
                            self.stack.push(ret_val);
                        }
                        _ => {
                            panic!("Not a function");
                        }
                    }
                }
                OpCode::LoadLocal(lev, idx) => {
                    let real_lev = self.locals.len().checked_sub(*lev).unwrap();
                    let fasts = self.locals.get_mut(real_lev).unwrap();
                    self.stack
                        .push(fasts.borrow_mut().get_or_new_var(*idx).clone())
                }
                OpCode::StoreLocal(lev, idx) => {
                    let real_lev = self.locals.len().checked_sub(*lev).unwrap();
                    //println!("{} {} {:?}",lev, real_lev, self.locals);
                    let fasts = self.locals.get_mut(real_lev).unwrap();
                    let value = self.stack.pop().expect("stack underflow");
                    *fasts.borrow_mut().get_or_new_var(*idx) = value;
                }
                OpCode::EnterScope => self.locals.push(Rc::new(RefCell::new(Env::new()))),
                OpCode::ExitScope => {
                    self.locals.pop();
                }
                OpCode::PushFn(param_count, opcodes) => self.stack.push(Value::Func(Rc::new(
                    Func::new(*param_count, opcodes.clone(), self.locals.clone()),
                ))),
                OpCode::Ret => {
                    self.locals.pop();
                    return self.stack.pop();
                }
                _ => panic!("{op:?}"),
            }
        }
        // Value::None
        self.stack.pop()
    }
    pub fn replace_code(&mut self, code: Vec<OpCode>) {
        self.code = code;
    }
    pub fn peek_code(&mut self) {
        println!("{:?}", self.code);
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
        let res = vm.run(None);
        println!("{:#?}", res);
        Ok(())
    }
}
