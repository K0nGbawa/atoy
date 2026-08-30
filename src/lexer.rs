use std::fmt::write;

use thiserror::Error;

#[derive(Debug)]
pub struct Position {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Error)]
pub enum LexError {
    #[error("multiple dots in number at line {line} col {col}", line = position.line, col = position.col)]
    MultipleDots { position: Position },

    #[error("unexpected character '{ch}' at line {line} col {col}", line = position.line, col = position.col)]
    UnexpectedChar { ch: char, position: Position },
}

type LexResult<T> = Result<T, LexError>;

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Integer(i64),
    Float(f64),
    Plus,
    Minus,
    Star,
    Slash,
    Assign,
    Eq,
    NEq,
    Gt,
    Lt,
    Gte,
    Lte,
    Not,
    LParen,
    RParen,
    Comma,
    LBrace,
    RBrace,
    Semicolon,
    Ident(String),
    Eof,

    // keywords
    Let,
    True,
    False,
    If,
    Else,
    Return,
    While,
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Integer(n) => write!(f, "{n}"),
            Self::Float(n) => write!(f, "{n}"),
            Self::Plus => write!(f, "+"),
            Self::Minus => write!(f, "-"),
            Self::Star => write!(f, "*"),
            Self::Slash => write!(f, "/"),
            Self::Assign => write!(f, "="),
            Self::Eq => write!(f, "=="),
            Self::NEq => write!(f, "!="),
            Self::Gt => write!(f, ">"),
            Self::Lt => write!(f, "<"),
            Self::Gte => write!(f, ">="),
            Self::Lte => write!(f, "<="),
            Self::Not => write!(f, "!"),
            Self::LParen => write!(f, "("),
            Self::RParen => write!(f, ")"),
            Self::Comma => write!(f, ","),
            Self::LBrace => write!(f, "{{"),
            Self::RBrace => write!(f, "}}"),
            Self::Semicolon => write!(f, ";"),
            Self::Ident(name) => write!(f, "{name}"),
            Self::Eof => write!(f, "EOF"),
            Self::Let => write!(f, "let"),
            Self::True => write!(f, "true"),
            Self::False => write!(f, "false"),
            Self::If => write!(f, "if"),
            Self::Else => write!(f, "else"),
            Self::Return => write!(f, "return"),
            Self::While => write!(f, "while"),
        }
    }
}

pub struct Lexer {
    pub source: Vec<char>,
    pub position: usize,
}

impl Lexer {
    pub fn new(source: String) -> Self {
        Self {
            source: source.chars().collect(),
            position: 0,
        }
    }

    fn get_position(&self) -> Position {
        let mut line = 0;
        let mut col = 0;
        for &ch in &self.source[..self.position] {
            if ch == '\n' {
                col = 0;
                line += 1;
            } else if ch != '\r' {
                col += 1;
            }
        }
        Position { line, col }
    }

    fn next_token(&mut self) -> LexResult<Token> {
        while self.position < self.source.len() && self.source[self.position].is_whitespace() {
            self.position += 1;
        }
        if self.position >= self.source.len() {
            return Ok(Token::Eof);
        }

        let ch = self.source[self.position];
        self.position += 1;
        match ch {
            '+' => Ok(Token::Plus),
            '-' => Ok(Token::Minus),
            '*' => Ok(Token::Star),
            '/' => Ok(Token::Slash),
            '(' => Ok(Token::LParen),
            ')' => Ok(Token::RParen),
            ',' => Ok(Token::Comma),
            '{' => Ok(Token::LBrace),
            '}' => Ok(Token::RBrace),
            ';' => Ok(Token::Semicolon),
            '=' => {
                if self.position < self.source.len() && self.source[self.position] == '=' {
                    self.position += 1;
                    Ok(Token::Eq)
                } else {
                    Ok(Token::Assign)
                }
            }
            '>' => {
                if self.position < self.source.len() && self.source[self.position] == '=' {
                    self.position += 1;
                    Ok(Token::Gte)
                } else {
                    Ok(Token::Gt)
                }
            }
            '<' => {
                if self.position < self.source.len() && self.source[self.position] == '=' {
                    self.position += 1;
                    Ok(Token::Lte)
                } else {
                    Ok(Token::Lt)
                }
            }
            '!' => {
                if self.position < self.source.len() && self.source[self.position] == '=' {
                    self.position += 1;
                    Ok(Token::NEq)
                } else {
                    Ok(Token::Not)
                }
            }
            '0'..='9' => {
                let mut num_str = String::new();
                let mut is_float = false;
                num_str.push(ch);
                while self.position < self.source.len() {
                    let next_ch = self.source[self.position];
                    if next_ch.is_ascii_digit() {
                        num_str.push(next_ch);
                        self.position += 1;
                    } else if next_ch == '.' {
                        if is_float {
                            return Err(LexError::MultipleDots {
                                position: self.get_position(),
                            });
                        }
                        is_float = true;
                        num_str.push(next_ch);
                        self.position += 1;
                    } else {
                        break;
                    }
                }
                if is_float {
                    Ok(Token::Float(num_str.parse().unwrap()))
                } else {
                    Ok(Token::Integer(num_str.parse().unwrap()))
                }
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                let mut ident_name = String::new();
                ident_name.push(ch);
                while self.position < self.source.len() {
                    let next_ch = self.source[self.position];
                    match next_ch {
                        'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => {
                            ident_name.push(next_ch);
                            self.position += 1
                        }
                        _ => break,
                    }
                }
                match ident_name.as_str() {
                    "let" => Ok(Token::Let),
                    "true" => Ok(Token::True),
                    "false" => Ok(Token::False),
                    "if" => Ok(Token::If),
                    "else" => Ok(Token::Else),
                    "return" => Ok(Token::Return),
                    "while" => Ok(Token::While),
                    _ => Ok(Token::Ident(ident_name)),
                }
            }
            _ => {
                return Err(LexError::UnexpectedChar {
                    ch,
                    position: self.get_position(),
                });
            }
        }
    }
    pub fn tokenize(&mut self) -> LexResult<Vec<Token>> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token()?;
            tokens.push(token.clone());
            if token == Token::Eof {
                break;
            }
        }
        Ok(tokens)
    }
}

#[cfg(test)]
mod lexer_test {
    use super::*;

    #[test]
    fn lexer_test() -> LexResult<()> {
        let mut lexer = Lexer::new("if a == 2.0 {}\n".to_owned());
        let tokens = lexer.tokenize()?;
        println!("{:?}", tokens);
        Ok(())
    }
}
