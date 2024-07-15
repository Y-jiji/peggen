// use crate::*;

// macro_rules! impl_i {($($int: ty)*) => {$(
//     impl<'a> ParserImpl<'a> for $int {
//         #[allow(unused)]
//         fn parser_impl(
//             source: Source<'a>, 
//             out_arena: &'a Arena,
//             err_arena: &'a Arena,
//             precedence: u16,
//         ) -> Result<(Self, Source<'a>), Error<'a>> {
//             let _ = out_arena;
//             let _ = precedence;
//             let mut by = 0;
//             if source[..].starts_with("-") { by += 1; }
//             for c in source[by..].chars() {
//                 if !c.is_ascii_digit() { break }
//                 by += c.len_utf8();
//             }
//             match source[..by].parse::<$int>() {
//                 Ok(out) => Ok((out, source.proceed(by))),
//                 Err(_) => Err(Error::Mismatch {
//                     range: (source.split, source.split + by), 
//                     token: stringify!($int), 
//                 })
//             }
//         }
//     })*
// };}

// impl_i!{i8 i16 i32 i64 i128}

// macro_rules! impl_u {($($int: ty)*) => {$(
//     impl<'a> ParserImpl<'a> for $int {
//         #[allow(unused)]
//         fn parser_impl(
//             source: Source<'a>, 
//             out_arena: &'a Arena,
//             err_arena: &'a Arena,
//             precedence: u16,
//         ) -> Result<(Self, Source<'a>), Error<'a>> {
//             let _ = out_arena;
//             let _ = precedence;
//             let mut by = 0;
//             for c in source[by..].chars() {
//                 if !c.is_ascii_digit() { break }
//                 by += c.len_utf8();
//             }
//             match source[..by].parse::<$int>() {
//                 Ok(out) => Ok((out, source.proceed(by))),
//                 Err(_) => Err(Error::Mismatch {
//                     range: (source.split, source.split + by), 
//                     token: stringify!($int), 
//                 })
//             }
//         }
//     })*
// };}

// impl_u!{u8 u16 u32 u64 u128}

// macro_rules! impl_f {($($flt: ty)*) => {$(
//     impl<'a> ParserImpl<'a> for $flt {
//         #[allow(unused)]
//         fn parser_impl(
//             source: Source<'a>, 
//             out_arena: &'a Arena,
//             err_arena: &'a Arena,
//             precedence: u16,
//         ) -> Result<(Self, Source<'a>), Error<'a>> {
//             let _ = out_arena;
//             let _ = precedence;
//             let mut by  = 0;
//             let mut dot = false;
//             if source[..].starts_with("-") { by += 1; }
//             for c in source[by..].chars() {
//                 if c == '.' && !dot {
//                     dot = true;
//                     by += c.len_utf8();
//                     continue; 
//                 }
//                 if !c.is_ascii_digit() { break }
//                 by += c.len_utf8();
//             }
//             match source[..by].parse::<$flt>() {
//                 Ok(out) => Ok((out, source.proceed(by))),
//                 Err(_) => Err(Error::Mismatch {
//                     range: (source.split, source.split + by), 
//                     token: stringify!($flt), 
//                 })
//             }
//         }
//     })*
// };}

// impl_f!{f32 f64}