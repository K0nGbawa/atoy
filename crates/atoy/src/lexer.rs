use std::{write};

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

    #[error("unexpected End of File")]
    UnexpectedEof,

    #[error("unexpected End of Line at line {line}", line = position.line)]
    UnexpectedEol { position: Position },

    #[error("illegal escaping sequence '\\{ch}' at line {line} col {col}", line = position.line, col = position.col)]
    IllegalEscapingSequence { ch: char, position: Position },
}

type LexResult<T> = Result<T, LexError>;

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Integer(i64),
    Float(f64),
    String(String),
    Plus,
    Minus,
    Star,
    Slash,
    Assign,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,
    Eq,
    NEq,
    Gt,
    Lt,
    Gte,
    Lte,
    And,
    Or,
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
    Fn,
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
            Self::PlusAssign => write!(f, "+="),
            Self::MinusAssign => write!(f, "-="),
            Self::StarAssign => write!(f, "*="),
            Self::SlashAssign => write!(f, "/="),
            Self::Eq => write!(f, "=="),
            Self::NEq => write!(f, "!="),
            Self::Gt => write!(f, ">"),
            Self::Lt => write!(f, "<"),
            Self::Gte => write!(f, ">="),
            Self::Lte => write!(f, "<="),
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
            Self::Fn => write!(f, "fn"),
            Self::And => write!(f, "and"),
            Self::Or => write!(f, "or"),
            Self::Not => write!(f, "not"),
            Self::String(string) => write!(f, "{}", string.escape_debug()),
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

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            while self.position < self.source.len() && self.source[self.position].is_whitespace() {
                self.position += 1;
            }

            if self.position + 1 < self.source.len() {
                let ch = self.source[self.position];
                let next_ch = self.source[self.position + 1];
                if ch == '/' && next_ch == '/' {
                    while self.position < self.source.len()
                        && !matches!(self.source[self.position], '\n')
                    {
                        self.position += 1;
                    }
                    continue;
                }
            }

            break;
        }
    }

    fn next_token(&mut self) -> LexResult<Token> {
        self.skip_whitespace_and_comments();
        if self.position >= self.source.len() {
            return Ok(Token::Eof);
        }

        let ch = self.source[self.position];
        self.position += 1;
        match ch {
            '"' | '\'' => {
                let starting = ch;
                let mut escaped = false;
                let mut string = String::new();
                while self.position < self.source.len() {
                    let next_ch = self.source[self.position];
                    if next_ch == starting {
                        self.position += 1;
                        return Ok(Token::String(string));
                    } else if next_ch == '\\' && !escaped {
                        escaped = true;
                        self.position += 1;
                    } else if escaped {
                        escaped = false;
                        if next_ch == '\n' {
                            self.position += 1;
                            continue;
                        }
                        if next_ch == 'u' {
                            self.position += 1;
                            let pos = self.position;
                            let hex_str: [char; 4] = self.source[pos..pos + 4].try_into().unwrap();
                            let mut result = 0;
                            for c in hex_str {
                                let digit = c.to_digit(16).unwrap();
                                result = (result << 4) | digit;
                            }
                            let character = char::from_u32(result).unwrap();
                            self.position += 4;
                            string.push(character);
                            continue;
                        }
                        let escaped = match next_ch {
                            '"' => '"',
                            '\'' => '\'',
                            '\\' => '\\',
                            'n' => '\n',
                            't' => '\t',
                            'r' => '\r',
                            _ => {
                                return Err(LexError::IllegalEscapingSequence {
                                    ch: next_ch,
                                    position: self.get_position(),
                                });
                            }
                        };
                        self.position += 1;
                        string.push(escaped);
                    } else if next_ch == '\n' {
                        return Err(LexError::UnexpectedEol {
                            position: self.get_position(),
                        });
                    } else {
                        string.push(next_ch);
                        self.position += 1;
                    }
                }
                Err(LexError::UnexpectedEof)
            }
            '+' => {
                if self.position < self.source.len() && self.source[self.position] == '=' {
                    self.position += 1;
                    Ok(Token::PlusAssign)
                } else {
                    Ok(Token::Plus)
                }
            }
            '-' => {
                if self.position < self.source.len() && self.source[self.position] == '=' {
                    self.position += 1;
                    Ok(Token::MinusAssign)
                } else {
                    Ok(Token::Minus)
                }
            }
            '*' => {
                if self.position < self.source.len() && self.source[self.position] == '=' {
                    self.position += 1;
                    Ok(Token::StarAssign)
                } else {
                    Ok(Token::Star)
                }
            }
            '/' => {
                if self.position < self.source.len() && self.source[self.position] == '=' {
                    self.position += 1;
                    Ok(Token::SlashAssign)
                } else {
                    Ok(Token::Slash)
                }
            }
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
                    "fn" => Ok(Token::Fn),
                    "and" => Ok(Token::And),
                    "or" => Ok(Token::Or),
                    "not" => Ok(Token::Not),
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
