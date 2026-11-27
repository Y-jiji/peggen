/// Defines one `callN` per line, each running `func` on its N arguments.
/// Generated code hands over a closure and its arguments in one call,
/// so the closure's parameter types come from the arguments.
macro_rules! calls {
    ($($(#[$tag:meta])* $name:ident($($arg:ident: $ty:ident),*);)*) => {$(
        /// Runs `func` on the arguments that follow it.
        $(#[$tag])*
        pub fn $name<$($ty,)* R>(func: impl FnOnce($($ty),*) -> R, $($arg: $ty),*) -> R {
            func($($arg),*)
        }
    )*};
}

calls! {
    call0();
    call1(a: A);
    call2(a: A, b: B);
    call3(a: A, b: B, c: C);
    call4(a: A, b: B, c: C, d: D);
    call5(a: A, b: B, c: C, d: D, e: E);
    #[doc = "SADDLER: SHAME(MANYARG)"]
    call6(a: A, b: B, c: C, d: D, e: E, f: F);
    #[doc = "SADDLER: SHAME(MANYARG)"]
    call7(a: A, b: B, c: C, d: D, e: E, f: F, g: G);
    #[doc = "SADDLER: SHAME(MANYARG)"]
    call8(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H);
    #[doc = "SADDLER: SHAME(MANYARG)"]
    call9(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I);
    #[doc = "SADDLER: SHAME(MANYARG)"]
    call10(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J);
    #[doc = "SADDLER: SHAME(MANYARG)"]
    call11(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J, k: K);
    #[doc = "SADDLER: SHAME(MANYARG)"]
    call12(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J, k: K, l: L);
}
