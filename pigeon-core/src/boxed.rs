use alloc::boxed::Box;
use crate::*;

impl<const GROUP: usize, const ERROR: bool, T> ParseImpl<GROUP, ERROR> for Box<T> 
    where T: ParseImpl<GROUP, ERROR>
{
    fn parse_impl(
        input: &str, end: usize,
        trace: &mut Vec<(usize, usize)>,
        stack: &mut Vec<Tag>,
    ) -> Result<usize, ()> {
        <T as ParseImpl<GROUP, ERROR>>::parse_impl(input, end, trace, stack)
    }
}

impl<Extra, T> Ast<Extra> for Box<T> 
    where T: Ast<Extra>
{
    fn ast<'a>(
        input: &'a str, 
        stack: &'a [Tag], 
        extra: &'a Extra
    ) -> (&'a [Tag], Self) {
        let (rest, this) = T::ast(input, stack, extra);
        (rest, Box::new(this))
    }
}