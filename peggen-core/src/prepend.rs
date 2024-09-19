use crate::*;

pub trait Prepend<Extra: Copy> {
    type Item;
    fn empty(with: Extra) -> Self;
    fn prepend(&mut self, value: Self::Item, with: Extra);
}

impl<T, Extra: Copy> Prepend<Extra> for Option<T> {
    type Item = T;
    fn empty(_: Extra) -> Self {
        Self::None
    }
    fn prepend(&mut self, value: Self::Item, _: Extra) {
        *self = Some(value)
    }
}

impl<T, Extra: Copy> AstImpl<Extra> for Option<T> where
    Self: Prepend<Extra>,
    <Self as Prepend<Extra>>::Item: AstImpl<Extra>
{
    fn ast<'lifetime>(
        input: &'lifetime str, 
        stack: &'lifetime [Tag], 
        with: Extra
    ) -> (&'lifetime [Tag], Self) {
        let tag = &stack[stack.len()-1];
        let mut stack = &stack[..stack.len()-1];
        let mut this = <Self as Prepend<Extra>>::empty(with);
        for _ in 0..tag.rule {
            let (stack_, value) = <<Self as Prepend<Extra>>::Item as AstImpl<Extra>>::ast(input, stack, with);
            this.prepend(value, with);
            stack = stack_;
        }
        (stack, this)
    }
}