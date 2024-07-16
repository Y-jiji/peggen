use crate::*;

// impl_u!{u8 u16 u32 u64 u128}

impl<'a, E> ParseImpl<'a, E> for u8 where 
    E: ErrorImpl<'a>,
{
    fn parse_impl(
        input: &'a str, 
        begin: usize,
        arena: &'a Arena,
        precedence: u16,
    ) -> Result<(Self, usize), E> {
        todo!()
    }
}

// impl_i!{i8 i16 i32 i64 i128}

// impl_f!{f32 f64}