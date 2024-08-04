use alloc::string::String;

use crate::*;

impl<Extra: Copy> AstImpl<Extra> for String {
    fn ast<'a>(
        input: &'a str, 
        stack: &'a [Tag], 
        _: Extra
    ) -> (&'a [Tag], Self) {
        let tag = &stack[stack.len()-1];
        (
            &stack[..stack.len()-1],
            <Self as core::str::FromStr>::from_str(&input[tag.span.clone()])
                .unwrap()
        )
    }
}