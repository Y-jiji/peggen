use std::fmt::Debug;
use peggen::*;

#[derive(Debug, ParseImpl, Num, EnumAstImpl)]
#[regex(_ = r"\s*")]
#[subrule(field = $0 _ ":" _ $1)]
pub enum Ty {
    #[rule($0)]
    Symbol(Id),
    #[rule("int")]
    Int{},
    #[rule("{" _ $0:field *% (_ "," _) _ "}")]
    Struct(Vec<(Id, Ty)>),
}

#[derive(Debug, ParseImpl, Num, EnumAstImpl)]
#[regex(kw = r"int\b")]
#[regex(alpha = r"[A-Za-z]+")]
#[rule(!kw $0:alpha)]
pub struct Id(String);

#[cfg(test)]
mod test {
    use peggen::*;
    use super::*;

    #[test]
    fn ty() {
        let mut parser = Parser::<Ty>::new();
        let ty = parser.parse("{x: integer, y: int}").unwrap();
        println!("{ty:?}");
    }

    fn gen_ident(rng: &mut impl rand::Rng) -> String {
        let keywords = ["int"];
        loop {
            let len = rng.gen_range(1..6);
            let s: String = (0..len).map(|_| (b'a' + rng.gen_range(0..26)) as char).collect();
            if !keywords.contains(&s.as_str()) {
                return s;
            }
        }
    }

    fn gen_ty(rng: &mut impl rand::Rng, depth: usize) -> String {
        if depth > 4 {
            return if rng.gen_bool(0.3) { "int".into() } else { gen_ident(rng) };
        }
        match rng.gen_range(0..3) {
            0 => "int".into(),
            1 => gen_ident(rng),
            _ => {
                let n = rng.gen_range(0..4);
                let fields: Vec<_> = (0..n)
                    .map(|_| format!("{}: {}", gen_ident(rng), gen_ty(rng, depth + 1)))
                    .collect();
                format!("{{{}}}", fields.join(", "))
            }
        }
    }

    #[test]
    fn fuzz_ty_valid_inputs() {
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(55);
        let mut parser = Parser::<Ty>::new();
        for i in 0..10_000 {
            let input = gen_ty(&mut rng, 0);
            let result = parser.parse(&input);
            assert!(result.is_ok(), "failed on iteration {i}, input: {input:?}");
        }
    }

    #[test]
    fn fuzz_ty_no_panic_on_garbage() {
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(300);
        let mut parser = Parser::<Ty>::new();
        for _ in 0..10_000 {
            let len = rng.gen_range(0..64);
            let bytes: Vec<u8> = (0..len).map(|_| rng.gen_range(0x20..0x7f)).collect();
            let input = String::from_utf8(bytes).unwrap();
            let _ = parser.parse(&input);
        }
    }

    #[test]
    fn fuzz_ty_edge_cases() {
        let mut parser = Parser::<Ty>::new();
        let cases = [
            "", " ", "int", "foo", "integer",
            "{}", "{x: int}", "{x: int, y: foo}",
            "{x: {y: int}}", "{x: {y: {z: int}}}",
            "{", "}", "{,}", "{x:}", "{: int}",
            "{x: int,}", "{x: int y: int}",
            "intt", "inta", "inti",
        ];
        for input in cases {
            let _ = parser.parse(input);
        }
    }

    #[test]
    fn fuzz_ty_differential_valid() {
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(700);
        let mut parser = Parser::<Ty>::new();
        for i in 0..10_000 {
            let input = gen_ty(&mut rng, 0);
            let opt = parser.parse(&input);
            let reff = parser.ref_parse(&input);
            assert_eq!(opt.is_ok(), reff.is_ok(),
                "differential divergence on iteration {i}, input: {input:?}");
        }
    }

    #[test]
    fn fuzz_ty_differential_garbage() {
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(800);
        let mut parser = Parser::<Ty>::new();
        for _ in 0..10_000 {
            let len = rng.gen_range(0..64);
            let bytes: Vec<u8> = (0..len).map(|_| rng.gen_range(0x20..0x7f)).collect();
            let input = String::from_utf8(bytes).unwrap();
            let opt = parser.parse(&input);
            let reff = parser.ref_parse(&input);
            assert_eq!(opt.is_ok(), reff.is_ok(),
                "differential divergence on: {input:?}");
        }
    }
}
