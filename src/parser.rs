use std::collections::HashMap;

use crate::lexer::Token;

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
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Gt,
    Lt,
    Gte,
    Lte,
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
            _ => None,
        }
    }

    pub fn parse(&mut self) -> Vec<Stmt> {
        let mut stmts = Vec::new();
        while *self.peek() != Token::Eof {
            let stmt = self.parse_stmt();
            stmts.push(stmt)
        }
        stmts
    }

    fn parse_stmt(&mut self) -> Stmt {
        match self.peek().clone() {
            Token::Let => self.parse_let_stmt(),
            Token::If => self.parse_if_stmt(),
            _ => {
                let expr = self.parse_cmp_op();
                if *self.peek() == Token::Semicolon {
                    self.advance();
                } else {
                    panic!("Expected ';' after expression")
                }
                Stmt::Expr(expr)
            }
        }
    }

    fn parse_if_stmt(&mut self) -> Stmt {
        self.advance();
        let condition = self.parse_cmp_op();
        let then_branch = self.parse_block();
        let else_branch = match self.peek() {
            Token::Else => {
                self.advance();
                Some(self.parse_block())
            }
            _ => None,
        };
        Stmt::IfElse {
            condition,
            then_branch,
            else_branch,
        }
    }

    fn parse_block(&mut self) -> Vec<Stmt> {
        match self.peek() {
            Token::LBrace => self.advance(),
            _ => panic!("Expected '{{' before block"),
        };

        let mut stmts = Vec::new();
        while *self.peek() != Token::RBrace && *self.peek() != Token::Eof {
            let stmt = self.parse_stmt();
            stmts.push(stmt)
        }
        match self.peek() {
            Token::RBrace => {
                self.advance();
                stmts
            }
            _ => panic!("Expected '}}' after block"),
        }
    }

    fn parse_let_stmt(&mut self) -> Stmt {
        self.advance();
        let name = match self.advance() {
            Token::Ident(name) => name,
            _ => panic!("Expected identifier after 'let'"),
        };
        match self.advance() {
            Token::Assign => {}
            _ => panic!("Expected '=' after identifier"),
        };
        let value = self.parse_addsub();
        match self.advance() {
            Token::Semicolon => {}
            _ => panic!("Expected ';' after let statement"),
        };
        Stmt::Let { name, value }
    }

    fn parse_cmp_op(&mut self) -> Expr {
        let left = self.parse_addsub();
        let op = match self.peek_cmp_op() {
            Some(op) => op,
            _ => return left,
        };
        self.advance();
        let right = self.parse_addsub();
        Expr::CompareOp {
            left: Box::new(left),
            op,
            right: Box::new(right),
        }
    }

    fn parse_addsub(&mut self) -> Expr {
        let mut left = self.parse_muldiv();
        while let Some(op) = self.peek_bin_op() {
            match op {
                BinOp::Add | BinOp::Sub => {
                    self.advance();
                    let right = self.parse_muldiv();
                    left = Expr::BinaryOp {
                        left: Box::new(left),
                        op,
                        right: Box::new(right),
                    }
                }
                _ => break,
            }
        }
        left
    }

    fn parse_muldiv(&mut self) -> Expr {
        let mut left = self.parse_factor();
        while let Some(op) = self.peek_bin_op() {
            match op {
                BinOp::Mul | BinOp::Div => {
                    self.advance();
                    let right = self.parse_factor();
                    left = Expr::BinaryOp {
                        left: Box::new(left),
                        op,
                        right: Box::new(right),
                    }
                }
                _ => break,
            }
        }
        left
    }

    pub fn parse_factor(&mut self) -> Expr {
        match self.peek().clone() {
            Token::Float(n) => {
                self.advance();
                Expr::Float(n)
            }
            Token::Integer(n) => {
                self.advance();
                Expr::Integer(n)
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_addsub();
                match self.peek() {
                    Token::RParen => {
                        self.advance();
                        expr
                    }
                    _ => panic!("Expected ')'"),
                }
            }
            Token::Minus => {
                self.advance();
                let expr = self.parse_factor();
                Expr::Unary(Box::new(expr))
            }
            Token::Plus => {
                self.advance();
                self.parse_factor()
            }
            Token::Ident(name) => {
                self.advance();
                Expr::Ident(name)
            }
            Token::True => {
                self.advance();
                Expr::Bool(true)
            }
            Token::False => {
                self.advance();
                Expr::Bool(false)
            }
            _ => panic!("Unexpected Token: {:?}", self.peek()),
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
    pub fn compile(stmts: &Vec<Stmt>) -> Vec<OpCode> {
        let mut compiler = Self::new();
        for stmt in stmts {
            Self::compile_stmt(stmt, &mut compiler.code, &mut compiler.symbol_table);
        }
        compiler.code
    }

    pub fn compile_stmt(
        stmt: &Stmt,
        code: &mut Vec<OpCode>,
        symbol_table: &mut HashMap<String, usize>,
    ) {
        match stmt {
            Stmt::Expr(expr) => Self::compile_expr(expr, code, symbol_table),
            Stmt::Let { name, value } => Self::compile_let_expr(name, value, code, symbol_table),
            Stmt::IfElse {
                condition,
                then_branch,
                else_branch,
            } => Self::compile_if_expr(condition, then_branch, else_branch, code, symbol_table),
        }
    }

    fn compile_if_expr(
        condition: &Expr,
        then_branch: &Vec<Stmt>,
        else_branch: &Option<Vec<Stmt>>,
        code: &mut Vec<OpCode>,
        symbol_table: &mut HashMap<String, usize>,
    ) {
        Self::compile_expr(condition, code, symbol_table);
        let jmp_idx = code.len();
        code.push(OpCode::JmpIfNot(usize::MAX));
        for stmt in then_branch {
            Self::compile_stmt(stmt, code, symbol_table);
        }
        let leave_jmp_idx = code.len();
        code.push(OpCode::Jmp(usize::MAX));
        code[jmp_idx] = OpCode::JmpIfNot(code.len());
        if let Some(stmts) = else_branch {
            for stmt in stmts {
                Self::compile_stmt(stmt, code, symbol_table);
            }
        }
        code[leave_jmp_idx] = OpCode::Jmp(code.len());
    }

    fn compile_let_expr(
        name: &String,
        value: &Expr,
        code: &mut Vec<OpCode>,
        symbol_table: &mut HashMap<String, usize>,
    ) {
        Self::compile_expr(value, code, symbol_table);
        let index = if let Some(&idx) = symbol_table.get(name) {
            idx
        } else {
            let idx = symbol_table.len();
            symbol_table.insert(name.clone(), idx);
            idx
        };
        code.push(OpCode::StoreGlobal(index));
    }

    fn compile_expr(expr: &Expr, code: &mut Vec<OpCode>, symbol_table: &HashMap<String, usize>) {
        match expr {
            Expr::Float(n) => code.push(OpCode::Push(Value::Float(*n))),
            Expr::Integer(n) => code.push(OpCode::Push(Value::Integer(*n))),
            Expr::Bool(b) => code.push(OpCode::Push(Value::Bool(*b))),
            Expr::BinaryOp { left, op, right } => {
                Self::compile_expr(left, code, symbol_table);
                Self::compile_expr(right, code, symbol_table);
                let opcode = match op {
                    BinOp::Add => OpCode::Add,
                    BinOp::Sub => OpCode::Sub,
                    BinOp::Mul => OpCode::Mul,
                    BinOp::Div => OpCode::Div,
                };
                code.push(opcode);
            }
            Expr::CompareOp { left, op, right } => {
                Self::compile_expr(left, code, symbol_table);
                Self::compile_expr(right, code, symbol_table);
                let opcode = match op {
                    CmpOp::Eq => OpCode::Eq,
                    CmpOp::Gt => OpCode::Gt,
                    CmpOp::Lt => OpCode::Lt,
                    CmpOp::Gte => OpCode::Gte,
                    CmpOp::Lte => OpCode::Lte,
                };
                code.push(opcode);
            }
            Expr::Unary(expr) => {
                Self::compile_expr(expr, code, symbol_table);
                code.push(OpCode::Neg);
            }
            Expr::Ident(name) => match symbol_table.get(name) {
                Some(idx) => code.push(OpCode::LoadGlobal(*idx)),
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
    fn parser_test() {
        let mut lexer = Lexer::new(r#"1 + 2;"#.to_owned());
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let expr = parser.parse();
        println!("{:#?}", expr)
    }
    #[test]
    fn compiler_test() {
        let mut lexer = Lexer::new("let a = 1; if a == 2 { 1; } else { 2; }".to_owned());
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let stmts = parser.parse();
        let opcodes = Compiler::compile(&stmts);
        println!("{:#?}", opcodes)
    }
}
