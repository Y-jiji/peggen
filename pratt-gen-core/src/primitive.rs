use once_cell::sync::Lazy;
use crate::*;

macro_rules! impl_i {($($int: ty)*) => {$(
    impl<'a> ParserImpl<'a> for $int {
        fn parser_impl(
            source: Source<'a>, 
            out_arena: &'a Arena,
            err_arena: &'a Arena,
            precedence: u16,
        ) -> Result<(Self, Source<'a>), Error<'a>> {
            let _ = out_arena;
            let _ = precedence;
            let mut by = 0;
            if source[..].starts_with("-") { by += 1; }
            for c in source[by..].chars() {
                if !c.is_ascii_digit() { break }
                by += c.len_utf8();
            }
            match source[..by].parse::<$int>() {
                Ok(out) => Ok((out, source.proceed(by))),
                Err(_) => Err(Error::Mismatch {
                    range: (source.split, source.split + by), 
                    token: stringify!($int), 
                    piece: unsafe { err_arena.alloc_str(&source[..source[..].chars().next().map(|x| x.len_utf8()).unwrap_or(0)]) }
                })
            }
        }
    })*
};}

impl_i!{i8 i16 i32 i64 i128}

macro_rules! impl_u {($($int: ty)*) => {$(
    impl<'a> ParserImpl<'a> for $int {
        fn parser_impl(
            source: Source<'a>, 
            out_arena: &'a Arena,
            err_arena: &'a Arena,
            precedence: u16,
        ) -> Result<(Self, Source<'a>), Error<'a>> {
            let _ = out_arena;
            let _ = precedence;
            let mut by = 0;
            for c in source[by..].chars() {
                if !c.is_ascii_digit() { break }
                by += c.len_utf8();
            }
            match source[..by].parse::<$int>() {
                Ok(out) => Ok((out, source.proceed(by))),
                Err(_) => Err(Error::Mismatch {
                    range: (source.split, source.split + by), 
                    token: stringify!($int), 
                    piece: unsafe { err_arena.alloc_str(&source[..source[..].chars().next().map(|x| x.len_utf8()).unwrap_or(0)]) }
                })
            }
        }
    })*
};}

impl_u!{u8 u16 u32 u64 u128}

macro_rules! impl_f {($($flt: ty)*) => {$(
    impl<'a> ParserImpl<'a> for $flt {
        fn parser_impl(
            source: Source<'a>, 
            out_arena: &'a Arena,
            err_arena: &'a Arena,
            precedence: u16,
        ) -> Result<(Self, Source<'a>), Error<'a>> {
            let _ = out_arena;
            let _ = precedence;
            let mut by  = 0;
            let mut dot = false;
            if source[..].starts_with("-") { by += 1; }
            for c in source[by..].chars() {
                if c == '.' && !dot {
                    dot = true;
                    by += c.len_utf8();
                    continue; 
                }
                if !c.is_ascii_digit() { break }
                by += c.len_utf8();
            }
            match source[..by].parse::<$flt>() {
                Ok(out) => Ok((out, source.proceed(by))),
                Err(_) => Err(Error::Mismatch {
                    range: (source.split, source.split + by), 
                    token: stringify!($flt), 
                    piece: unsafe { err_arena.alloc_str(&source[..source[..].chars().next().map(|x| x.len_utf8()).unwrap_or(0)]) }
                })
            }
        }
    })*
};}

impl_f!{f32 f64}

#[derive(Debug, Clone, Copy)]
pub struct Ident<'a>(pub &'a str);

impl<'a> Ident<'a> { pub fn to_str(&self) -> &'a str { self.0 } }

impl<'a> ParserImpl<'a> for Ident<'a> {
    fn parser_impl(
        source: Source<'a>, 
        out_arena: &'a Arena,
        err_arena: &'a Arena,
        _: u16,
    ) -> Result<(Self, Source<'a>), Error<'a>> {
        let mut len = 0;
        let mut char_len = 0;
        for c in source[..].chars() {
            if c.is_ascii_alphanumeric() {
                len += c.len_utf8();
            } else {
                char_len = c.len_utf8();
                break;
            }
        }
        unsafe {if len == 0 {
            Err(Error::Mismatch {
                range: (source.split, source.split + len), 
                token: "", 
                piece: err_arena.alloc_str(&source[..char_len])
            })
        } else {
            let ident = out_arena.alloc_str(&source[..len]);
            Ok((Ident(ident), source.proceed(len)))
        } }
    }
}

impl<'a> ParserImpl<'a> for &'a str {
    fn parser_impl(
        source: Source<'a>, 
        out_arena: &'a Arena,
        err_arena: &'a Arena,
        _: u16,
    ) -> Result<(Self, Source<'a>), Error<'a>> {
        use regex::Regex;
        static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(
            r#"^"([^"\\]|\\\\|\\")*""#
        ).unwrap());
        let Some(by) = REGEX.shortest_match(&source[..]) else {
            Err(Error::Mismatch {
                range: (source.split, source.split+2), 
                token: "<str>", 
                piece: unsafe{err_arena.alloc_str(&source[..source[..].chars().next().map(|x| x.len_utf8()).unwrap_or(0)])}
            })?
        };
        let s = unsafe{ out_arena.alloc_str(&source[1..by-1]) };
        Ok((s, source.proceed(by)))
    }
}