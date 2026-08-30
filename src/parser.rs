use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::{Display, write},
    matches,
    ops::Index,
    println,
    rc::Rc,
    write,
};

use thiserror::Error;

use crate::{
    lexer::Token,
    vm::{Args, Env},
};

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("expected '{0}', found '{1}'")]
    ExpectedToken(String, String),
    #[error("unexpected '{0}'")]
    UnexpectedToken(String),
    #[error("expected identifier, found '{0}'")]
    ExpectedIdentifier(String),
    #[error("unexpected end of input")]
    UnexpectedEof,
}

type ParseResult<T> = std::result::Result<T, ParseError>;

#[derive(Debug, PartialEq, Clone)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, PartialEq, Clone)]
pub enum CmpOp {
    Eq,
    NEq,
    Gt,
    Lt,
    Gte,
    Lte,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expr(Expr),
    Let {
        name: String,
        value: Expr,
    },
    IfElse {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },
    While {
        condition: Expr,
        block: Box<Stmt>,
    },
    Assign {
        name: String,
        value: Expr,
    },
    CompoundAssign {
        name: String,
        op: BinOp,
        value: Expr,
    },
    Block(Vec<Stmt>),
    Return(Expr)
}

#[derive(Debug, Clone)]
pub enum Expr {
    Integer(i64),
    Float(f64),
    Bool(bool),
    Ident(String),
    BinaryOp {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },
    CompareOp {
        left: Box<Expr>,
        op: CmpOp,
        right: Box<Expr>,
    },
    Unary(Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Fn(Vec<String>, Box<Stmt>),
}

#[derive(Debug, Clone)]
pub enum OpCode {
    Push(Value),
    StoreGlobal(String),
    LoadGlobal(String),
    StoreLocal(usize, usize),
    LoadLocal(usize, usize),
    Jmp(usize),
    JmpIfNot(usize),
    JmpIf(usize),
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Gt,
    Lt,
    Gte,
    Lte,
    NEq,
    Neg,
    Ret,
    Call(usize),
    EnterScope,
    ExitScope,
    PushFn(usize, Vec<OpCode>),
}

#[derive(Debug)]
pub struct Func {
    pub param_count: usize,
    pub code: Vec<OpCode>,
    pub env: Vec<Rc<RefCell<Env>>>,
}

impl Func {
    pub fn new(param_count: usize, code: Vec<OpCode>, env: Vec<Rc<RefCell<Env>>>) -> Self {
        Self {
            param_count,
            code,
            env,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RcFunc(Rc<Func>);

impl PartialEq for RcFunc {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

// impl RcFunc {
//     fn new(param_count: usize, code: Vec<OpCode>, env: Vec<Rc<Env>>) -> Self {
//         Self (Rc::new(Func { param_count, code, env }))
//     }
// }

pub enum Value {
    Float(f64),
    Integer(i64),
    Bool(bool),
    BuiltInFunc(Rc<dyn Fn(Args) -> Value>),
    Func(Rc<Func>),
    None,
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Value::*;
        match self {
            Float(n) => write!(f, "Value(Float({n}))"),
            Integer(n) => write!(f, "Value(Integer({n}))"),
            Bool(n) => write!(f, "Value(Bool({n}))"),
            BuiltInFunc(func) => write!(f, "Value(BuiltinFunc({:p}))", *func),
            Func(func) => write!(f, "Value(Func({:p}))", *func),
            None => write!(f, "Value(None)"),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        use Value::*;
        match (self, other) {
            (Float(n1), Float(n2)) => *n1 == *n2,
            (Integer(n1), Integer(n2)) => *n1 == *n2,
            (Bool(n1), Bool(n2)) => *n1 == *n2,
            (BuiltInFunc(n1), BuiltInFunc(n2)) => Rc::ptr_eq(n1, n2),
            (Func(n1), Func(n2)) => Rc::ptr_eq(n1, n2),
            (None, None) => true,
            (_, _) => false,
        }
    }
    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}

impl Clone for Value {
    fn clone(&self) -> Self {
        use Value::*;
        match self {
            Float(n) => Float(*n),
            Integer(n) => Integer(*n),
            Bool(n) => Bool(*n),
            BuiltInFunc(func) => BuiltInFunc(func.clone()),
            Func(func) => Func(func.clone()),
            None => None,
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Value::*;
        match self {
            Float(n) => write!(f, "{n}"),
            Integer(n) => write!(f, "{n}"),
            Bool(n) => write!(f, "{n}"),
            BuiltInFunc(func) => write!(f, "Builtin Function at {:p}", *func),
            Func(func) => write!(f, "Function at {:p}", *func),
            None => write!(f, "None"),
        }
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
    // 用于判断return的合法性
    in_func: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
            in_func: 0,
        }
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.position].clone();
        self.position += 1;
        token
    }

    fn expect(&mut self, expected: Token) -> ParseResult<()> {
        let found = self.advance();
        if std::mem::discriminant(&found) == std::mem::discriminant(&expected) {
            Ok(())
        } else {
            Err(ParseError::ExpectedToken(
                expected.to_string(),
                found.to_string(),
            ))
        }
    }

    fn expect_ident(&mut self) -> ParseResult<String> {
        let found = self.advance();
        if let Token::Ident(name) = found {
            Ok(name)
        } else {
            Err(ParseError::ExpectedIdentifier(found.to_string()))
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn peek_bin_op(&self) -> Option<BinOp> {
        match self.peek() {
            Token::Plus => Some(BinOp::Add),
            Token::Minus => Some(BinOp::Sub),
            Token::Star => Some(BinOp::Mul),
            Token::Slash => Some(BinOp::Div),
            _ => None,
        }
    }

    fn peek_cmp_op(&self) -> Option<CmpOp> {
        match self.peek() {
            Token::Eq => Some(CmpOp::Eq),
            Token::Gt => Some(CmpOp::Gt),
            Token::Lt => Some(CmpOp::Lt),
            Token::Gte => Some(CmpOp::Gte),
            Token::Lte => Some(CmpOp::Lte),
            Token::NEq => Some(CmpOp::NEq),
            _ => None,
        }
    }

    pub fn parse(&mut self) -> ParseResult<Vec<Stmt>> {
        let mut stmts = Vec::new();
        while *self.peek() != Token::Eof {
            let stmt = self.parse_stmt()?;
            stmts.push(stmt)
        }
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> ParseResult<Stmt> {
        match self.peek().clone() {
            Token::Let => self.parse_let_stmt(),
            Token::If => self.parse_if_stmt(),
            Token::While => self.parse_while_stmt(),
            Token::LBrace => self.parse_block(),
            Token::Return => {
                if self.in_func == 0 {
                    return Err(ParseError::UnexpectedToken("return".to_owned()));
                }
                self.advance();
                let res = Stmt::Return(self.parse_expr()?);
                self.expect(Token::Semicolon)?;
                Ok(res)
            }
            _ => {
                let left: Expr = self.parse_expr()?;
                let res = match &left {
                    Expr::Ident(name) => match self.peek() {
                        Token::Assign => self.parse_assign_stmt(name.clone())?,
                        Token::PlusAssign => self.parse_compound_assign_stmt(name.clone(), BinOp::Add)?,
                        Token::MinusAssign => self.parse_compound_assign_stmt(name.clone(), BinOp::Sub)?,
                        Token::StarAssign => self.parse_compound_assign_stmt(name.clone(), BinOp::Mul)?,
                        Token::SlashAssign => self.parse_compound_assign_stmt(name.clone(), BinOp::Div)?,
                        _ => Stmt::Expr(left),
                    },
                    _ => Stmt::Expr(left),
                };
                self.expect(Token::Semicolon)?;
                Ok(res)
            }
        }
    }

    fn parse_assign_stmt(&mut self, name: String) -> ParseResult<Stmt> {
        self.expect(Token::Assign)?;

        let value = self.parse_expr()?;

        Ok(Stmt::Assign { name, value })
    }

    fn parse_compound_assign_stmt(&mut self, name: String, op: BinOp) -> ParseResult<Stmt> {
        self.advance();

        let value = self.parse_cmp_op()?;

        Ok(Stmt::CompoundAssign { name, op, value })
    }

    fn parse_if_stmt(&mut self) -> ParseResult<Stmt> {
        self.advance();
        let condition = self.parse_expr()?;
        let then_branch = self.parse_stmt()?;
        let else_branch = match self.peek() {
            Token::Else => {
                self.advance();
                Some(self.parse_stmt()?)
            }
            _ => None,
        };
        Ok(Stmt::IfElse {
            condition,
            then_branch: Box::new(then_branch),
            else_branch: else_branch.and_then(|e| Some(Box::new(e))),
        })
    }

    fn parse_while_stmt(&mut self) -> ParseResult<Stmt> {
        self.advance();
        let condition = self.parse_expr()?;
        let block = self.parse_stmt()?;
        Ok(Stmt::While {
            condition,
            block: Box::new(block),
        })
    }

    fn parse_block(&mut self) -> ParseResult<Stmt> {
        self.expect(Token::LBrace)?;

        let mut stmts = Vec::new();
        while *self.peek() != Token::RBrace && *self.peek() != Token::Eof {
            let stmt = self.parse_stmt()?;
            stmts.push(stmt)
        }
        self.expect(Token::RBrace)?;
        Ok(Stmt::Block(stmts))
    }

    fn parse_let_stmt(&mut self) -> ParseResult<Stmt> {
        self.advance();
        let name = match self.advance() {
            Token::Ident(name) => name,
            other => return Err(ParseError::ExpectedIdentifier(other.to_string())),
        };
        self.expect(Token::Assign)?;
        let value = self.parse_expr()?;
        self.expect(Token::Semicolon)?;
        Ok(Stmt::Let { name, value })
    }

    fn parse_expr(&mut self) -> ParseResult<Expr> {
        if self.peek().clone() == Token::Fn {
            self.parse_fn()
        } else {
            self.parse_cmp_op()
        }
    }

    fn parse_fn(&mut self) -> ParseResult<Expr> {
        self.advance();
        self.expect(Token::LParen)?;
        let mut args = Vec::new();
        while *self.peek() != Token::RParen && *self.peek() != Token::Eof {
            let arg = self.expect_ident()?;
            args.push(arg);
            let token = self.peek();
            if *token == Token::Comma {
                self.advance();
            } else if !matches!(*token, Token::RParen | Token::Eof) {
                return Err(ParseError::UnexpectedToken(token.to_string()));
            }
        }
        self.expect(Token::RParen)?;
        self.in_func += 1;
        let block = self.parse_block()?;
        self.in_func -= 1;
        Ok(Expr::Fn(args, Box::new(block)))
    }

    fn parse_cmp_op(&mut self) -> ParseResult<Expr> {
        let left = self.parse_addsub()?;
        let op = match self.peek_cmp_op() {
            Some(op) => op,
            _ => return Ok(left),
        };
        self.advance();
        let right = self.parse_addsub()?;
        Ok(Expr::CompareOp {
            left: Box::new(left),
            op,
            right: Box::new(right),
        })
    }

    fn parse_addsub(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_muldiv()?;
        while let Some(op) = self.peek_bin_op() {
            match op {
                BinOp::Add | BinOp::Sub => {
                    self.advance();
                    let right = self.parse_muldiv()?;
                    left = Expr::BinaryOp {
                        left: Box::new(left),
                        op,
                        right: Box::new(right),
                    }
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_muldiv(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_unary()?;
        while let Some(op) = self.peek_bin_op() {
            match op {
                BinOp::Mul | BinOp::Div => {
                    self.advance();
                    let right = self.parse_unary()?;
                    left = Expr::BinaryOp {
                        left: Box::new(left),
                        op,
                        right: Box::new(right),
                    }
                }
                _ => break,
            }
        }
        Ok(left)
    }

    pub fn parse_unary(&mut self) -> ParseResult<Expr> {
        match self.peek().clone() {
            Token::Minus => {
                self.advance();
                let expr = self.parse_call()?;
                Ok(Expr::Unary(Box::new(expr)))
            }
            Token::Plus => {
                self.advance();
                self.parse_call()
            }
            _other => self.parse_call(),
        }
    }

    pub fn parse_call(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_atom()?;
        if *self.peek() != Token::LParen {
            Ok(left)
        } else {
            self.advance();
            let mut args = Vec::new();
            while *self.peek() != Token::RParen && *self.peek() != Token::Eof {
                let arg = self.parse_expr()?;
                args.push(arg);
                let token = self.peek();
                if *token == Token::Comma {
                    self.advance();
                } else if *token != Token::RParen && *token != Token::Eof {
                    return Err(ParseError::UnexpectedToken(token.to_string()));
                }
            }
            self.expect(Token::RParen)?;
            left = Expr::Call(Box::new(left), args);
            Ok(left)
        }
    }

    pub fn parse_atom(&mut self) -> ParseResult<Expr> {
        match self.peek().clone() {
            Token::Float(n) => {
                self.advance();
                Ok(Expr::Float(n))
            }
            Token::Integer(n) => {
                self.advance();
                Ok(Expr::Integer(n))
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            Token::Ident(name) => {
                self.advance();
                Ok(Expr::Ident(name))
            }
            Token::True => {
                self.advance();
                Ok(Expr::Bool(true))
            }
            Token::False => {
                self.advance();
                Ok(Expr::Bool(false))
            }
            other => {
                if matches!(other, Token::Eof) {
                    Err(ParseError::UnexpectedEof)
                } else {
                    Err(ParseError::UnexpectedToken(other.to_string()))
                }
            }
        }
    }
}

pub struct Compiler {
    code: Vec<OpCode>,
    fn_codes: Vec<Vec<OpCode>>,
    symbol_tables: Vec<HashMap<String, usize>>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            fn_codes: Vec::new(),
            symbol_tables: Vec::new(),
        }
    }

    fn context(&mut self) -> &mut Vec<OpCode> {
        self.fn_codes.last_mut().unwrap_or(&mut self.code)
    }
    fn push(&mut self, code: OpCode) {
        self.context().push(code);
    }

    fn enter_fn(&mut self) {
        self.fn_codes.push(Vec::new());
    }

    fn exit_fn(&mut self) -> Vec<OpCode> {
        self.fn_codes.pop().expect("退无可退")
    }

    fn enter_block(&mut self) {
        self.symbol_tables.push(HashMap::new());
        self.push(OpCode::EnterScope);
    }

    fn exit_block(&mut self) {
        let _ = self.symbol_tables.pop();
        self.push(OpCode::ExitScope);
    }

    fn add_name(&mut self, name: &String) {
        let table = self
            .symbol_tables
            .last_mut()
            .expect("未进入任何块却尝试添加局部变量");
        table.insert(name.clone(), table.len());
    }

    pub fn compile_program(stmts: &Vec<Stmt>) -> Vec<OpCode> {
        let mut compiler = Self::new();
        compiler.compile(stmts)
    }

    pub fn compile(&mut self, stmts: &Vec<Stmt>) -> Vec<OpCode> {
        for stmt in stmts {
            self.compile_stmt(stmt);
        }
        self.code.clone()
    }
    pub fn compile_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Expr(expr) => self.compile_expr(expr),
            Stmt::Let { name, value } => self.compile_let_expr(name, value),
            Stmt::Block(stmts) => self.compile_block(stmts),
            Stmt::IfElse {
                condition,
                then_branch,
                else_branch,
            } => self.compile_if_expr(condition, then_branch, else_branch),
            Stmt::While { condition, block } => self.compile_while_expr(condition, block),
            Stmt::Assign { name, value } => self.compile_assign_expr(name, value),
            Stmt::Return(expr) => {
                self.compile_expr(expr);
                self.push(OpCode::Ret);
            }
            Stmt::CompoundAssign { name, op, value } => {
                self.compile_compound_assign_expr(name, op, value)
            }
        }
    }

    fn compile_compound_assign_expr(&mut self, name: &String, op: &BinOp, value: &Expr) {
        let mut is_global = true;
        for (lev, table) in self.symbol_tables.iter().enumerate().rev() {
            if let Some(idx) = table.get(name) {
                self.push(OpCode::LoadLocal(self.symbol_tables.len() - lev, *idx));
                is_global = false;
                break;
            }
        }
        if is_global {
            self.push(OpCode::LoadGlobal(name.clone()));
        }
        self.compile_expr(value);
        self.code.push(match op {
            BinOp::Add => OpCode::Add,
            BinOp::Sub => OpCode::Sub,
            BinOp::Mul => OpCode::Mul,
            BinOp::Div => OpCode::Div,
        });
        for (lev, table) in self.symbol_tables.iter().enumerate().rev() {
            if let Some(idx) = table.get(name) {
                self.push(OpCode::StoreLocal(self.symbol_tables.len() - lev, *idx));
                return;
            }
        }
        self.push(OpCode::StoreGlobal(name.clone()))
    }

    fn compile_assign_expr(&mut self, name: &String, value: &Expr) {
        self.compile_expr(value);
        for (lev, table) in self.symbol_tables.iter().enumerate().rev() {
            if let Some(idx) = table.get(name) {
                self.push(OpCode::StoreLocal(self.symbol_tables.len() - lev, *idx));
                return;
            }
        }
        self.push(OpCode::StoreGlobal(name.clone()))
    }

    fn compile_block(&mut self, stmts: &Vec<Stmt>) {
        self.enter_block();
        for stmt in stmts {
            if let Stmt::Let { name, value: _ } = stmt {
                self.add_name(name);
            }
        }
        for stmt in stmts {
            self.compile_stmt(stmt);
        }
        self.exit_block();
    }

    fn compile_while_expr(&mut self, condition: &Expr, stmt: &Box<Stmt>) {
        let start_idx = self.context().len();
        self.compile_expr(condition);
        let jmp_idx = self.context().len();
        self.push(OpCode::JmpIfNot(usize::MAX));
        self.compile_stmt(stmt);
        self.push(OpCode::Jmp(start_idx));
        self.context()[jmp_idx] = OpCode::JmpIfNot(self.context().len());
    }

    fn compile_if_expr(
        &mut self,
        condition: &Expr,
        then_branch: &Box<Stmt>,
        else_branch: &Option<Box<Stmt>>,
    ) {
        self.compile_expr(condition);
        let jmp_idx = self.context().len();
        self.push(OpCode::JmpIfNot(usize::MAX));
        self.compile_stmt(then_branch);
        self.context()[jmp_idx] = OpCode::JmpIfNot(self.context().len());
        if let Some(stmts) = else_branch {
            let leave_jmp_idx = self.context().len();
            self.push(OpCode::Jmp(usize::MAX));
            self.compile_stmt(stmts);
            self.context()[leave_jmp_idx] = OpCode::Jmp(self.context().len());
        }
    }

    fn compile_let_expr(&mut self, name: &String, value: &Expr) {
        self.compile_assign_expr(name, value);
    }

    fn compile_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Float(n) => self.push(OpCode::Push(Value::Float(*n))),
            Expr::Integer(n) => self.push(OpCode::Push(Value::Integer(*n))),
            Expr::Bool(b) => self.push(OpCode::Push(Value::Bool(*b))),
            Expr::BinaryOp { left, op, right } => {
                self.compile_expr(left);
                self.compile_expr(right);
                let opcode = match op {
                    BinOp::Add => OpCode::Add,
                    BinOp::Sub => OpCode::Sub,
                    BinOp::Mul => OpCode::Mul,
                    BinOp::Div => OpCode::Div,
                };
                self.push(opcode);
            }
            Expr::CompareOp { left, op, right } => {
                self.compile_expr(left);
                self.compile_expr(right);
                let opcode = match op {
                    CmpOp::Eq => OpCode::Eq,
                    CmpOp::Gt => OpCode::Gt,
                    CmpOp::Lt => OpCode::Lt,
                    CmpOp::Gte => OpCode::Gte,
                    CmpOp::Lte => OpCode::Lte,
                    CmpOp::NEq => OpCode::NEq,
                };
                self.push(opcode);
            }
            Expr::Unary(expr) => {
                self.compile_expr(expr);
                self.push(OpCode::Neg);
            }
            Expr::Ident(name) => {
                for (lev, table) in self.symbol_tables.iter().enumerate().rev() {
                    // println!("{} {:?} {name}", lev, table);
                    if let Some(idx) = table.get(name) {
                        self.push(OpCode::LoadLocal(self.symbol_tables.len() - lev, *idx));
                        return;
                    }
                }
                self.push(OpCode::LoadGlobal(name.clone()))
            }
            Expr::Call(expr, args) => {
                self.compile_expr(expr);
                for arg in args {
                    self.compile_expr(arg);
                }
                self.push(OpCode::Call(args.len()));
            }
            Expr::Fn(names, block) => {
                self.enter_fn();
                self.symbol_tables.push(HashMap::new());
                for name in names {
                    self.add_name(name);
                }
                if let Stmt::Block(stmts) = &**block {
                    for stmt in stmts {
                        if let Stmt::Let { name, value: _ } = stmt {
                            self.add_name(name);
                        }
                    }
                    println!("{:?}", self.symbol_tables);
                    for stmt in stmts {
                        self.compile_stmt(stmt);
                    }
                } else {
                    panic!("函数只能带块")
                }
                self.exit_block();
                let opcodes = self.exit_fn();
                self.push(OpCode::PushFn(names.len(), opcodes));
            }
        }
    }
}

#[cfg(test)]
mod parser_test {
    use super::*;
    use crate::lexer::Lexer;
    #[test]
    fn parser_test() -> Result<(), Box<dyn std::error::Error>> {
        let mut lexer = Lexer::new(r#"1 + 2;"#.to_owned());
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        let expr = parser.parse()?;
        println!("{:#?}", expr);
        Ok(())
    }
    #[test]
    fn compiler_test() -> Result<(), Box<dyn std::error::Error>> {
        let mut lexer = Lexer::new("if (11 == 22) { }".to_owned());
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        let stmts = parser.parse()?;
        let opcodes = Compiler::compile_program(&stmts);
        println!("{:#?}", opcodes);
        Ok(())
    }
}
