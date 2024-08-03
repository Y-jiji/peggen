use core::marker::PhantomData;

use crate::*;

pub struct Parser<T: ParseImpl<0, ERROR> + AstImpl<Extra>, Extra: Copy = (), const ERROR: bool=false>(PhantomData<(T, Extra)>);

impl<T: ParseImpl<0, ERROR> + AstImpl<Extra>, Extra: Copy, const ERROR: bool> Parser<T, Extra, ERROR> {
    pub fn parse_with(input: &str, extra: Extra) -> Result<T, ()> {
        let mut trace = Vec::new();
        let mut stack = Vec::new();
        let end = 0;
        <T as ParseImpl<0, ERROR>>::parse_impl(input, end, &mut trace, &mut stack)?;
        Ok(T::ast(input, &stack, extra).1)
    }
}

impl<T: ParseImpl<0, ERROR> + AstImpl<()>, const ERROR: bool> Parser<T, (), ERROR> {
    pub fn parse(input: &str) -> Result<T, ()> {
        let mut trace = Vec::new();
        let mut stack = Vec::new();
        let end = 0;
        <T as ParseImpl<0, ERROR>>::parse_impl(input, end, &mut trace, &mut stack)?;
        Ok(T::ast(input, &stack, ()).1)
    }
    pub fn sequence(input: &str) -> Result<Vec<Tag>, ()> {
        let mut trace = Vec::new();
        let mut stack = Vec::new();
        let end = 0;
        <T as ParseImpl<0, ERROR>>::parse_impl(input, end, &mut trace, &mut stack)?;
        Ok(stack)
    }
}