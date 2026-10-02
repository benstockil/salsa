use std::{fs, io, path::PathBuf};

use crate::{
    color::Color,
    parser::{ParseError, ParseErrorKind, Span, Token, token::TokenType},
};

pub struct Lexer {
    source: Vec<char>,
    current_char: usize,
    line: u32,
    col: u32,
}

impl Lexer {
    pub fn from_file(path: impl Into<PathBuf>) -> io::Result<Self> {
        let source = fs::read_to_string(path.into())?;
        Ok(Self::from_source(&source))
    }

    pub fn from_source(source: &str) -> Self {
        Self {
            source: source.chars().collect(),
            current_char: 0,
            line: 0,
            col: 0,
        }
    }

    pub fn current(&self) -> Option<char> {
        self.source.get(self.current_char).copied()
    }

    /// Build a spanned lexing error at the given position.
    fn error(&self, line: u32, col: u32, length: u32, kind: LexingError) -> ParseError {
        ParseError {
            span: Span::new(line, col, length),
            kind: ParseErrorKind::Lex(kind),
        }
    }

    /// Consume the current character, advancing the cursor and keeping the
    /// line/column position in sync. Returns the consumed character, if any.
    fn advance(&mut self) -> Option<char> {
        let c = self.current()?;
        self.current_char += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 0;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn parse_numeric(&mut self, negative: bool) -> Result<Token, ParseError> {
        let start_col = if negative {
            self.col.saturating_sub(1)
        } else {
            self.col
        };

        let digits = self.parse_digits()?;

        // lex floats
        let token = if let Some('.') = self.current() {
            self.advance();
            let start = self.current_char;
            let second = self.parse_digits()?;
            let len = self.current_char - start;

            let value = digits as f64 + second as f64 / f64::powi(10.0, len as i32);
            let value = if negative { -value } else { value };

            TokenType::Float(value).at(self.line, start_col, self.col.saturating_sub(start_col))
        }
        // lex time signatures and beat identifiers
        else if let Some(':') = self.current()
            && !negative
        {
            self.advance();
            let second = self.parse_digits()?;

            TokenType::Beat(digits, second).at(self.line, start_col, self.col.saturating_sub(start_col))
        }
        // lex negative integer as float
        else if negative {
            TokenType::Float(-(digits as f64)).at(
                self.line,
                start_col,
                self.col.saturating_sub(start_col),
            )
        }
        // lex positive integer as int
        else {
            TokenType::UInt(digits).at(self.line, start_col, self.col.saturating_sub(start_col))
        };

        Ok(token)
    }

    fn parse_digits(&mut self) -> Result<u64, ParseError> {
        let (line, col) = (self.line, self.col);
        let start = self.current_char;
        while self.current().is_some_and(|c| c.is_ascii_digit()) {
            self.advance();
        }

        let digits = String::from_iter(&self.source[start..self.current_char]);

        digits.parse().map_err(|e| {
            self.error(
                line,
                col,
                self.col.saturating_sub(col),
                LexingError::InvalidNumber(e),
            )
        })
    }

    fn parse_ident_or_keyword(&mut self) -> Token {
        let (l, c) = (self.line, self.col);

        let start = self.current_char;
        while self.current().is_some_and(|c| c.is_alphabetic()) {
            self.advance();
        }

        let ident = String::from_iter(&self.source[start..self.current_char]);

        match ident.as_str() {
            "all" => TokenType::All.at(l, c, 3),
            "along" => TokenType::Along.at(l, c, 5),
            "at" => TokenType::At.at(l, c, 2),
            "bars" => TokenType::Bars.at(l, c, 4),
            "beats" => TokenType::Beats.at(l, c, 5),
            "bpm" => TokenType::Bpm.at(l, c, 3),
            "by" => TokenType::By.at(l, c, 2),
            "dancers" => TokenType::Dancers.at(l, c, 7),
            "except" => TokenType::Except.at(l, c, 6),
            "every" => TokenType::Every.at(l, c, 5),
            "in" => TokenType::In.at(l, c, 2),
            "is" => TokenType::Is.at(l, c, 2),
            "let" => TokenType::Let.at(l, c, 3),
            "meta" => TokenType::Meta.at(l, c, 4),
            "move" => TokenType::Move.at(l, c, 4),
            "rotate" => TokenType::Rotate.at(l, c, 6),
            "stagesize" => TokenType::Stagesize.at(l, c, 9),
            "start" => TokenType::Start.at(l, c, 5),
            "title" => TokenType::Title.at(l, c, 5),
            "barlength" => TokenType::Barlength.at(l, c, 9),
            "to" => TokenType::To.at(l, c, 2),
            "until" => TokenType::Until.at(l, c, 5),
            _ => match Color::from_ident(&ident) {
                Some(color) => TokenType::Color(color),
                None => TokenType::Ident(ident.clone()),
            }
            .at(l, c, ident.len() as u32),
        }
    }
}

impl Iterator for Lexer {
    type Item = Result<Token, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut ready = false;

        while !ready {
            ready = true;

            while self.current()?.is_whitespace() {
                ready = false;
                self.advance();
            }

            while self.current()? == '/' && self.source.get(self.current_char + 1) == Some(&'/') {
                ready = false;
                while self.current()? != '\n' {
                    self.advance();
                }
                self.advance();
            }
        }

        let mut advance_one = true;

        let (l, c) = (self.line, self.col);

        use TokenType as T;
        let ch = self.current()?;
        let token = match ch {
            // one-char tokens
            '(' => T::LParen.at(l, c, 1),
            ')' => T::RParen.at(l, c, 1),
            '{' => T::LBrace.at(l, c, 1),
            '}' => T::RBrace.at(l, c, 1),
            '[' => T::LBracket.at(l, c, 1),
            ']' => T::RBracket.at(l, c, 1),
            ';' => T::Semicolon.at(l, c, 1),
            ':' => T::Colon.at(l, c, 1),
            ',' => T::Comma.at(l, c, 1),
            '=' => T::Equals.at(l, c, 1),
            '/' => T::Slash.at(l, c, 1),

            // negative numbers
            '-' => {
                self.advance();
                advance_one = false;
                match self.parse_numeric(true) {
                    Ok(token) => token,
                    Err(e) => return Some(Err(e)),
                }
            }

            '"' => {
                self.advance(); // opening quote
                let start = self.current_char;

                loop {
                    match self.current() {
                        Some('"') => break,
                        Some(_) => {
                            self.advance();
                        }
                        None => {
                            return Some(Err(self.error(
                                l,
                                c,
                                self.col.saturating_sub(c),
                                LexingError::UnterminatedString,
                            )));
                        }
                    }
                }

                let value = String::from_iter(&self.source[start..self.current_char]);

                self.advance(); // closing quote
                advance_one = false;

                TokenType::StringLiteral(value).at(l, c, self.col.saturating_sub(c))
            }

            // numeric tokens
            x if x.is_ascii_digit() => {
                advance_one = false;
                match self.parse_numeric(false) {
                    Ok(token) => token,
                    Err(e) => return Some(Err(e)),
                }
            }

            // identifiers and keywords
            x if x.is_ascii_alphabetic() => {
                advance_one = false;
                self.parse_ident_or_keyword()
            }

            // invalid char
            x => {
                return Some(Err(self.error(l, c, 1, LexingError::InvalidToken(x))));
            }
        };

        if advance_one {
            self.advance();
        }

        Some(Ok(token))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LexingError {
    #[error("Cannot lex input token {0}")]
    InvalidToken(char),
    #[error("Unterminated string literal")]
    UnterminatedString,
    #[error("Invalid number: {0}")]
    InvalidNumber(std::num::ParseIntError),
}
