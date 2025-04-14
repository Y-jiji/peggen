use crate::*;
use bumpalo::Bump;

pub trait PushImpl<Extra: Copy>: Sized {
    type Item: AstImpl<Extra>;
    fn empty(with: Extra) -> Self;
    fn push(&mut self, value: Self::Item, with: Extra);
    fn peggen_ast<'lifetime>(
        input: &'lifetime str, 
        stack: &'lifetime [Tag], 
        with: Extra
    ) -> (&'lifetime [Tag], Self) {
        let tag = &stack[stack.len()-1];
        let stack = &stack[..stack.len()-1];
        fn rev<'lifetime, Extra: Copy, T: PushImpl<Extra>>(
            depth: usize,
            input: &'lifetime str,
            stack: &'lifetime [Tag],
            with: Extra,
        ) -> (&'lifetime [Tag], T)
        where
            T: PushImpl<Extra>,
            <T as PushImpl<Extra>>::Item: AstImpl<Extra>,
        {
            if depth == 0 {
                (stack, <T as PushImpl<Extra>>::empty(with))
            } else {
                let (stack, val) = <<T as PushImpl<
                    Extra,
                >>::Item as AstImpl<Extra>>::peggen_ast(input, stack, with);
                let (stack, mut seq) = rev::<Extra, T>(depth - 1, input, stack, with);
                seq.push(val, with);
                return (stack, seq);
            }
        }
        rev::<Extra, Self>(tag.rule, input, stack, with)
    }
}

impl<T: AstImpl<Extra>, Extra: Copy> PushImpl<Extra> for Option<T> {
    type Item = T;
    fn empty(_: Extra) -> Self {
        Self::None
    }
    fn push(&mut self, value: Self::Item, _: Extra) {
        *self = Some(value)
    }
}

#[cfg(feature="bumpalo")]
impl<'b, T: AstImpl<&'b Bump>> PushImpl<&'b Bump> for bumpalo::collections::Vec<'b, T> {
    type Item = T;
    fn empty(with: &'b Bump) -> Self {
        bumpalo::collections::Vec::new_in(with)
    }
    fn push(&mut self, value: Self::Item, _: &'b Bump) {
        bumpalo::collections::Vec::<T>::push(self, value);
    }
}

impl<T: AstImpl<Extra>, Extra: Copy> PushImpl<Extra> for Vec<T> {
    type Item = T;
    fn empty(_: Extra) -> Self {
        Vec::new()
    }
    fn push(&mut self, value: Self::Item, _: Extra) {
        Vec::<T>::push(self, value);
    }
}
