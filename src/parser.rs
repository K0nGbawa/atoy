use std::collections::HashMap;

use thiserror::Error;

use crate::lexer::Token;

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
        then_branch: Vec<Stmt>,
        else_branch: Option<Vec<Stmt>>,
    },
    While {
        condition: Expr,
        stmts: Vec<Stmt>,
    },
    Assign {
        name: String,
        value: Expr,
    },
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
}

#[derive(Debug, PartialEq, Clone)]
pub enum OpCode {
    Push(Value),
    StoreGlobal(usize),
    LoadGlobal(usize),
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
}

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Float(f64),
    Integer(i64),
    Bool(bool),
    None,
}

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.position].clone();
        self.position += 1;
        token
    }

    fn expect_token(&mut self, expected: Token) -> ParseResult<Token> {
        let tok = self.advance();
        if std::mem::discriminant(&tok) == std::mem::discriminant(&expected) {
            Ok(tok)
        } else {
            Err(ParseError::ExpectedToken(
                expected.to_string(),
                tok.to_string(),
            ))
        }
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
            _ => {
                let left: Expr = self.parse_cmp_op()?;
                let res = match &left {
                    Expr::Ident(name) => match self.peek() {
                        Token::Assign => self.parse_assign_stmt(name.clone())?,
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

        let value = self.parse_cmp_op()?;

        Ok(Stmt::Assign { name, value })
    }

    fn parse_if_stmt(&mut self) -> ParseResult<Stmt> {
        self.advance();
        let condition = self.parse_cmp_op()?;
        let then_branch = self.parse_block()?;
        let else_branch = match self.peek() {
            Token::Else => {
                self.advance();
                Some(self.parse_block()?)
            }
            _ => None,
        };
        Ok(Stmt::IfElse {
            condition,
            then_branch,
            else_branch,
        })
    }

    fn parse_while_stmt(&mut self) -> ParseResult<Stmt> {
        self.advance();
        let condition = self.parse_cmp_op()?;
        let stmts = self.parse_block()?;
        Ok(Stmt::While { condition, stmts })
    }

    fn parse_block(&mut self) -> ParseResult<Vec<Stmt>> {
        self.expect(Token::LBrace)?;

        let mut stmts = Vec::new();
        while *self.peek() != Token::RBrace && *self.peek() != Token::Eof {
            let stmt = self.parse_stmt()?;
            stmts.push(stmt)
        }
        self.expect(Token::RBrace)?;
        Ok(stmts)
    }

    fn parse_let_stmt(&mut self) -> ParseResult<Stmt> {
        self.advance();
        let name = match self.advance() {
            Token::Ident(name) => name,
            other => return Err(ParseError::ExpectedIdentifier(other.to_string())),
        };
        self.expect(Token::Assign)?;
        let value = self.parse_addsub()?;
        self.expect(Token::Semicolon)?;
        Ok(Stmt::Let { name, value })
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
        let mut left = self.parse_factor()?;
        while let Some(op) = self.peek_bin_op() {
            match op {
                BinOp::Mul | BinOp::Div => {
                    self.advance();
                    let right = self.parse_factor()?;
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

    pub fn parse_factor(&mut self) -> ParseResult<Expr> {
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
                let expr = self.parse_cmp_op()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            Token::Minus => {
                self.advance();
                let expr = self.parse_factor()?;
                Ok(Expr::Unary(Box::new(expr)))
            }
            Token::Plus => {
                self.advance();
                self.parse_factor()
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
    symbol_table: HashMap<String, usize>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            symbol_table: std::collections::HashMap::new(),
        }
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
            Stmt::IfElse {
                condition,
                then_branch,
                else_branch,
            } => self.compile_if_expr(condition, then_branch, else_branch),
            Stmt::While { condition, stmts } => self.compile_while_expr(condition, stmts),
            Stmt::Assign { name, value } => self.compile_assign_expr(name, value),
        }
    }

    fn compile_assign_expr(&mut self, name: &String, value: &Expr) {
        self.compile_expr(value);
        let index = if let Some(&idx) = self.symbol_table.get(name) {
            idx
        } else {
            panic!("变量 {name} 未定义");
        };
        self.code.push(OpCode::StoreGlobal(index));
    }

    fn compile_while_expr(&mut self, condition: &Expr, stmts: &Vec<Stmt>) {
        let start_idx = self.code.len();
        self.compile_expr(condition);
        let jmp_idx = self.code.len();
        self.code.push(OpCode::JmpIfNot(usize::MAX));
        for stmt in stmts {
            self.compile_stmt(stmt);
        }
        self.code.push(OpCode::Jmp(start_idx));
        self.code[jmp_idx] = OpCode::JmpIfNot(self.code.len());
    }

    fn compile_if_expr(
        &mut self,
        condition: &Expr,
        then_branch: &Vec<Stmt>,
        else_branch: &Option<Vec<Stmt>>,
    ) {
        self.compile_expr(condition);
        let jmp_idx = self.code.len();
        self.code.push(OpCode::JmpIfNot(usize::MAX));
        for stmt in then_branch {
            self.compile_stmt(stmt);
        }
        let leave_jmp_idx = self.code.len();
        self.code.push(OpCode::Jmp(usize::MAX));
        self.code[jmp_idx] = OpCode::JmpIfNot(self.code.len());
        if let Some(stmts) = else_branch {
            for stmt in stmts {
                self.compile_stmt(stmt);
            }
        }
        self.code[leave_jmp_idx] = OpCode::Jmp(self.code.len());
    }

    fn compile_let_expr(&mut self, name: &String, value: &Expr) {
        self.compile_expr(value);
        let index = if let Some(&idx) = self.symbol_table.get(name) {
            idx
        } else {
            let idx = self.symbol_table.len();
            self.symbol_table.insert(name.clone(), idx);
            idx
        };
        self.code.push(OpCode::StoreGlobal(index));
    }

    fn compile_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Float(n) => self.code.push(OpCode::Push(Value::Float(*n))),
            Expr::Integer(n) => self.code.push(OpCode::Push(Value::Integer(*n))),
            Expr::Bool(b) => self.code.push(OpCode::Push(Value::Bool(*b))),
            Expr::BinaryOp { left, op, right } => {
                self.compile_expr(left);
                self.compile_expr(right);
                let opcode = match op {
                    BinOp::Add => OpCode::Add,
                    BinOp::Sub => OpCode::Sub,
                    BinOp::Mul => OpCode::Mul,
                    BinOp::Div => OpCode::Div,
                };
                self.code.push(opcode);
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
                self.code.push(opcode);
            }
            Expr::Unary(expr) => {
                self.compile_expr(expr);
                self.code.push(OpCode::Neg);
            }
            Expr::Ident(name) => match self.symbol_table.get(name) {
                Some(idx) => self.code.push(OpCode::LoadGlobal(*idx)),
                None => panic!("变量 {} 未定义", name),
            },
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
