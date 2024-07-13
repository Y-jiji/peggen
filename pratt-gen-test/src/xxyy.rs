use pratt_gen::*;

// A -> xA | xB | x
// B -> yA | xA | y | A
// S -> A

#[derive(Debug, ParserImpl, Space, Clone, Copy)]
pub enum A<'a> {
    #[parse("x{0}")]
    XA(&'a A<'a>),
    #[parse("x{0}")]
    XB(&'a B<'a>),
    #[parse("x")]
    X(),
}

#[derive(Debug, ParserImpl, Space, Clone, Copy)]
pub enum B<'a> {
    #[parse("y{0}")]
    YA(&'a A<'a>),
    #[parse("{0}x")]
    AX(&'a A<'a>),
    #[parse("y")]
    Y(),
    #[parse("{0}")]
    A(&'a A<'a>),
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn xxyy() {
        // 318979 characters/sec
        // 480769 characters/sec
        let source = (0..2000000*40).map(|x| if x % 998224353 % 2 == 0 { 'x' } else { 'y' }).collect::<String>();
        println!("here");
        let source = Source::new(&source);
        let out_arena = Arena::new();
        let err_arena = Arena::new();
        let result: Result<A, Error> = parse::<A<'_>>(source, &out_arena, &err_arena);
        result.unwrap();
    }
}