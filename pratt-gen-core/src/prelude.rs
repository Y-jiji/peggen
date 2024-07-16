use crate::*;

macro_rules! Primitive {($($($X: ident)* : $MESSAGE: literal: $REGEX: literal;)*) => {$($(
    impl<'a, E> ParseImpl<'a, E> for $X where 
        E: ErrorImpl<'a>,
    {
        fn parse_impl(
            input: &'a str, 
            begin: usize,
            _: &'a Arena,
            arena_err: &'a Arena,
            _: u16,
        ) -> Result<(Self, usize), E> {
            static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new($REGEX).unwrap());
            let Some(mat) = REGEX.find(&input[begin..]) else {
                Err(E::message(input, begin, arena_err, $MESSAGE, begin))?
            };
            let mat = mat.as_str();
            let end = begin + mat.len();
            let $X = match mat.parse::<$X>() {
                Ok($X) => $X,
                Err(_) => Err(E::message(input, begin, arena_err, $MESSAGE, end))?,
            };
            Ok(($X, end))
        }
    }
)*)*};}

Primitive!{
    f32 f64       : "floating point number" : r"^-?(0|[1-9][0-9]*)\.([0-9]+)";
    bool          : "boolean"           : r"^(true|false)";
    u8 u16 u32 u64: "unsigned integer"  : r"^(0|[1-9][0-9]*)";
    i8 i16 i32 i64: "signed integer"    : r"^-?(0|[1-9][0-9]*)";
}

/// ### Brief
/// Standard error type with no handling strategy. 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error<'a> {
    Rest {
        input: &'a str,
        begin: usize,
    },
    Mismatch {
        input: &'a str,
        begin: usize,
        token: &'static str,
    },
    Message {
        input: &'a str,
        begin: usize,
        message: &'static str,
        end: usize,
    },
    Unknown,
}

impl<'a> Error<'a> {
    /// ### Brief
    /// Get the estimated span of current error. 
    pub fn span(self) -> (usize, usize) {
        use Error::*;
        match self {
            Rest { begin, .. } => (begin, begin),
            Mismatch { begin, .. } => (begin, begin),
            Message { begin, end, .. } => (begin, end),
            Unknown => (0, 0),
        }
    }
}

impl<'a> Merge<'a> for Error<'a> {
    /// ### Brief
    /// Just keep the error with longer reach. 
    /// If the reach is the same, select the error with longer span. 
    fn merge(&self, that: &Self, _: &'a Arena) -> Self {
        let self_span = self.span();
        let that_span = that.span();
        if self_span.1 > that_span.1 {
            return *self;
        }
        if self_span.1 < that_span.1 {
            return *that;
        }
        if self_span.0 > that_span.0 {
            return *that;
        }
        if self_span.0 < that_span.0 {
            return *self;
        }
        return *self;
    }
}

impl<'a> ErrorImpl<'a> for Error<'a> {
    fn rest(
        input: &'a str, 
        begin: usize, 
        _: &'a Arena
    ) -> Self {
        Self::Rest { input, begin }
    }
    fn mismatch(
        input: &'a str,
        begin: usize,
        _: &'a Arena,
        token: &'static str
    ) -> Self {
        Self::Mismatch { input, begin, token }
    }
    fn message(
        input: &'a str,
        begin: usize,
        _: &'a Arena,
        message: &'static str,
        end: usize
    ) -> Self {
        Self::Message { input, begin, message, end }
    }
    fn error_impl(
        _: &'a str,
        _: usize,
        _: &'a Arena,
        _: u16,
    ) -> Result<(Self, usize), Self> {
        Err(Self::Unknown)
    }
}

/// ### Brief
/// Eat a token or raise an error. (Not very sure I should put it here. )
#[inline(always)]
pub fn token<'a, E: ErrorImpl<'a>>(
    input: &'a str, 
    begin: usize,
    arena: &'a Arena,
    expected: &'static str,
) -> Result<usize, E> {
    let begin = begin.min(input.len());
    let end = (begin+expected.len()).min(input.len());
    let piece = &input[begin..end];
    if expected == piece {
        return Ok(end)
    }
    Err(E::mismatch(input, begin, arena, expected))
}
