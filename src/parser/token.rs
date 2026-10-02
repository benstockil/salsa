use crate::color::Color;

#[derive(Debug, Clone, Copy, Default)]
pub struct Span {
    line: u32,
    col: u32,
    length: u32,
}

impl Span {
    pub fn new(line: u32, col: u32, length: u32) -> Self {
        Self { line, col, length }
    }

    /// Zero-based line index.
    pub fn line(&self) -> u32 {
        self.line
    }

    /// Zero-based column index.
    pub fn col(&self) -> u32 {
        self.col
    }

    /// Length of the span, in characters.
    pub fn length(&self) -> u32 {
        self.length
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub span: Span,
    pub ty: TokenType,
}
#[derive(Debug, Clone)]
pub enum TokenType {
    // Meta
    Meta,
    Title,
    Bpm,
    Barlength,
    Stagesize,

    // Dancers
    Dancers,
    Ident(String),
    StringLiteral(String),
    Color(Color),
    Is,
    In,

    // Start
    Start,
    Along,

    // Let
    Let,

    // At
    At,
    All,
    Bars,
    Beat(u64, u64),
    Beats,
    By,
    Every,
    Except,
    Move,
    Rotate,
    To,
    Until,

    // Punctuation
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Semicolon,
    Equals,
    Slash,

    // Value
    UInt(u64),
    Float(f64),
}

impl TokenType {
    pub fn at(self, line: u32, col: u32, length: u32) -> Token {
        Token {
            span: Span { line, col, length },
            ty: self,
        }
    }
}
