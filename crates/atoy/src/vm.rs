use atoy_macros::{register_fns, register_methods};

use crate::{
    builtin::{self, repr, setMetatableOf, setPrototypeOf}, ops::{ self,
        call, index_table, internal_call, ops_add, ops_concat, ops_div, ops_eq, ops_gt, ops_gte, ops_lt, ops_lte, ops_mod, ops_mul, ops_ne, ops_sub,
    }, parser::{Func, OpCode, Table, Value},
};
use std::{
    cell::RefCell,
    collections::hash_map::HashMap,
    fmt::{Debug, Display, Formatter},
    ops::RangeInclusive,
    panic, println,
    rc::Rc,
    vec, write,
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
where
    T: Fn(Args, &mut VM) -> RuntimeResult<Value> + 'static,
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

impl From<Table> for Value {
    fn from(value: Table) -> Self {
        Self::Table(Rc::new(RefCell::new(value)))
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

#[derive(Clone)]
pub enum ValueType {
    Integer,
    Float,
    Bool,
    String,
    Function,
    Set,
    Array,
    Table,
    Union(Box<ValueType>, Vec<ValueType>),
    Two(Box<ValueType>, Box<ValueType>),
    None,
}

impl ValueType {
    pub fn float_or_int_tuple() -> Self {
        Self::two(
            Self::Union(Box::new(Self::Float), vec![Self::Integer]),
            Self::Union(Box::new(Self::Float), vec![Self::Integer]),
        )
    }
    pub fn two(a: ValueType, b: ValueType) -> Self {
        Self::Two(Box::new(a), Box::new(b))
    }
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

impl From<(&Value, &Value)> for ValueType {
    fn from(value: (&Value, &Value)) -> Self {
        Self::Two(Box::new(value.0.into()), Box::new(value.1.into()))
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
            Self::Union(last, others) => {
                for each in others {
                    write!(f, "{} | ", each)?;
                }
                write!(f, "{}", last)
            }
            Self::Two(a, b) => write!(f, "({a}, {b})"),
        }
    }
}

impl Debug for ValueType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "ValueType({})", self)
    }
}

#[derive(Debug, Clone)]
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

#[derive(thiserror::Error, Debug, Clone)]
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
    #[error("OperatorNotSupportedError: Operator `{op}` is not supported for `{r}`", r = repr(table))]
    OperatorNotSupportedError { op: String, table: Value },
}

pub type RuntimeResult<T> = Result<T, RuntimeError>;

macro_rules! gen_eq_ne_op {
    ($a: expr, $b: expr, $op:tt) => {
        Ok(Value::from($a $op $b))
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
        if idx >= self.vars.len() {
            // 这里之前不知道resize也可能缩小Vec，警钟撅烂）
            self.vars.resize(idx + 1, Value::None);
        }
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
    array_prototype: Rc<RefCell<Table>>,
    string_prototype: Rc<RefCell<Table>>,
}

impl VM {
    pub fn new(code: Vec<OpCode>) -> Self {
        let mut arr_meta_raw = Table::new();
        let mut str_meta_raw = Table::new();
        let mut ops_raw = Table::new();

        register_methods!(
            &mut str_meta_raw,
            (
                builtin::String_len,
                builtin::String_toInteger,
                builtin::String_upper,
                builtin::String_lower,
                builtin::String_from
            )
        );

        register_methods!(
            &mut arr_meta_raw,
            (
                builtin::Array_len,
                builtin::Array_push,
                builtin::Array_pop,
                builtin::Array_new,
                builtin::Array_join,
                builtin::Array_map
            )
        );
        
        register_methods!(
            &mut ops_raw,
            (
                ops::ops_add,
                ops::ops_concat,
                ops::ops_div,
                ops::ops_eq,
                ops::ops_gt,
                ops::ops_gte,
                ops::ops_lt,
                ops::ops_lte,
                ops::ops_mod,
                ops::ops_mul,
                ops::ops_ne,
                ops::ops_sub
            )
        );

        let mut instance = Self {
            stack: Vec::new(),
            code,
            globals: HashMap::new(),
            locals: Vec::new(),
            to_throw: None,
            array_prototype: Rc::new(RefCell::new(arr_meta_raw)),
            string_prototype: Rc::new(RefCell::new(str_meta_raw)),
        };

        instance.globals.insert(
            String::from("Array"),
            Value::Table(instance.array_prototype.clone()),
        );
        instance.globals.insert(
            String::from("String"),
            Value::Table(instance.string_prototype.clone()),
        );
        instance.globals.insert(
            String::from("Ops"),
            Value::Table(Rc::new(RefCell::new(ops_raw))),
        );

        register_fns!(
            &mut instance,
            (
                builtin::println,
                builtin::repr,
                builtin::input,
                builtin::r#type,
                builtin::Table,
                builtin::getMetatableOf,
                builtin::setMetatableOf,
                builtin::clearMetatableOf,
                builtin::getPrototypeOf,
                builtin::setPrototypeOf,
                builtin::clearPrototypeOf,
                ops::toString
            )
        );

        return instance;
    }
    pub fn register_func(
        &mut self,
        name: &str,
        func: Rc<dyn Fn(Args, &mut Self) -> RuntimeResult<Value>>,
    ) {
        self.globals
            .insert(name.to_owned(), Value::BuiltInFunc(func));
    }
    pub fn run(&mut self, codes: Option<&Vec<OpCode>>) -> RuntimeResult<Value> {
        let mut ip = 0;
        while ip < codes.unwrap_or(&self.code).len() {
            if let Some(error) = &self.to_throw {
                return Err(error.clone());
            }
            let op = &codes.unwrap_or(&self.code)[ip];
            // println!(
            //     "|\t\t\t\t\t栈\t[{}]",
            //     self.stack
            //         .iter()
            //         .map(|v| repr(v))
            //         .collect::<Vec<_>>()
            //         .join(", ")
            // );
            // {
            //     if let Some(l) = self.locals.last() {
            //         println!(
            //             "|\t\t\t\t\t局部\t[{}]",
            //             l.borrow()
            //                 .vars
            //                 .iter()
            //                 .map(|v| repr(v))
            //                 .collect::<Vec<_>>()
            //                 .join(", ")
            //         );
            //     }
            // }
            // println!("| {} {:?}", ip, op);
            ip += 1;
            match op {
                OpCode::Push(value) => self.stack.push(value.clone()),
                OpCode::Add => self.bin_op(ops_add)?,
                OpCode::Sub => self.bin_op(ops_sub)?,
                OpCode::Mul => self.bin_op(ops_mul)?,
                OpCode::Div => self.bin_op(ops_add)?,
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
                OpCode::Eq => self.bin_op(ops_eq)?,
                OpCode::NEq => self.bin_op(ops_ne)?,
                OpCode::Gt => self.bin_op(ops_gt)?,
                OpCode::Lt => self.bin_op(ops_lt)?,
                OpCode::Gte => self.bin_op(ops_gte)?,
                OpCode::Lte => self.bin_op(ops_lte)?,
                OpCode::Concat => self.bin_op(ops_concat)?,
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
                    let v = internal_call(&callee, args, self)?;
                    self.stack.push(v);
                }
                OpCode::New(arg_count) => {
                    let mut args = self.stack.split_off(self.stack.len() - arg_count);
                    let cls = self.stack.pop().expect("stack underflow");
                    match cls {
                        Value::Table(cls) => {
                            let mut new_obj = Table::new();
                            let meta = {
                                cls.borrow()
                                    .meta
                                    .as_ref()
                                    .expect("Class should have a metatable")
                                    .clone()
                            };
                            let meta_ref = meta.borrow();
                            let new_method = meta_ref.data.get(&Value::from("new"));
                            new_obj.meta = Some(meta.clone());
                            new_obj.prototype = Some(cls);
                            args.insert(0, Value::from(new_obj));
                            let res = match new_method {
                                Some(Value::Func(func)) => self.call(func, &args),
                                Some(Value::BuiltInFunc(fp)) => fp(Args::new(args.clone()), self),
                                Some(other) => Err(RuntimeError::TypeError {
                                    expected: ValueType::Function,
                                    found: ValueType::from(other),
                                    thrower: Some("New"),
                                }),
                                None => Err(RuntimeError::OperatorNotSupportedError {
                                    op: "new".to_owned(),
                                    table: args[0].clone(),
                                }),
                            };
                            res?;
                            let obj = args.into_iter().next().unwrap();
                            self.stack.push(obj);
                        }
                        callee => self.throw(RuntimeError::TypeError {
                            expected: ValueType::Table,
                            found: ValueType::from(&callee),
                            thrower: Some("New"),
                        }),
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
                    return Ok(self.stack.pop().unwrap_or(Value::None));
                }
                OpCode::Index => {
                    let b = self.stack.pop().expect("Stack underflow");
                    let a = self.stack.pop().expect("Stack underflow");
                    match (a, b) {
                        (Value::Table(t), v) => {
                            self.stack.push(index_table(t, &v));
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
                        (Value::Array(_a), k) => {
                            let proto = self.array_prototype.borrow();
                            if let Some(v) = proto.data.get(&k) {
                                self.stack.push(v.clone());
                            } else {
                                self.stack.push(Value::None);
                            }
                        }
                        (Value::String(_a), k) => {
                            let proto = self.string_prototype.borrow();
                            if let Some(v) = proto.data.get(&k) {
                                self.stack.push(v.clone());
                            } else {
                                self.stack.push(Value::None);
                            }
                        }
                        (a, _) => {
                            return Err(RuntimeError::TypeError {
                                expected: ValueType::Table,
                                found: ValueType::from(&a),
                                thrower: Some("Index"),
                            });
                        }
                    }
                }
                OpCode::IndexAssign(preserves_container) => {
                    let val = self.stack.pop().expect("Stack underflow");
                    let idx = self.stack.pop().expect("Stack underflow");
                    let con = if *preserves_container {
                        self.stack.last().expect("Stack underflow").clone()
                    } else {
                        self.stack.pop().expect("Stack underflow")
                    };
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
                                return Err(RuntimeError::OverflowError {
                                    required: "usize".to_owned(),
                                    found: "i64".to_owned(),
                                });
                            };
                            if i > a.len() {
                                return Err(RuntimeError::IndexError {
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
                            return Err(RuntimeError::TypeError {
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
                }
                OpCode::NewArray(size) => {
                    let arr;
                    if self.stack.len() < *size {
                        panic!("stack underflow");
                    } else {
                        arr = self.stack.split_off(self.stack.len() - *size);
                    }
                    self.stack.push(Value::Array(Rc::new(RefCell::new(arr))));
                }
                OpCode::NewTable => {
                    self.stack
                        .push(Value::Table(Rc::new(RefCell::new(Table::new()))));
                }
                OpCode::GetMeta => {
                    let value = self.stack.pop().expect("stack underflow");
                    match value {
                        Value::Table(table) => {
                            let table_ref = table.borrow();
                            let meta_opt = table_ref.meta.as_ref();
                            if let Some(meta) = meta_opt {
                                self.stack.push(Value::Table(meta.clone()));
                            } else {
                                return Err(RuntimeError::TypeError {
                                    expected: ValueType::Table,
                                    found: ValueType::None,
                                    thrower: Some("OpCode::GetMeta, super class metatable"),
                                });
                            }
                        }
                        other => {
                            return Err(RuntimeError::TypeError {
                                expected: ValueType::Table,
                                found: ValueType::from(&other),
                                thrower: Some("OpCode::GetMeta"),
                            });
                        }
                    }
                }
                OpCode::SetMeta => {
                    let meta = self.stack.pop().expect("stack underflow");
                    let table = self.stack.pop().expect("stack underflow");
                    match (table, meta) {
                        (Value::Table(table), Value::Table(meta)) => {
                            setMetatableOf(table, meta);
                        }
                        (table, meta) => {
                            return Err(RuntimeError::TypeError {
                                expected: ValueType::Two(
                                    Box::new(ValueType::Table),
                                    Box::new(ValueType::Table),
                                ),
                                found: ValueType::from((&table, &meta)),
                                thrower: Some("OpCode::SetMeta"),
                            });
                        }
                    }
                }
                OpCode::GetProto => {
                    let value = self.stack.pop().expect("stack underflow");
                    match value {
                        Value::Table(table) => {
                            let table_ref = table.borrow();
                            let proto_opt = table_ref.prototype.as_ref();
                            if let Some(proto) = proto_opt {
                                self.stack.push(Value::Table(proto.clone()));
                            } else {
                                return Err(RuntimeError::TypeError {
                                    expected: ValueType::Table,
                                    found: ValueType::None,
                                    thrower: Some("OpCode::GetProto, super class"),
                                });
                            }
                        }
                        other => {
                            return Err(RuntimeError::TypeError {
                                expected: ValueType::Table,
                                found: ValueType::from(&other),
                                thrower: Some("OpCode::GetProto"),
                            });
                        }
                    }
                }
                OpCode::SetProto(preserves_table) => {
                    let proto = self.stack.pop().expect("stack underflow");
                    let table = self.stack.pop().expect("stack underflow");
                    match (table, proto) {
                        (Value::Table(table), Value::Table(meta)) => {
                            if *preserves_table {
                                self.stack.push(Value::Table(table.clone()));
                            }
                            setPrototypeOf(table, meta);
                        }
                        (table, proto) => {
                            return Err(RuntimeError::TypeError {
                                expected: ValueType::Two(
                                    Box::new(ValueType::Table),
                                    Box::new(ValueType::Table),
                                ),
                                found: ValueType::from((&table, &proto)),
                                thrower: Some("OpCode::SetProto"),
                            });
                        }
                    }
                }
                OpCode::Rot3 => {
                    // Rotate the top three elements of the stack
                    let a = self.stack.pop().expect("stack underflow");
                    let b = self.stack.pop().expect("stack underflow");
                    let c = self.stack.pop().expect("stack underflow");
                    self.stack.push(a);
                    self.stack.push(c);
                    self.stack.push(b);
                } //_ => panic!("{op:?}"),
            }
        }
        // Value::None
        Ok(self.stack.pop().unwrap_or(Value::None))
    }
    fn bin_op(
        &mut self,
        func: impl Fn(&Value, &Value, &mut VM) -> RuntimeResult<Value>,
    ) -> RuntimeResult<()> {
        let b = self.stack.pop().expect("stack underflow");
        let a = self.stack.pop().expect("stack underflow");
        let ret = func(&a, &b, self)?;
        self.stack.push(ret);
        Ok(())
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
    pub fn call(&mut self, func: &Rc<Func>, args: &Vec<Value>) -> RuntimeResult<Value> {
        let arg_count = args.len();
        let param_count = func.param_count;
        if param_count != arg_count {
            return Err(RuntimeError::ParamError {
                expected: ExpectedParamCount::Constant(param_count),
                found: arg_count,
            });
        }
        let original_level = self.locals.len();
        self.locals.extend(func.env.clone());
        let mut new_env = Env::new();
        for i in 0..arg_count {
            *new_env.get_or_new_var(i) = args[i].clone();
        }
        self.locals.push(Rc::new(RefCell::new(new_env)));
        let tmp = std::mem::take(&mut self.stack);
        let ret_val = self.run(Some(&func.code))?;
        self.locals.truncate(original_level);
        self.stack = tmp;
        Ok(ret_val)
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
