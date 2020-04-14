use std::borrow::Cow;
use std::fmt;
use std::num::{ParseFloatError, ParseIntError};
use std::path::PathBuf;
use std::string::FromUtf8Error;
use wain_syntax_text::lexer::{LexError, Token};
use wain_syntax_text::parser::ParseError;
use wain_syntax_text::source::describe_position;
use wain_syntax_text::wat2wasm::TransformError;

pub enum ErrorKind<'source> {
    Parse(ParseKind<'source>),
}

pub enum ParseKind<'source> {
    Unexpected {
        expected: Cow<'static, str>,
        token: Option<Token<'source>>,
    },
    EndOfFile {
        expected: &'static str,
    },
    Utf8Error(FromUtf8Error),
    InvalidStringLiteral {
        lit: &'source str,
        reason: &'static str,
    },
    InvalidInt {
        ty: &'static str,
        err: ParseIntError,
    },
    TooSmallInt {
        ty: &'static str,
        digits: u64,
    },
    InvalidFloat {
        ty: &'static str,
        err: ParseFloatError,
    },
    InvalidHexFloat {
        ty: &'static str,
    },
    Lex(LexError<'source>),
    ParseWat(ParseError<'source>),
    Wat2Wasm(TransformError<'source>),
}

pub struct Error<'source> {
    pub pos: usize,
    source: &'source str,
    kind: ErrorKind<'source>,
    pub prev_error: Option<Box<Error<'source>>>,
    file: Option<PathBuf>,
}

impl<'s> Error<'s> {
    pub fn parse_error(kind: ParseKind<'s>, source: &'s str, pos: usize) -> Box<Error<'s>> {
        Box::new(Error {
            pos,
            source,
            kind: ErrorKind::Parse(kind),
            prev_error: None,
            file: None,
        })
    }

    pub fn set_file(&mut self, p: PathBuf) {
        self.file = Some(p);
    }
}

macro_rules! parse_error_from {
    ($from:ty, $kind:ident) => {
        impl<'s> From<Box<$from>> for Box<Error<'s>> {
            fn from(err: Box<$from>) -> Box<Error<'s>> {
                let source = err.source();
                let offset = err.offset();
                Error::parse_error(ParseKind::$kind(*err), source, offset)
            }
        }
    };
}
parse_error_from!(LexError<'s>, Lex);
parse_error_from!(ParseError<'s>, ParseWat);
parse_error_from!(TransformError<'s>, Wat2Wasm);

impl<'s> fmt::Display for Error<'s> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let doing = match &self.kind {
            ErrorKind::Parse(kind) => {
                use ParseKind::*;
                match kind {
                    Lex(err) => write!(f, "lexer error: {}", err)?,
                    ParseWat(err) => write!(f, "parse error on parsing WAT module: {}", err)?,
                    Wat2Wasm(err) => write!(f, "could not transform from WAT to WASM: {}", err)?,
                    Unexpected {
                        expected,
                        token: None,
                    } => write!(f, "unexpected token while {} is expected", expected)?,
                    Unexpected {
                        expected,
                        token: Some(token),
                    } => write!(
                        f,
                        "unexpected token {} while {} is expected",
                        token, expected
                    )?,
                    EndOfFile { expected } => {
                        write!(f, "unxpected EOF while {} is expected", expected)?
                    }
                    Utf8Error(err) => write!(f, "cannot parse text as UTF-8: {}", err)?,
                    InvalidStringLiteral { lit, reason } => {
                        write!(f, "invalid string literal '{}': {}", lit, reason)?
                    }
                    InvalidInt { ty, err } => write!(f, "invalid int literal for {}: {}", ty, err)?,
                    TooSmallInt { ty, digits } => {
                        write!(f, "-{} is too small value for {}", digits, ty)?
                    }
                    InvalidFloat { ty, err } => {
                        write!(f, "invalid float number literal for {}: {}", ty, err)?
                    }
                    InvalidHexFloat { ty } => {
                        write!(f, "invalid hex float number literal for {}", ty)?
                    }
                }
                "parsing"
            }
        };

        write!(f, " while {}", doing)?;
        if let Some(path) = &self.file {
            write!(f, " '{:?}'", path)?;
        }

        describe_position(f, self.source, self.pos)?;

        if let Some(prev) = &self.prev_error {
            write!(
                f,
                "\n\nabove error may be caused by below previous error: {}",
                prev
            )?;
        }

        Ok(())
    }
}

impl<'s> fmt::Debug for Error<'s> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Display>::fmt(self, f)
    }
}

pub type Result<'s, T> = ::std::result::Result<T, Box<Error<'s>>>;
