use core::marker::PhantomData;

use crate::*;

/// A simple wrapper to make ParseImpl easier to use. 
pub struct Parser<T>(PhantomData<T>);

impl<T> Parser<T> {
    /// Parse without extra value
    pub fn parse(input: &str) -> Result<T, ()> 
        where T: ParseImpl<0, false> + AstImpl<()>
    {
        // Parse input input a tag stack
        let mut trace = Vec::new();
        let mut stack = Vec::new();
        let end = 0;
        <T as ParseImpl<0, false>>::parse_impl(input, end, &mut trace, &mut stack)?;
        // Analyze the tag stack into this value
        Ok(T::ast(input, &stack, ()).1)
    }
    /// Parse with extra value provided
    pub fn parse_with<Extra>(input: &str, with: Extra) -> Result<T, ()> 
        where T: ParseImpl<0, false> + AstImpl<Extra>,
              Extra: Copy
    {
        // Parse input input a tag stack
        let mut trace = Vec::new();
        let mut stack = Vec::new();
        let end = 0;
        <T as ParseImpl<0, false>>::parse_impl(input, end, &mut trace, &mut stack)?;
        // Analyze the tag stack into this value, with extra value attached
        Ok(T::ast(input, &stack, with).1)
    }
    /// Only parse into a tag stack
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