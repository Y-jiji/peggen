use crate::*;

macro_rules! Impl {
    ($($($A: ident)*, $($B: ident)*;)*) => {$(
        impl<Extra, $($A, )*> AstImpl<Extra> for ($($A, )*)
            where $($A: AstImpl<Extra>, )*
        {
            fn ast<'a>(
                input: &'a str, 
                stack: &'a [Tag], 
                extra: &'a Extra
            ) -> (&'a [Tag], Self) {
                $(
                    // Because tag code is suffix coding, we have to parse from tail to head
                    let (stack, casey::lower!($B)) = $B::ast(input, stack, extra);
                )*
                (stack, ($(casey::lower!($A), )*))
            }
        }
    )*};
}

Impl!(
    A, A;
    A B, B A;
    A B C, C B A;
    A B C D, D C B A;
    A B C D E, E D C B A;
    A B C D E F, F E D C B A;
    A B C D E F G, G F E D C B A;
);