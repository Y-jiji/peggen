use crate::*;

type Ref<'a, T> = &'a T;

impl<'a, X, Y> Map<'a, X> for &'a Y where
    Y: Map<'a, X> + Sized,
    X: Sized
{
    fn map(
        input: &'a str, 
        begin: usize,
        arena: &'a Arena,
        value: X,
        end: usize,
    ) -> Self {
        let value = Y::map(input, begin, arena, value, end);
        arena.alloc_val(value)
    }
}

DeriveParseImpl!{Ref}
