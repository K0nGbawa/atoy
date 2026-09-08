use std::cell::RefCell;
use std::rc::Rc;

use crate::{
    builtin::{getMetatableOf, repr}, parser::{Table, Value}, vm::{Args, RuntimeError, RuntimeResult, VM, ValueType},
};
use atoy_macros::atoy_function;

type RRTable = Rc<RefCell<Table>>;
type RRVec = Rc<RefCell<Vec<Value>>>;

pub fn index_table(target: RRTable, key: &Value) -> Value {
    let tref = target.borrow();
    if let Some(value) = tref.data.get(key) {
        return value.clone();
    } else if let Some(proto) = &tref.prototype {
        return index_table(proto.clone(), key);
    }
    Value::None
}

pub fn internal_call(callee: &Value, args: Vec<Value>, vm: &mut VM) -> RuntimeResult<Value> {
    use Value::*;
    match callee {
        BuiltInFunc(f) => f(Args::new(args), vm),
        Func(func) => vm.call(func, &args),
        other => Err(RuntimeError::TypeError {
            expected: ValueType::Function,
            found: ValueType::from(other),
            thrower: Some("Function call"),
        }),
    }
}

#[atoy_function]
pub fn call(callee: &Value, args: RRVec, vm: &mut VM) -> RuntimeResult<Value> {
    internal_call(callee, args.borrow().clone(), vm)
}

#[inline]
fn call_meta(a: &Value, b: &Value, table: &RRTable, magic_method: &'static str, op: &'static str, vm: &mut VM) -> RuntimeResult<Value> {
    let tref = table.borrow();
    let meta = tref.meta.as_ref();
    if let Some(meta) = meta {
        let func = index_table(meta.clone(), &Value::from(magic_method));
        internal_call(&func, vec![a.clone(), b.clone()], vm)
    } else {
        Err(RuntimeError::OperatorNotSupportedError {
            op: op.to_string(),
            table: Value::Table(table.clone()),
        })
    }
}

#[inline]
fn call_meta_or(a: &Value, b: &Value, table: &RRTable, magic_method: &'static str, vm: &mut VM, fallback: impl Fn(&Value, &Value) -> Value) -> RuntimeResult<Value> {
    let tref = table.borrow();
    let meta = tref.meta.as_ref();
    if let Some(meta) = meta {
        let func = index_table(meta.clone(), &Value::from(magic_method));
        internal_call(&func, vec![a.clone(), b.clone()], vm)
    } else {
        Ok(fallback(a, b))
    }
}

macro_rules! gen_arithop {
    ($a: expr, $b: expr, $vm: expr, $op:tt, $magic_method: literal) => {
        match ($a, $b) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a $op b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a $op b)),
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Float((*a as f64) $op b)),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a $op (*b as f64))),
            (Value::Table(at), _any_value) => {
                call_meta($a, $b, at, $magic_method, stringify!(op), $vm)
            }
            (a, b) => Err(RuntimeError::TypeError {
                expected: ValueType::float_or_int_tuple(),
                found: ValueType::from((a, b)),
                thrower: Some(concat!("Operator ", stringify!($op))),
            }),
        }
    };
}

#[atoy_function(method = add)]
pub fn ops_add(a: &Value, b: &Value, vm: &mut VM) -> RuntimeResult<Value> {
    gen_arithop!(a, b, vm, +, "add")
}

#[atoy_function(method = sub)]
pub fn ops_sub(a: &Value, b: &Value, vm: &mut VM) -> RuntimeResult<Value> {
    gen_arithop!(a, b, vm, -, "sub")
}

#[atoy_function(method = mul)]
pub fn ops_mul(a: &Value, b: &Value, vm: &mut VM) -> RuntimeResult<Value> {
    gen_arithop!(a, b, vm, *, "mul")
}

#[atoy_function(method = div)]
pub fn ops_div(a: &Value, b: &Value, vm: &mut VM) -> RuntimeResult<Value> {
    gen_arithop!(a, b, vm, /, "div")
}

#[atoy_function(method = r#mod)]
pub fn ops_mod(a: &Value, b: &Value, vm: &mut VM) -> RuntimeResult<Value> {
    gen_arithop!(a, b, vm, %, "mod")
}

#[atoy_function(method = concat)]
pub fn ops_concat(a: &Value, b: &Value, vm: &mut VM) -> RuntimeResult<Value> {
    match (a, b) {
        (Value::String(a), Value::String(b)) => Ok(Value::String(Rc::new((**a).clone() + &**b))),
        (Value::Table(at), _other) => {
            call_meta(a, b, at, "concat", "..", vm)
        }
        (a, b) => Err(RuntimeError::TypeError {
            expected: ValueType::two(ValueType::String, ValueType::String),
            found: ValueType::from((a, b)),
            thrower: Some("concat"),
        }),
    }
}

macro_rules! gen_cmpop {
    ($a: expr, $b: expr, $vm: expr, $op:tt, $magic_method: literal) => {
        match ($a, $b) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Bool(a $op b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a $op b)),
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Bool((*a as f64) $op *b)),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Bool(*a $op (*b as f64))),
            (Value::Table(at), _any_value) => {
                call_meta($a, $b, at, $magic_method, stringify!(op), $vm)
            }
            (a, b) => Err(RuntimeError::TypeError {
                expected: ValueType::float_or_int_tuple(),
                found: ValueType::from((a, b)),
                thrower: Some(concat!("Operator ", stringify!($op))),
            }),
        }
    };
}

#[atoy_function(method = lt)]
pub fn ops_lt(a: &Value, b: &Value, vm: &mut VM) -> RuntimeResult<Value> {
    gen_cmpop!(a, b, vm, <, "lt")
}

#[atoy_function(method = gt)]
pub fn ops_gt(a: &Value, b: &Value, vm: &mut VM) -> RuntimeResult<Value> {
    gen_cmpop!(a, b, vm, >, "gt")
}

#[atoy_function(method = lte)]
pub fn ops_lte(a: &Value, b: &Value, vm: &mut VM) -> RuntimeResult<Value> {
    gen_cmpop!(a, b, vm, <=, "lte")
}

#[atoy_function(method = gte)]
pub fn ops_gte(a: &Value, b: &Value, vm: &mut VM) -> RuntimeResult<Value> {
    gen_cmpop!(a, b, vm, >=, "gte")
}

#[atoy_function(method = eq)]
pub fn ops_eq(a: &Value, b: &Value, vm: &mut VM) -> RuntimeResult<Value> {
    match (a, b) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Bool(a == b)),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a == b)),
        (Value::Integer(a), Value::Float(b)) => Ok(Value::Bool((*a as f64) == *b)),
        (Value::Float(a), Value::Integer(b)) => Ok(Value::Bool(*a == (*b as f64))),
        (Value::String(a), Value::String(b)) => Ok(Value::Bool(Rc::ptr_eq(a, b) || **a == **b)),
        (Value::Table(at), _any_value) => call_meta_or(a, b, at, "eq", vm, |a, b| {
            if let Value::Table(other) = b {
                Value::Bool(Rc::ptr_eq(at, other))
            } else {
                Value::Bool(false)
            }
        }),
        (a, b) => Ok(Value::Bool(a == b)),
    }
}
#[atoy_function(method = ne)]
pub fn ops_ne(a: &Value, b: &Value, vm: &mut VM) -> RuntimeResult<Value> {
    let eq_result = ops_eq(a, b, vm)?;
    Ok(Value::Bool(!eq_result.is_truthy()))
}

#[atoy_function]
pub fn toString(v: &Value, vm: &mut VM) -> RuntimeResult<Value> {
    match v {
        Value::Table(table) => {
            let tref = table.borrow();
            let meta = tref.meta.as_ref();
            if let Some(meta) = meta {
                let func = index_table(meta.clone(), &Value::from("toString"));
                internal_call(&func, vec![v.clone()], vm)
            } else {
                Ok(Value::from(repr(v)))
            }
        }
        Value::String(_) => Ok(v.clone()),
        _ => {
            Ok(Value::from(repr(v)))
        }
    }
} 

