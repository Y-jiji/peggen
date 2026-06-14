use crate::*;
use bumpalo::boxed::Box as BBox;
use bumpalo::collections::String as BString;

#[derive(Debug, ParseImpl, Num, EnumAstImpl)]
#[with(&'a bumpalo::Bump)]
#[regex(_ = r"\s*")]
#[regex(id = r"[a-z0-9]")]
pub enum Expr<'a> {
    #[tag(add)]
    #[rule($0@add _ "+" _ $1@mul)]
    Add(BBox<'a, Expr<'a>>, BBox<'a, Expr<'a>>),
    #[tag(add)]
    #[rule($0@add _ "-" _ $1@mul)]
    Sub(BBox<'a, Expr<'a>>, BBox<'a, Expr<'a>>),
    #[tag(add, mul)]
    #[rule($0@mul _ "*" _ $1@atom)]
    Mul(BBox<'a, Expr<'a>>, BBox<'a, Expr<'a>>),
    #[tag(add, mul)]
    #[rule($0@mul _ "/" _ $1@atom)]
    Div(BBox<'a, Expr<'a>>, BBox<'a, Expr<'a>>),
    #[tag(add, mul, atom)]
    #[rule($0:id)]
    Ident(BString<'a>),
    #[tag(add, mul, atom)]
    #[rule("(" _ $0@add _ ")")]
    Scope(BBox<'a, Expr<'a>>),
}

#[cfg(test)]
mod test {
    use super::*;
    use bumpalo::Bump;
    use peggen::Parser;

    #[test]
    fn expr() {
        let bump = Bump::new();
        let mut parser = Parser::<Expr>::new();
        let expr = parser.parse_with("1 + 2 * a - 4", &bump).unwrap();
        println!("{expr:?}");
    }

    #[test]
    fn expr_bench() {
        let x = std::time::SystemTime::now();
        let mut parser = Parser::<Expr>::new();
        let input = "1 + 2 * a - 4 + (3 * b) / c + d - e * f";
        for _ in 0..10_000 {
            let bump = Bump::new();
            let _ = parser.parse_with(input, &bump).unwrap();
        }
        println!("peggen expr: {}", x.elapsed().unwrap().as_nanos() / 10000);
    }

    #[test]
    fn expr_bench_ref() {
        let x = std::time::SystemTime::now();
        let mut parser = Parser::<Expr>::new();
        let input = "1 + 2 * a - 4 + (3 * b) / c + d - e * f";
        for _ in 0..10_000 {
            let bump = Bump::new();
            let _ = parser.ref_parse_with(input, &bump).unwrap();
        }
        println!("peggen expr ref: {}", x.elapsed().unwrap().as_nanos() / 10000);
    }

    fn gen_expr(rng: &mut impl rand::Rng, depth: usize) -> String {
        if depth > 5 {
            let atoms = b"0123456789abcdefghijklmnopqrstuvwxyz";
            return format!("{}", atoms[rng.gen_range(0..atoms.len())] as char);
        }
        match rng.gen_range(0..6) {
            0..=1 => {
                let atoms = b"0123456789abcdefghijklmnopqrstuvwxyz";
                format!("{}", atoms[rng.gen_range(0..atoms.len())] as char)
            }
            2 => format!("{} + {}", gen_expr(rng, depth + 1), gen_expr(rng, depth + 1)),
            3 => format!("{} - {}", gen_expr(rng, depth + 1), gen_expr(rng, depth + 1)),
            4 => format!("{} * {}", gen_expr(rng, depth + 1), gen_expr(rng, depth + 1)),
            _ => format!("({})", gen_expr(rng, depth + 1)),
        }
    }

    #[test]
    fn fuzz_expr_valid_inputs() {
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(77);
        let mut parser = Parser::<Expr>::new();
        for i in 0..10_000 {
            let input = gen_expr(&mut rng, 0);
            let bump = Bump::new();
            let result = parser.parse_with(&input, &bump);
            assert!(result.is_ok(), "failed on iteration {i}, input: {input:?}");
        }
    }

    #[test]
    fn fuzz_expr_no_panic_on_garbage() {
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(200);
        let mut parser = Parser::<Expr>::new();
        for _ in 0..10_000 {
            let len = rng.gen_range(0..64);
            let bytes: Vec<u8> = (0..len).map(|_| rng.gen_range(0x20..0x7f)).collect();
            let input = String::from_utf8(bytes).unwrap();
            let bump = Bump::new();
            let _ = parser.parse_with(&input, &bump);
        }
    }

    #[test]
    fn fuzz_expr_edge_cases() {
        let cases = [
            "", " ", "a", "0", "(a)", "((a))",
            "a + b", "a + b + c", "a + b * c",
            "a * b + c * d", "(a + b) * (c - d)",
            "(((((a)))))", "a+b", "a +b", "a+ b",
            "+", "*", "(", ")", "()", "a +",
            "+ a", "a b", "a + + b", "a * * b",
            "(a + b", "a + b)",
        ];
        let mut parser = Parser::<Expr>::new();
        for input in cases {
            let bump = Bump::new();
            let _ = parser.parse_with(input, &bump);
        }
    }

    #[test]
    fn fuzz_expr_differential_valid() {
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(900);
        let mut parser = Parser::<Expr>::new();
        for i in 0..10_000 {
            let input = gen_expr(&mut rng, 0);
            let bump = Bump::new();
            let opt = parser.parse_with(&input, &bump);
            let bump2 = Bump::new();
            let reff = parser.ref_parse_with(&input, &bump2);
            assert_eq!(opt.is_ok(), reff.is_ok(),
                "differential divergence on iteration {i}, input: {input:?}");
        }
    }

    #[test]
    fn expr_fused() {
        let bump = Bump::new();
        let mut parser = Parser::<Expr>::new();
        let expr: Expr = parser.fused_parse_with("1 + 2 * a - 4", &bump).unwrap();
        println!("{expr:?}");
    }

    #[test]
    fn expr_bench_fused() {
        let x = std::time::SystemTime::now();
        let mut parser = Parser::<Expr>::new();
        let input = "1 + 2 * a - 4 + (3 * b) / c + d - e * f";
        for _ in 0..10_000 {
            let bump = Bump::new();
            let _: Expr = parser.fused_parse_with(input, &bump).unwrap();
        }
        println!("peggen expr fused: {}", x.elapsed().unwrap().as_nanos() / 10000);
    }

    #[test]
    fn fuzz_expr_fused_differential_valid() {
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(1100);
        let mut parser = Parser::<Expr>::new();
        for i in 0..10_000 {
            let input = gen_expr(&mut rng, 0);
            let bump = Bump::new();
            let opt = parser.parse_with(&input, &bump);
            let bump2 = Bump::new();
            let fused: Result<Expr, ()> = parser.fused_parse_with(&input, &bump2);
            assert_eq!(opt.is_ok(), fused.is_ok(),
                "fused differential divergence on iteration {i}, input: {input:?}");
        }
    }

    #[test]
    fn fuzz_expr_fused_differential_garbage() {
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(1200);
        let mut parser = Parser::<Expr>::new();
        for _ in 0..10_000 {
            let len = rng.gen_range(0..64);
            let bytes: Vec<u8> = (0..len).map(|_| rng.gen_range(0x20..0x7f)).collect();
            let input = String::from_utf8(bytes).unwrap();
            let bump = Bump::new();
            let opt = parser.parse_with(&input, &bump);
            let bump2 = Bump::new();
            let fused: Result<Expr, ()> = parser.fused_parse_with(&input, &bump2);
            assert_eq!(opt.is_ok(), fused.is_ok(),
                "fused differential divergence on: {input:?}");
        }
    }

    #[test]
    fn fuzz_expr_differential_garbage() {
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(1000);
        let mut parser = Parser::<Expr>::new();
        for _ in 0..10_000 {
            let len = rng.gen_range(0..64);
            let bytes: Vec<u8> = (0..len).map(|_| rng.gen_range(0x20..0x7f)).collect();
            let input = String::from_utf8(bytes).unwrap();
            let bump = Bump::new();
            let opt = parser.parse_with(&input, &bump);
            let bump2 = Bump::new();
            let reff = parser.ref_parse_with(&input, &bump2);
            assert_eq!(opt.is_ok(), reff.is_ok(),
                "differential divergence on: {input:?}");
        }
    }
}
