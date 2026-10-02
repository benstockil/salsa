mod diagnostic;
mod lexer;
mod token;

pub use diagnostic::render_diagnostic;
pub use lexer::{Lexer, LexingError};
pub use token::{Span, Token, TokenType};

use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
    coordinates::Coordinates,
    dance::{
        Beat, DanceScript, Dancer, DancerId, DancerSpecifier, Instruction, Metadata, Movement,
        PositionExpression, ShapeDefinition, ShapeId, StageSize, Start, StartInstruction, Step,
    },
};

#[derive(Debug, thiserror::Error)]
#[error("{kind} (at {span})")]
pub struct ParseError {
    pub span: Span,
    pub kind: ParseErrorKind,
}

impl ParseError {
    /// Build an [`ParseErrorKind::Expected`] error, taking its span from the
    /// offending token.
    fn expected(what: &'static str, token: Token) -> Self {
        ParseError {
            span: token.span,
            kind: ParseErrorKind::Expected(what, token.ty),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseErrorKind {
    #[error("expected {0}, got {1:?}")]
    Expected(&'static str, TokenType),
    #[error("property {0} already set")]
    AlreadySet(&'static str),
    #[error("property {0} not set")]
    NotSet(&'static str),
    #[error("dancer {0} already exists")]
    DancerExists(String),
    #[error("dancer {0} does not exist")]
    UndefinedDancer(String),
    #[error("dancer {0} already has a start position")]
    DancerAlreadyStarted(String),
    #[error("shape {0} does not exist")]
    UndefinedShape(String),
    #[error("Unexpected EOF")]
    UnexpectedEof,
    #[error("unexpected property {0}")]
    UnexpectedProperty(String),
    #[error("expected a single dancer")]
    ExpectedSingleDancer,
    #[error("expected explicit dancers, not 'all'")]
    ExpectedExplicitDancers,
    #[error("{0} is not yet implemented")]
    NotImplemented(&'static str),
    #[error(transparent)]
    Lex(#[from] LexingError),
}

fn set_once<T>(
    option: &mut Option<T>,
    value: T,
    prop: &'static str,
    span: Span,
) -> Result<(), ParseError> {
    if option.is_some() {
        return Err(ParseError {
            span,
            kind: ParseErrorKind::AlreadySet(prop),
        });
    }

    *option = Some(value);
    Ok(())
}

use TokenType as T;

macro_rules! expect {
    ($self:expr, $( $token:pat ),* $(,)?) => {
        $(
            let __macro_get_t = $self.next()?;
            let Token { ty: $token, .. } = __macro_get_t else {
                return Err(ParseError::expected(stringify!($token), __macro_get_t));
            };
        )*
    };
}

macro_rules! maybe {
    ($self:expr, $token:pat) => {{
        let exists = matches!($self.tokens.front(), Some(Token { ty: $token, .. }));
        if exists {
            $self.tokens.pop_front();
        }
        exists
    }};
}

pub struct Parser {
    tokens: VecDeque<Token>,
    errors: Vec<ParseError>,

    /// Span of the most recently consumed token, used to locate errors that
    /// aren't tied to a specific token (e.g. unexpected EOF).
    current_span: Span,

    shape_ids: HashMap<String, ShapeId>,
    shapes: HashMap<ShapeId, ShapeDefinition>,
    next_shape_id: u64,

    dancer_ids: HashMap<String, DancerId>,
    dancers: HashMap<DancerId, Dancer>,
    next_dancer_id: u64,

    /// Dancers that have already been given a start position, so a second
    /// assignment in the `start` block is rejected.
    started: HashSet<DancerId>,
}

impl Parser {
    pub fn new(lexer: Lexer) -> Self {
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        for token in lexer {
            match token {
                Ok(t) => tokens.push(t),
                Err(e) => errors.push(e),
            }
        }

        Self {
            tokens: VecDeque::from(tokens),
            errors,
            current_span: Span::default(),
            shape_ids: HashMap::new(),
            shapes: HashMap::new(),
            next_shape_id: 0,
            dancer_ids: HashMap::new(),
            dancers: HashMap::new(),
            next_dancer_id: 0,
            started: HashSet::new(),
        }
    }

    fn next(&mut self) -> Result<Token, ParseError> {
        match self.tokens.pop_front() {
            Some(token) => {
                self.current_span = token.span;
                Ok(token)
            }
            None => Err(self.error(ParseErrorKind::UnexpectedEof)),
        }
    }

    /// Attach the current span to an error kind. Used for errors that don't
    /// carry their own token.
    fn error(&self, kind: ParseErrorKind) -> ParseError {
        ParseError {
            span: self.current_span,
            kind,
        }
    }

    fn push_error(&mut self, err: ParseError) {
        self.errors.push(err);
    }

    /// Panic-mode recovery: discard tokens until the next one is an anchor
    /// (a token that can begin the following item, or closes the block), or
    /// we reach EOF. Consumes nothing if already parked on an anchor, so a
    /// purely semantic error doesn't swallow the next good item.
    fn synchronize(&mut self, is_anchor: impl Fn(&TokenType) -> bool) {
        while let Some(front) = self.tokens.front() {
            if is_anchor(&front.ty) {
                break;
            }
            self.tokens.pop_front();
        }
    }

    /// Parse a `}`-terminated sequence of items, recovering past any that fail
    /// so a single bad item doesn't abort the whole block. `anchor` identifies
    /// where to resume after an error (and must include `RBrace`).
    fn parse_block<Item>(
        &mut self,
        mut parse_item: impl FnMut(&mut Self) -> Result<Item, ParseError>,
        anchor: impl Fn(&TokenType) -> bool,
    ) -> Vec<Item> {
        let mut items = Vec::new();
        loop {
            if maybe!(self, T::RBrace) || self.tokens.is_empty() {
                break;
            }

            let before = self.tokens.len();
            match parse_item(self) {
                Ok(item) => items.push(item),
                Err(err) => {
                    self.push_error(err);
                    self.synchronize(&anchor);
                    // Guarantee forward progress even if recovery consumed nothing.
                    if self.tokens.len() == before {
                        self.tokens.pop_front();
                    }
                }
            }
        }
        items
    }

    fn get_shape(&self, ident: &str) -> Result<ShapeId, ParseError> {
        self.shape_ids
            .get(ident)
            .copied()
            .ok_or_else(|| self.error(ParseErrorKind::UndefinedShape(ident.to_string())))
    }

    fn register_anonymous_shape(&mut self, shape: ShapeDefinition) -> Result<ShapeId, ParseError> {
        let id = ShapeId(self.next_shape_id);
        self.shapes.insert(id, shape);
        self.next_shape_id += 1;
        Ok(id)
    }

    fn register_named_shape(
        &mut self,
        ident: String,
        shape: ShapeDefinition,
    ) -> Result<ShapeId, ParseError> {
        let id = self.register_anonymous_shape(shape)?;
        self.shape_ids.insert(ident, id);
        Ok(id)
    }

    fn get_dancer(&self, ident: &str) -> Result<DancerId, ParseError> {
        self.dancer_ids
            .get(ident)
            .copied()
            .ok_or_else(|| self.error(ParseErrorKind::UndefinedDancer(ident.to_string())))
    }

    /// Reverse-lookup a dancer's source identifier from its id, for error
    /// messages. Every id comes from `get_dancer`, so this should always hit.
    fn dancer_ident(&self, id: DancerId) -> String {
        self.dancer_ids
            .iter()
            .find_map(|(ident, &d)| (d == id).then(|| ident.clone()))
            .unwrap_or_default()
    }

    /// Record that `dancer` has been given a start position, erroring (pointed
    /// at `span`) if it already had one.
    fn mark_started(&mut self, dancer: DancerId, span: Span) -> Result<(), ParseError> {
        if self.started.insert(dancer) {
            Ok(())
        } else {
            Err(ParseError {
                span,
                kind: ParseErrorKind::DancerAlreadyStarted(self.dancer_ident(dancer)),
            })
        }
    }

    fn register_dancer(&mut self, ident: String, dancer: Dancer) -> Result<DancerId, ParseError> {
        if self.dancer_ids.contains_key(&ident) {
            return Err(self.error(ParseErrorKind::DancerExists(ident)));
        }

        let id = DancerId(self.next_dancer_id);
        self.dancer_ids.insert(ident, id);
        self.dancers.insert(id, dancer);
        self.next_dancer_id += 1;

        Ok(id)
    }

    fn parse_float(&mut self) -> Result<f64, ParseError> {
        let t = self.next()?;
        match t.ty {
            T::UInt(v) => Ok(v as f64),
            T::Float(v) => Ok(v),
            _ => Err(ParseError::expected("float", t)),
        }
    }

    fn parse_coordinates(&mut self) -> Result<Coordinates, ParseError> {
        expect!(self, T::LParen);
        let x = self.parse_float()?;
        expect!(self, T::Comma);
        let y = self.parse_float()?;
        expect!(self, T::RParen);

        Ok(Coordinates::new(x, y))
    }

    fn parse_position(&mut self) -> Result<PositionExpression, ParseError> {
        let front = self
            .tokens
            .front()
            .ok_or_else(|| self.error(ParseErrorKind::UnexpectedEof))?;
        let position = match front.ty.clone() {
            T::Ident(shape) => {
                self.next()?;
                expect!(
                    self,
                    T::LBracket,
                    T::UInt(index),
                    T::Slash,
                    T::UInt(positions),
                    T::RBracket
                );

                let shape_id = self.get_shape(&shape)?;
                PositionExpression::Shape(shape_id, index, positions)
            }
            _ => {
                let coordinates = self.parse_coordinates()?;
                PositionExpression::Literal(coordinates)
            }
        };
        Ok(position)
    }

    fn parse_shape_literal(&mut self, shape_name: String) -> Result<ShapeDefinition, ParseError> {
        let shape = match shape_name.as_str() {
            "line" => {
                let mut from = None;
                let mut through = None;

                let braced = maybe!(self, T::LBrace);

                while !{
                    if braced {
                        maybe!(self, T::RBrace)
                    } else {
                        maybe!(self, T::Semicolon)
                    }
                } {
                    expect!(self, T::Ident(key));
                    match key.as_str() {
                        "from" => {
                            let value = self.parse_coordinates()?;
                            set_once(&mut from, value, "from", self.current_span)?
                        }
                        "through" => {
                            let value = self.parse_coordinates()?;
                            set_once(&mut through, value, "through", self.current_span)?
                        }
                        _ => {
                            return Err(self.error(ParseErrorKind::UnexpectedProperty(key)));
                        }
                    }
                }

                ShapeDefinition::Line {
                    from: from.ok_or_else(|| self.error(ParseErrorKind::NotSet("from")))?,
                    through: through
                        .ok_or_else(|| self.error(ParseErrorKind::NotSet("through")))?,
                }
            }
            "horseshoe" => {
                let mut about = None;
                let mut closure = None;
                let mut width = None;
                let mut depth = None;

                let braced = maybe!(self, T::LBrace);

                while !{
                    if braced {
                        maybe!(self, T::RBrace)
                    } else {
                        maybe!(self, T::Semicolon)
                    }
                } {
                    expect!(self, T::Ident(key));
                    match key.as_str() {
                        "about" => {
                            let value = self.parse_coordinates()?;
                            set_once(&mut about, value, "about", self.current_span)?
                        }
                        "closure" => {
                            let value = self.parse_float()?;
                            set_once(&mut closure, value, "closure", self.current_span)?
                        }
                        "width" => {
                            let value = self.parse_float()?;
                            set_once(&mut width, value, "width", self.current_span)?
                        }
                        "depth" => {
                            let value = self.parse_float()?;
                            set_once(&mut depth, value, "depth", self.current_span)?
                        }
                        _ => {
                            return Err(self.error(ParseErrorKind::UnexpectedProperty(key)));
                        }
                    }
                }

                ShapeDefinition::Horseshoe {
                    about: about.ok_or_else(|| self.error(ParseErrorKind::NotSet("about")))?,
                    closure: closure
                        .ok_or_else(|| self.error(ParseErrorKind::NotSet("closure")))?,
                    width: width.ok_or_else(|| self.error(ParseErrorKind::NotSet("width")))?,
                    depth: depth.ok_or_else(|| self.error(ParseErrorKind::NotSet("depth")))?,
                }
            }
            _ => return Err(self.error(ParseErrorKind::UndefinedShape(shape_name))),
        };

        Ok(shape)
    }

    fn parse_shape(&mut self) -> Result<ShapeId, ParseError> {
        expect!(self, T::Ident(shape_name));
        let shape_id = match shape_name.as_str() {
            "line" | "horseshoe" => {
                let shape = self.parse_shape_literal(shape_name)?;
                self.register_anonymous_shape(shape)?
            }
            _ => {
                expect!(self, T::Semicolon);
                self.get_shape(&shape_name)?
            }
        };

        Ok(shape_id)
    }

    fn parse_start_line(&mut self) -> Result<StartInstruction, ParseError> {
        let dancers = self.parse_dancer_list()?;
        let dancers_span = self.current_span;

        let t = self.next()?;
        let instruction = match t.ty {
            T::Along => {
                let shape = self.parse_shape()?;
                for &dancer in &dancers {
                    self.mark_started(dancer, dancers_span)?;
                }
                StartInstruction::Shape { dancers, shape }
            }
            T::At => {
                let [dancer] = dancers.as_slice() else {
                    return Err(self.error(ParseErrorKind::ExpectedSingleDancer));
                };
                let dancer = *dancer;

                let position = self.parse_position()?;

                expect!(self, T::Semicolon);
                self.mark_started(dancer, dancers_span)?;
                StartInstruction::Position { dancer, position }
            }
            _ => return Err(ParseError::expected("along or to", t)),
        };

        Ok(instruction)
    }

    fn parse_dancer(&mut self) -> Result<DancerId, ParseError> {
        expect!(self, T::Ident(id));
        self.get_dancer(&id)
    }

    fn parse_dancer_list(&mut self) -> Result<Vec<DancerId>, ParseError> {
        let t = self.next()?;
        match t.ty {
            T::Ident(id) => Ok(vec![self.get_dancer(&id)?]),
            T::LBracket => {
                let mut dancers = Vec::new();

                while !maybe!(self, T::RBracket) {
                    let dancer = self.parse_dancer()?;
                    dancers.push(dancer);

                    if !maybe!(self, T::Comma) {
                        expect!(self, T::RBracket);
                        break;
                    }
                }

                Ok(dancers)
            }
            _ => Err(ParseError::expected("dancer list", t)),
        }
    }

    fn parse_dancer_specifier(&mut self) -> Result<DancerSpecifier, ParseError> {
        if maybe!(self, T::All) {
            let exceptions = if maybe!(self, T::Except) {
                self.parse_dancer_list()?
            } else {
                vec![]
            };
            Ok(DancerSpecifier::AllExcept(exceptions))
        } else {
            let dancers = self.parse_dancer_list()?;
            Ok(DancerSpecifier::Dancers(dancers))
        }
    }

    /// Parse either a `{ ... }` block of items or a single bare item, recovering
    /// past failures. `block_anchor` resynchronises within braces; `single_anchor`
    /// resynchronises after a failed bare item (to the next enclosing statement).
    fn parse_braced_or_single<Item>(
        &mut self,
        mut parse_item: impl FnMut(&mut Self) -> Result<Item, ParseError>,
        block_anchor: impl Fn(&TokenType) -> bool,
        single_anchor: impl Fn(&TokenType) -> bool,
    ) -> Vec<Item> {
        if maybe!(self, T::LBrace) {
            self.parse_block(parse_item, block_anchor)
        } else {
            let mut items = Vec::new();
            match parse_item(self) {
                Ok(item) => items.push(item),
                Err(err) => {
                    self.push_error(err);
                    self.synchronize(single_anchor);
                }
            }
            items
        }
    }

    fn parse_instruction(&mut self) -> Result<Instruction, ParseError> {
        let first = self.next()?;
        let instruction = match first.ty {
            T::Move => {
                let dancers = self.parse_dancer_specifier()?;
                let next = self.next()?;
                let movement = match next.ty {
                    T::Along => {
                        let DancerSpecifier::Dancers(dancers) = dancers else {
                            return Err(self.error(ParseErrorKind::ExpectedExplicitDancers));
                        };

                        Movement::AlongShape {
                            dancers,
                            shape: self.parse_shape()?,
                        }
                    }
                    T::To => {
                        let DancerSpecifier::Dancers(dancers) = dancers else {
                            return Err(self.error(ParseErrorKind::ExpectedExplicitDancers));
                        };

                        let [dancer] = *dancers.as_slice() else {
                            return Err(self.error(ParseErrorKind::ExpectedSingleDancer));
                        };

                        let position = self.parse_position()?;
                        Movement::ToPosition { dancer, position }
                    }
                    T::By => {
                        let delta = self.parse_coordinates()?;

                        let repeat = maybe!(self, T::Every)
                            .then(|| {
                                let value = self.parse_float()?;
                                expect!(self, T::Beats);
                                Ok(value)
                            })
                            .transpose()?;

                        Movement::ByDelta {
                            dancers,
                            delta,
                            repeat,
                        }
                    }
                    _ => return Err(ParseError::expected("along, to or by", next)),
                };

                let until = maybe!(self, T::Until)
                    .then(|| {
                        expect!(self, T::Beat(bar, beat));
                        Ok(Beat(bar, beat as f64))
                    })
                    .transpose()?;

                Instruction { movement, until }
            }
            T::Rotate => return Err(self.error(ParseErrorKind::NotImplemented("rotate"))),
            _ => return Err(ParseError::expected("move or rotate", first)),
        };

        expect!(self, T::Semicolon);

        Ok(instruction)
    }

    pub fn parse(mut self) -> Result<DanceScript, Vec<ParseError>> {
        let meta = match self.parse_meta() {
            Ok(meta) => meta,
            Err(err) => {
                self.push_error(err);
                self.synchronize(|ty| matches!(ty, T::Dancers | T::Start | T::At | T::Let));
                None
            }
        };

        if let Err(err) = self.parse_dancers() {
            self.push_error(err);
            self.synchronize(|ty| matches!(ty, T::Start | T::At | T::Let));
        }

        let start = match self.parse_start() {
            Ok(start) => start,
            Err(err) => {
                self.push_error(err);
                self.synchronize(|ty| matches!(ty, T::At | T::Let));
                Vec::new()
            }
        };

        let steps = self.parse_body();

        // All-or-nothing: a fully clean parse yields the tree; otherwise every
        // error we recovered from is returned together.
        if self.errors.is_empty()
            && let Some(meta) = meta
        {
            return Ok(DanceScript {
                meta,
                dancers: self.dancers,
                shapes: self.shapes,
                start: Start {
                    instructions: start,
                },
                steps,
            });
        }

        Err(self.errors)
    }

    /// Parse the `meta { ... }` block. Returns `None` (after recording errors)
    /// if the header is malformed or any required property is missing.
    fn parse_meta(&mut self) -> Result<Option<Metadata>, ParseError> {
        expect!(self, T::Meta, T::LBrace);

        let mut title = None;
        let mut bpm = None;
        let mut bar_length = None;
        let mut stage_size = None;

        loop {
            if maybe!(self, T::RBrace) || self.tokens.is_empty() {
                break;
            }

            let before = self.tokens.len();
            if let Err(err) =
                self.parse_meta_property(&mut title, &mut bpm, &mut bar_length, &mut stage_size)
            {
                self.push_error(err);
                self.synchronize(|ty| {
                    matches!(ty, T::Title | T::Bpm | T::Barlength | T::Stagesize | T::RBrace)
                });
                if self.tokens.len() == before {
                    self.tokens.pop_front();
                }
            }
        }

        match (title, bpm, bar_length, stage_size) {
            (Some(title), Some(bpm), Some(beats_per_bar), Some(stage_size)) => Ok(Some(Metadata {
                title,
                bpm,
                beats_per_bar,
                stage_size,
            })),
            (title, bpm, bar_length, stage_size) => {
                if title.is_none() {
                    self.push_error(self.error(ParseErrorKind::NotSet("title")));
                }
                if bpm.is_none() {
                    self.push_error(self.error(ParseErrorKind::NotSet("bpm")));
                }
                if bar_length.is_none() {
                    self.push_error(self.error(ParseErrorKind::NotSet("barlength")));
                }
                if stage_size.is_none() {
                    self.push_error(self.error(ParseErrorKind::NotSet("stagesize")));
                }
                Ok(None)
            }
        }
    }

    fn parse_meta_property(
        &mut self,
        title: &mut Option<String>,
        bpm: &mut Option<f64>,
        bar_length: &mut Option<u8>,
        stage_size: &mut Option<StageSize>,
    ) -> Result<(), ParseError> {
        let t = self.next()?;
        match t.ty {
            T::Title => {
                expect!(self, T::StringLiteral(value));
                set_once(title, value, "title", self.current_span)?;
            }
            T::Bpm => {
                let value = self.parse_float()?;
                set_once(bpm, value, "bpm", self.current_span)?;
            }
            T::Barlength => {
                expect!(self, T::UInt(length));
                set_once(bar_length, length as u8, "barlength", self.current_span)?;
            }
            T::Stagesize => {
                expect!(
                    self,
                    T::LParen,
                    T::UInt(width),
                    T::Comma,
                    T::UInt(depth),
                    T::RParen
                );
                set_once(
                    stage_size,
                    StageSize { width, depth },
                    "stagesize",
                    self.current_span,
                )?;
            }
            _ => return Err(ParseError::expected("metadata property", t)),
        }
        Ok(())
    }

    /// Parse the `dancers { ... }` block, registering each dancer.
    fn parse_dancers(&mut self) -> Result<(), ParseError> {
        expect!(self, T::Dancers, T::LBrace);
        self.parse_block(Self::parse_dancer_decl, |ty| {
            matches!(ty, T::Ident(_) | T::RBrace)
        });
        Ok(())
    }

    fn parse_dancer_decl(&mut self) -> Result<(), ParseError> {
        expect!(
            self,
            T::Ident(ident),
            T::Is,
            T::StringLiteral(name),
            T::In,
            T::Color(color),
            T::Semicolon,
        );
        self.register_dancer(ident, Dancer { name, color })?;
        Ok(())
    }

    /// Parse the `start` section (a braced block or a single line).
    fn parse_start(&mut self) -> Result<Vec<StartInstruction>, ParseError> {
        expect!(self, T::Start);
        Ok(self.parse_braced_or_single(
            Self::parse_start_line,
            |ty| matches!(ty, T::Ident(_) | T::LBracket | T::RBrace),
            |ty| matches!(ty, T::At | T::Let),
        ))
    }

    /// Parse the sequence of top-level `at` / `let` statements until EOF.
    fn parse_body(&mut self) -> Vec<Step> {
        let mut steps = Vec::new();
        while !self.tokens.is_empty() {
            let before = self.tokens.len();
            match self.parse_statement() {
                Ok(Some(step)) => steps.push(step),
                Ok(None) => {}
                Err(err) => {
                    self.push_error(err);
                    self.synchronize(|ty| matches!(ty, T::At | T::Let));
                    if self.tokens.len() == before {
                        self.tokens.pop_front();
                    }
                }
            }
        }
        steps
    }

    /// Parse a single top-level statement. Returns `Some` for an `at` step and
    /// `None` for a `let` shape definition (a pure side effect).
    fn parse_statement(&mut self) -> Result<Option<Step>, ParseError> {
        let token = self.next()?;
        match token.ty {
            T::At => {
                expect!(self, T::Beat(bar, beat));
                let instructions = self.parse_braced_or_single(
                    Self::parse_instruction,
                    |ty| matches!(ty, T::Move | T::Rotate | T::RBrace),
                    |ty| matches!(ty, T::At | T::Let),
                );
                Ok(Some(Step {
                    beat: Beat(bar, beat as f64),
                    instructions,
                }))
            }
            T::Let => {
                expect!(self, T::Ident(ident), T::Equals, T::Ident(shape_name));
                let definition = self.parse_shape_literal(shape_name)?;
                self.register_named_shape(ident, definition)?;
                Ok(None)
            }
            _ => Err(ParseError::expected("at or let clause", token)),
        }
    }
}
