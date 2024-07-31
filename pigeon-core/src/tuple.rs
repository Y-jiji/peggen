use crate::*;

macro_rules! Impl {
    ($($($T: ident)*;)*) => {$(
        impl<Extra, $($T, )*> AstImpl<Extra> for ($($T, )*)
            where $($T: AstImpl<Extra>, )*
        {
            fn ast<'a>(
                input: &'a str, 
                stack: &'a [Tag], 
                extra: &'a Extra
            ) -> (&'a [Tag], Self) {
                $(
                    let (stack, casey::lower!($T)) = $T::ast(input, stack, extra);
                )*
                (stack, ($(casey::lower!($T), )*))
            }
        }
    )*};
}

Impl!(
    A;
    A B;
    A B C;
    A B C D;
    A B C D E;
    A B C D E F;
    A B C D E F G;
);