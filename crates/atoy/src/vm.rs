use atoy_macros::register_fns;

use crate::{
    builtin, parser::{Func, OpCode, Table, Value},
};
use std::{
    cell::RefCell,
    collections::hash_map::HashMap,
    fmt::{Debug, Display, Formatter},
    ops::RangeInclusive,
    panic, println,
    rc::Rc,
    write,
};

macro_rules! impl_try_from_value {
    ($from_type:ident, $type:ident) => {
        impl TryFrom<&Value> for $type {
            type Error = RuntimeError;
            fn try_from(value: &Value) -> Result<Self, Self::Error> {
                match value {
                    Value::$from_type(n) => match $type::try_from(*n) {
                        Ok(n) => Ok(n),
                        Err(e) => Err(RuntimeError::OverflowError {
                            required: stringify!($type).to_owned(),
                            found: e.to_string(),
                        }),
                    },
                    _ => Err(RuntimeError::TypeError {
                        expected: ValueType::$from_type,
                        found: ValueType::from(value),
                        thrower: None,
                    }),
                }
            }
        }
    };
}

macro_rules! impl_try_from_value_no_overflow {
    ($from_type:ident, $type:ty) => {
        impl TryFrom<&Value> for $type {
            type Error = RuntimeError;
            fn try_from(value: &Value) -> Result<Self, Self::Error> {
                match value {
                    Value::$from_type(n) => Ok(<$type>::from(*n)),
                    _ => Err(RuntimeError::TypeError {
                        expected: ValueType::$from_type,
                        found: ValueType::from(value),
                        thrower: None,
                    }),
                }
            }
        }
    };
}

// 适用于，Rc<T>且T不是Copy
macro_rules! impl_try_from_value_rc {
    ($from_type:ident, $type:ty) => {
        impl TryFrom<&Value> for $type {
            type Error = RuntimeError;
            fn try_from(value: &Value) -> Result<Self, Self::Error> {
                match value {
                    Value::$from_type(n) => Ok(<$type>::from(&**n)),
                    _ => Err(RuntimeError::TypeError {
                        expected: ValueType::$from_type,
                        found: ValueType::from(value),
                        thrower: None,
                    }),
                }
            }
        }
    };
}

macro_rules! impl_try_from_value_for_rc_refcell {
    
    ($from_type:ident, $type:ty) => {
        impl TryFrom<&Value> for Rc<RefCell<$type>> {
            type Error = RuntimeError;
            fn try_from(value: &Value) -> Result<Self, Self::Error> {
                match value {
                    Value::$from_type(n) => Ok(n.clone()),
                    _ => Err(RuntimeError::TypeError {
                        expected: ValueType::$from_type,
                        found: ValueType::from(value),
                        thrower: None,
                    }),
                }
            }
        }
    };
}

// 可恶的孤儿规则（
impl_try_from_value_no_overflow!(Integer, i64);
impl_try_from_value!(Integer, i32);
impl_try_from_value!(Integer, i16);
impl_try_from_value!(Integer, i8);
impl_try_from_value!(Integer, u64);
impl_try_from_value!(Integer, u32);
impl_try_from_value!(Integer, u16);
impl_try_from_value!(Integer, u8);
impl_try_from_value!(Integer, isize);
impl_try_from_value!(Float, f64);
// 不是为什么f32没有TryFrom<f64>
// impl_try_from_value!(Float, f32);
impl_try_from_value_no_overflow!(Bool, bool);
impl_try_from_value_rc!(String, String);

impl_try_from_value_for_rc_refcell!(Array, Vec<Value>);
impl_try_from_value_for_rc_refcell!(Table, Table);

impl<T> From<T> for Value
where T: Fn(Args) -> RuntimeResult<Value> + 'static
{
    fn from(value: T) -> Self {
        Self::BuiltInFunc(Rc::new(value))
    }
}

// 为所有能无损转为 i64 的整数类型实现 From
macro_rules! impl_from_int_for_value {
    ($($int:ty),*) => {
        $(
            impl From<$int> for Value {
                fn from(n: $int) -> Self {
                    Value::Integer(n.into())
                }
            }
        )*
    };
}

impl_from_int_for_value!(i8, i16, i32, i64, u8, u16, u32);

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(Rc::new(value))
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::String(Rc::new(value.to_owned()))
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}


pub struct Args {
    pub values: Vec<Value>,
}

impl Args {
    pub fn new(values: Vec<Value>) -> Self {
        Self { values }
    }
    pub fn get_arg_into<'a, T: TryFrom<&'a Value, Error = RuntimeError>>(
        &'a self,
        i: usize,
    ) -> RuntimeResult<T> {
        if let Some(arg) = self.values.get(i) {
            Ok(arg.try_into()?)
        } else {
            Err(RuntimeError::ParamError {
                expected: ExpectedParamCount::Constant(i),
                found: self.values.len(),
            })
        }
    }
    pub fn get_arg(&self, i: usize) -> RuntimeResult<&Value> {
        if let Some(arg) = self.values.get(i) {
            Ok(arg)
        } else {
            Err(RuntimeError::ParamError {
                expected: ExpectedParamCount::Constant(i),
                found: self.values.len(),
            })
        }
    }
    pub fn ensure_len(&self, len: usize) -> RuntimeResult<()> {
        if self.values.len() != len {
            Err(RuntimeError::ParamError {
                expected: ExpectedParamCount::Constant(len),
                found: self.values.len(),
            })
        } else {
            Ok(())
        }
    }
    pub fn ensure_len_ranged(&self, len: RangeInclusive<usize>) -> RuntimeResult<()> {
        let current_len = self.values.len();
        if !len.contains(&current_len) {
            Err(RuntimeError::ParamError {
                expected: ExpectedParamCount::Range(len.clone()),
                found: current_len,
            })
        } else {
            Ok(())
        }
    }
}

pub enum ValueType {
    Integer,
    Float,
    Bool,
    String,
    Function,
    Set,
    Array,
    Table,
    None,
}

impl From<&Value> for ValueType {
    fn from(value: &Value) -> Self {
        match value {
            Value::Integer(_) => Self::Integer,
            Value::Float(_) => Self::Float,
            Value::Bool(_) => Self::Bool,
            Value::Func(_) | Value::BuiltInFunc(_) => Self::Function,
            Value::None => Self::None,
            Value::String(_) => Self::String,
            Value::Set(_) => Self::Set,
            Value::Array(_) => Self::Array,
            Value::Table(_) => Self::Table,
        }
    }
}

impl Display for ValueType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::Integer => write!(f, "int"),
            Self::Float => write!(f, "float"),
            Self::Bool => write!(f, "bool"),
            Self::Function => write!(f, "function"),
            Self::None => write!(f, "none"),
            Self::String => write!(f, "string"),
            Self::Set => write!(f, "set"),
            Self::Array => write!(f, "array"),
            Self::Table => write!(f, "table"),
        }
    }
}

impl Debug for ValueType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "ValueType({})", self)
    }
}

#[derive(Debug)]
pub enum ExpectedParamCount {
    Constant(usize),
    Range(RangeInclusive<usize>),
}

impl Display for ExpectedParamCount {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Constant(n) => write!(f, "{}", n),
            Self::Range(range) => write!(f, "{}~{}", range.start(), range.end()),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum RuntimeError {
    #[error("ParamError: Function takes {expected} args but {found} were provided")]
    ParamError {
        expected: ExpectedParamCount,
        found: usize,
    },
    #[error("TypeError: {thrower_str} expected type {expected} but found {found}", thrower_str = thrower.unwrap_or(""))]
    TypeError {
        expected: ValueType,
        found: ValueType,
        thrower: Option<&'static str>,
    },
    #[error("OverflowError: Required {required} but found {found}")]
    OverflowError { required: String, found: String },
    #[error("IndexError: Index {index} out of bounds for length {len}")]
    IndexError { index: usize, len: usize },
}

pub type RuntimeResult<T> = Result<T, RuntimeError>;

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
pub struct VM {
    stack: Vec<Value>,
    code: Vec<OpCode>,
    globals: HashMap<String, Value>,
    // 可能被闭包函数捕获
    locals: Vec<Rc<RefCell<Env>>>,
    to_throw: Option<RuntimeError>,
    arr_prototype: Table
}

impl VM {
    pub fn new(code: Vec<OpCode>) -> Self {
        let mut arr_meta_raw = Table::new();
        arr_meta_raw.data.insert(
            Value::from("push"),
            Value::from(builtin::wrapped_Array_push)
        );
        arr_meta_raw.data.insert(
            Value::from("len"),
            Value::from(builtin::wrapped_Array_len)
        );
        let arr_meta = arr_meta_raw;
        let mut instance = Self {
            stack: Vec::new(),
            code,
            globals: HashMap::new(),
            locals: Vec::new(),
            to_throw: None,
            arr_prototype: arr_meta
        };
        register_fns!(
            &mut instance,
            (
                builtin::println,
                builtin::input,
                builtin::r#type,
                builtin::Table,
                builtin::Array,
                builtin::getMetatableOf,
                builtin::setMetatableOf,
                builtin::clearMetatableOf,
                builtin::getPrototypeOf,
                builtin::setPrototypeOf,
                builtin::clearPrototypeOf
            )
        );

        return instance;
    }
    pub fn register_func(&mut self, name: &str, func: Rc<dyn Fn(Args) -> RuntimeResult<Value>>) {
        self.globals
            .insert(name.to_owned(), Value::BuiltInFunc(func));
    }
    pub fn run(&mut self, codes: Option<&Vec<OpCode>>) -> Option<Value> {
        let mut ip = 0;
        while ip < codes.unwrap_or(&self.code).len() {
            if let Some(error) = &self.to_throw {
                println!("RuntimeError:\n  {}", error);
                self.to_throw = None;
                return None;
            }
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
                    let val = match v {
                        Value::Integer(n) => Value::Integer(-n),
                        Value::Float(n) => Value::Float(-n),
                        _ => {
                            self.throw(RuntimeError::TypeError {
                                expected: ValueType::Integer,
                                found: ValueType::from(&v),
                                thrower: Some("Operator '-' "),
                            });
                            continue;
                        }
                    };
                    self.stack.push(val)
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
                    self.stack.push(Value::Bool(!v.is_truthy()));
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
                                Err(e) => {
                                    self.throw(e);
                                    break;
                                }
                            };
                            self.stack.push(ret);
                        }
                        Value::Func(func) => {
                            let param_count = func.param_count;
                            if param_count != *arg_count {
                                self.throw(RuntimeError::ParamError {
                                    expected: ExpectedParamCount::Constant(param_count),
                                    found: *arg_count,
                                });
                                break;
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
                            self.throw(RuntimeError::TypeError {
                                expected: ValueType::Function,
                                found: ValueType::from(&callee),
                                thrower: Some("Function call"),
                            });
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
                OpCode::Concat => {
                    let b = self.stack.pop().expect("Stack underflow");
                    let a = self.stack.pop().expect("Stack underflow");
                    self.stack
                        .push(Value::String(Rc::new(format!("{}{}", a, b))));
                }
                OpCode::Index => {
                    let b = self.stack.pop().expect("Stack underflow");
                    let a = self.stack.pop().expect("Stack underflow");
                    match (a, b) {
                        (Value::Table(t), v) => {
                            self.stack.push(
                                t.borrow()
                                    .data
                                    .get(&v)
                                    .map(|x| x.clone())
                                    .unwrap_or(Value::None),
                            );
                        }
                        (Value::Array(a), Value::Integer(i)) => {
                            let a = a.borrow();
                            let Ok(i) = i.try_into() else {
                                self.throw(RuntimeError::OverflowError {
                                    required: "usize".to_owned(),
                                    found: "i64".to_owned(),
                                });
                                break;
                            };
                            if i > a.len() {
                                self.throw(RuntimeError::IndexError {
                                    index: i,
                                    len: a.len(),
                                });
                            } else {
                                self.stack.push(a[i].clone());
                            }
                        }
                        (Value::Array(a), k) => {
                            let proto = &self.arr_prototype;
                            if let Some(v) = proto.data.get(&k) {
                                self.stack.push(v.clone());
                            } else {
                                self.stack.push(Value::None);
                            }
                        }
                        (a, _) => {
                            self.throw(RuntimeError::TypeError {
                                expected: ValueType::Table,
                                found: ValueType::from(&a),
                                thrower: Some("Index"),
                            });
                        }
                    }
                }
                OpCode::IndexAssign => {
                    let val = self.stack.pop().expect("Stack underflow");
                    let idx = self.stack.pop().expect("Stack underflow");
                    let con = self.stack.pop().expect("Stack underflow");
                    match (con, idx) {
                        (Value::Table(t), v) => {
                            let mut t = t.borrow_mut();
                            if let Some(field) = t.data.get_mut(&v) {
                                *field = val;
                            } else {
                                t.data.insert(v, val);
                            }
                        }
                        (Value::Array(a), Value::Integer(i)) => {
                            let mut a = a.borrow_mut();
                            let Ok(i) = i.try_into() else {
                                self.throw(RuntimeError::OverflowError {
                                    required: "usize".to_owned(),
                                    found: "i64".to_owned(),
                                });
                                break;
                            };
                            if i > a.len() {
                                self.throw(RuntimeError::IndexError {
                                    index: i,
                                    len: a.len(),
                                });
                            } else if i == a.len() {
                                a.push(val);
                            } else {
                                a[i] = val;
                            }
                        }
                        (con, _) => {
                            self.throw(RuntimeError::TypeError {
                                expected: ValueType::Table,
                                found: ValueType::from(&con),
                                thrower: Some("Index"),
                            });
                        }
                    }
                }
                OpCode::Dup(count) => {
                    let tmp = self
                        .stack
                        .get(self.stack.len() - *count..)
                        .expect("stack underflow");
                    let to_dup: Vec<_> = tmp.iter().map(|x| x.clone()).collect();
                    self.stack.extend(to_dup);
                }
                OpCode::Swap2 => {
                    let a = self.stack.pop().expect("stack underflow");
                    let b = self.stack.pop().expect("stack underflow");
                    self.stack.push(a);
                    self.stack.push(b);
                } //_ => panic!("{op:?}"),
            }
        }
        if let Some(error) = &self.to_throw {
            println!("RuntimeError:\n  {}", error);
            self.to_throw = None;
            return None;
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
    pub fn throw(&mut self, error: RuntimeError) {
        self.to_throw = Some(error);
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
