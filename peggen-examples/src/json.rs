use std::fmt::Debug;
use peggen::*;

#[derive(Debug, Parse)]
#[regex(_    = r"\s*")]
#[regex(num  = r"0|-?[1-9][0-9]*")]
#[regex(flt  = r"-?(0|[1-9][0-9]*)\.([0-9]+)")]
#[regex(str  = r#"[^"]*"#)]
#[regex(bool = r"false|true")]
#[subrule(kv = "\"" $0:str "\"" _ ":" _ $1)]
pub enum Json {
    #[rule("null")]
    Null,
    #[rule($0:bool)]
    Bool(bool),
    #[rule($0:flt)]
    Flt(f32),
    #[rule($0:num)]
    Num(i32),
    #[rule("\"" $0:str "\"")]
    Str(String),
    #[rule("{" _ $0:kv *% (_ "," _) _ "}")]
    Obj(Vec<(String, Json)>),
    #[rule("[" _ $0 *% (_ "," _) _ "]")]
    Arr(Vec<Json>),
}

#[cfg(test)]
mod test {
    use peggen::*;
    use super::*;

    #[test]
    fn json() {
        let json_str = include_str!("../samples/sample.json");
        let mut parser = Parser::<Json>::new();
        let _json = parser.parse(json_str).unwrap();
    }

    #[test]
    fn json_bench() {
        let x = std::time::SystemTime::now();
        let mut parser = Parser::<Json>::new();
        let json_str = include_str!("../samples/sample.json");
        for _ in 0..100 {
            let _: Json = parser.parse(json_str).unwrap();
        }
        println!("peggen: {}", x.elapsed().unwrap().as_nanos() / 10000);
    }

    #[test]
    fn json_bench_ref() {
        let x = std::time::SystemTime::now();
        let mut parser = Parser::<Json>::new();
        let json_str = include_str!("../samples/sample.json");
        for _ in 0..100 {
            let _: Json = parser.ref_parse(json_str).unwrap();
        }
        println!("peggen ref: {}", x.elapsed().unwrap().as_nanos() / 10000);
    }

    #[test]
    fn json_fused() {
        let json_str = include_str!("../samples/sample.json");
        let mut parser = Parser::<Json>::new();
        let _json: Json = parser.fused_parse(json_str).unwrap();
    }

    #[test]
    fn json_bench_fused() {
        let x = std::time::SystemTime::now();
        let mut parser = Parser::<Json>::new();
        let json_str = include_str!("../samples/sample.json");
        for _ in 0..100 {
            let _: Json = parser.fused_parse(json_str).unwrap();
        }
        println!("peggen fused: {}", x.elapsed().unwrap().as_nanos() / 10000);
    }

    fn gen_json(rng: &mut impl rand::Rng, depth: usize) -> String {
        if depth > 6 {
            return match rng.gen_range(0..4) {
                0 => "null".into(),
                1 => format!("{}", rng.gen_bool(0.5)),
                2 => format!("{}", rng.gen_range(-999i32..999)),
                _ => format!("\"{}\"", gen_safe_str(rng)),
            };
        }
        match rng.gen_range(0..7) {
            0 => "null".into(),
            1 => format!("{}", rng.gen_bool(0.5)),
            2 => {
                let n = rng.gen_range(-999i32..999);
                if n == 0 { "0".into() }
                else { format!("{}", n) }
            }
            3 => {
                let i = rng.gen_range(-99i32..99);
                let f = rng.gen_range(0u32..999);
                if i == 0 { format!("0.{:0>1}", f) }
                else { format!("{}.{:0>1}", i, f) }
            }
            4 => format!("\"{}\"", gen_safe_str(rng)),
            5 => {
                let n = rng.gen_range(0..4);
                let items: Vec<_> = (0..n).map(|_| gen_json(rng, depth + 1)).collect();
                format!("[{}]", items.join(", "))
            }
            _ => {
                let n = rng.gen_range(0..4);
                let pairs: Vec<_> = (0..n)
                    .map(|_| format!("\"{}\": {}", gen_safe_str(rng), gen_json(rng, depth + 1)))
                    .collect();
                format!("{{{}}}", pairs.join(", "))
            }
        }
    }

    fn gen_safe_str(rng: &mut impl rand::Rng) -> String {
        let len = rng.gen_range(0..8);
        let chars = b"abcdefghijklmnopqrstuvwxyz0123456789 _-.";
        (0..len).map(|_| chars[rng.gen_range(0..chars.len())] as char).collect()
    }

    #[test]
    fn fuzz_json_valid_inputs() {
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let mut parser = Parser::<Json>::new();
        for i in 0..10_000 {
            let input = gen_json(&mut rng, 0);
            let result = parser.parse(&input);
            assert!(result.is_ok(), "failed on iteration {i}, input: {input:?}");
        }
    }

    #[test]
    fn fuzz_json_no_panic_on_garbage() {
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(123);
        let mut parser = Parser::<Json>::new();
        for _ in 0..10_000 {
            let len = rng.gen_range(0..64);
            let bytes: Vec<u8> = (0..len).map(|_| rng.gen_range(0x20..0x7f)).collect();
            let input = String::from_utf8(bytes).unwrap();
            let _ = parser.parse(&input);
        }
    }

    #[test]
    fn fuzz_json_edge_cases() {
        let mut parser = Parser::<Json>::new();
        let cases = [
            "", " ", "  null  ", "[", "]", "{", "}", ",", ":",
            "[[[[[[]]]]]]", "{\"a\":{\"b\":{\"c\":null}}}",
            "[1,2,3,4,5,6,7,8,9,10]",
            "\"\"", "0", "-1", "0.0", "-0.0",
            "[null, true, false, 1, \"x\"]",
            "{\"\":null}", "nul", "tru", "fals",
            "[,]", "{,}", "[1,]", "{\"a\":1,}",
            "nullnull", "[][]",
        ];
        for input in cases {
            let _ = parser.parse(input);
        }
    }

    #[test]
    fn fuzz_json_parser_reuse() {
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(99);
        let mut parser = Parser::<Json>::new();
        for _ in 0..1000 {
            let input = gen_json(&mut rng, 0);
            let r1 = parser.parse(&input);
            let r2 = parser.parse(&input);
            assert_eq!(r1.is_ok(), r2.is_ok(), "parser reuse divergence on: {input:?}");
        }
    }

    #[test]
    fn fuzz_json_differential_valid() {
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(500);
        let mut parser = Parser::<Json>::new();
        for i in 0..10_000 {
            let input = gen_json(&mut rng, 0);
            let opt = parser.parse(&input);
            let reff = parser.ref_parse(&input);
            assert_eq!(opt.is_ok(), reff.is_ok(),
                "differential divergence on iteration {i}, input: {input:?}");
        }
    }

    #[test]
    fn fuzz_json_differential_garbage() {
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(600);
        let mut parser = Parser::<Json>::new();
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

    #[test]
    fn fuzz_json_fused_differential_valid() {
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(700);
        let mut parser = Parser::<Json>::new();
        for i in 0..10_000 {
            let input = gen_json(&mut rng, 0);
            let opt = parser.parse(&input);
            let fused: Result<Json, ()> = parser.fused_parse(&input);
            assert_eq!(opt.is_ok(), fused.is_ok(),
                "fused differential divergence on iteration {i}, input: {input:?}");
        }
    }

    #[test]
    fn fuzz_json_fused_differential_garbage() {
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(800);
        let mut parser = Parser::<Json>::new();
        for _ in 0..10_000 {
            let len = rng.gen_range(0..64);
            let bytes: Vec<u8> = (0..len).map(|_| rng.gen_range(0x20..0x7f)).collect();
            let input = String::from_utf8(bytes).unwrap();
            let opt = parser.parse(&input);
            let fused: Result<Json, ()> = parser.fused_parse(&input);
            assert_eq!(opt.is_ok(), fused.is_ok(),
                "fused differential divergence on: {input:?}");
        }
    }
}
