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
    Gt,
    Lt,
    Gte,
    Lte,
    LParen,
    RParen,
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

    fn next_token(&mut self) -> Token {
        while self.position < self.source.len() && self.source[self.position].is_whitespace() {
            self.position += 1;
        }
        if self.position >= self.source.len() {
            return Token::Eof;
        }

        let ch = self.source[self.position];
        self.position += 1;
        match ch {
            '+' => Token::Plus,
            '-' => Token::Minus,
            '*' => Token::Star,
            '/' => Token::Slash,
            '(' => Token::LParen,
            ')' => Token::RParen,
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            ';' => Token::Semicolon,
            '=' => {
                if self.position < self.source.len() && self.source[self.position] == '=' {
                    self.position += 1;
                    Token::Eq
                } else {
                    Token::Assign
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
                            panic!("Multiple dots in number")
                        }
                        is_float = true;
                        num_str.push(next_ch);
                        self.position += 1;
                    } else {
                        break;
                    }
                }
                if is_float {
                    Token::Float(num_str.parse().unwrap())
                } else {
                    Token::Integer(num_str.parse().unwrap())
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
                    "let" => Token::Let,
                    "true" => Token::True,
                    "false" => Token::False,
                    "if" => Token::If,
                    "else" => Token::Else,
                    _ => Token::Ident(ident_name),
                }
            }
            _ => panic!("Unexpected char: {}", ch),
        }
    }
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            tokens.push(token.clone());
            if token == Token::Eof {
                break;
            }
        }
        tokens
    }
}

#[cfg(test)]
mod lexer_test {
    use super::*;

    #[test]
    fn lexer_test() {
        let mut lexer = Lexer::new("if a == 2.0 {}".to_owned());
        let tokens = lexer.tokenize();
        println!("{:?}", tokens);
    }
}
