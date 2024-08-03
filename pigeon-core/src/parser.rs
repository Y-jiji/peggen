use core::marker::PhantomData;

use crate::*;

pub struct Parser<T>(PhantomData<T>);

impl<T> Parser<T> {
    pub fn parse(input: &str) -> Result<T, ()> 
        where T: ParseImpl<0, false> + AstImpl<()>
    {
        let mut trace = Vec::new();
        let mut stack = Vec::new();
        let end = 0;
        <T as ParseImpl<0, false>>::parse_impl(input, end, &mut trace, &mut stack)?;
        Ok(T::ast(input, &stack, ()).1)
    }
    pub fn parse_with<Extra>(input: &str, with: Extra) -> Result<T, ()> 
        where T: ParseImpl<0, false> + AstImpl<Extra>,
              Extra: Copy
    {
        let mut trace = Vec::new();
        let mut stack = Vec::new();
        let end = 0;
        <T as ParseImpl<0, false>>::parse_impl(input, end, &mut trace, &mut stack)?;
        Ok(T::ast(input, &stack, with).1)
    }
    pub fn sequence(input: &str) -> Result<Vec<Tag>, ()> 
        where T: ParseImpl<0, false> + AstImpl<()>
    {
        let mut trace = Vec::new();
        let mut stack = Vec::new();
        let end = 0;
        <T as ParseImpl<0, false>>::parse_impl(input, end, &mut trace, &mut stack)?;
        Ok(stack)
    }
}