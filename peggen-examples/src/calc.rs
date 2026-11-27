use peggen::{peggen, Message};

/// The README calculator: `+` and `-` bind looser than `*` and `/`; all four associate to the left.
#[peggen(space = r"\s*")]
#[derive(Debug, Clone, PartialEq)]
pub enum CalcEx {
    #[parse(
        _0 = (CalcEx@{Add, Sub, Mul, Div, Paren, Num} ; Box::new) "+" %
        _1 = (CalcEx@{Mul, Div, Paren, Num} ; Box::new)
    )]
    Add(Box<CalcEx>, Box<CalcEx>),
    #[parse(
        _0 = (CalcEx@{Add, Sub, Mul, Div, Paren, Num} ; Box::new) "-" %
        _1 = (CalcEx@{Mul, Div, Paren, Num} ; Box::new)
    )]
    Sub(Box<CalcEx>, Box<CalcEx>),
    #[parse(
        _0 = (CalcEx@{Mul, Div, Paren, Num} ; Box::new) "*" %
        _1 = (CalcEx@{Paren, Num} ; Box::new)
    )]
    Mul(Box<CalcEx>, Box<CalcEx>),
    #[parse(
        _0 = (CalcEx@{Mul, Div, Paren, Num} ; Box::new) "/" %
        _1 = (CalcEx@{Paren, Num} ; Box::new)
    )]
    Div(Box<CalcEx>, Box<CalcEx>),
    #[parse("(" % _0 = (CalcEx ; Box::new) ")")]
    Paren(Box<CalcEx>),
    #[error("unclosed parenthesis" : "(" r"[^)]*")]
    Unclosed,
    #[parse(_0 = (r"[0-9]+" ; |x: &str| x.parse::<usize>().unwrap()))]
    Num(usize),
}

/// Parses an arithmetic expression. On any failure, returns every message: each unclosed parenthesis
/// that was skipped, and the one that stopped parsing, if any.
pub fn parse(input: &str) -> Result<CalcEx, Vec<Message<'_>>> {
    match peggen::parse::<CalcEx, ()>(input, &mut ()) {
        Ok((tree, messages)) if messages.is_empty() => Ok(tree),
        Ok((_, messages)) | Err(messages) => Err(messages),
    }
}

#[cfg(test)]
mod correct {
    use super::*;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    /// `1+2*3`, `1*2+3`, `1-2-3`, `1+2-3`, `(1+2)*3` and ` 1 * ( 2 + 3 ) / 4 ` give the precedence and left-associative
    /// trees; `1+`, `*1`, `1 2`, `(1` and `` fail.
    #[test]
    fn precedence() {
        use CalcEx::*;
        let num = |value| Box::new(Num(value));
        let cases = [
            ("1+2*3", Add(num(1), Box::new(Mul(num(2), num(3))))),
            ("1*2+3", Add(Box::new(Mul(num(1), num(2))), num(3))),
            ("1-2-3", Sub(Box::new(Sub(num(1), num(2))), num(3))),
            ("1+2-3", Sub(Box::new(Add(num(1), num(2))), num(3))),
            ("(1+2)*3", Mul(Box::new(Paren(Box::new(Add(num(1), num(2))))), num(3))),
            (" 1 * ( 2 + 3 ) / 4 ", Div(Box::new(Mul(num(1), Box::new(Paren(Box::new(Add(num(2), num(3))))))), num(4))),
        ];
        for (input, tree) in cases {
            assert_eq!(parse(input).ok(), Some(tree), "{input:?}");
        }
        for input in ["1+", "*1", "1 2", "(1", ""] {
            assert!(parse(input).is_err(), "{input:?}");
        }
    }

    /// `1*(2` recovers to `Mul(Num(1), Unclosed)`, with "unclosed parenthesis" at 1:3.
    #[test]
    fn recovery() {
        let input = "1*(2";
        let (tree, messages) = peggen::parse::<CalcEx, ()>(input, &mut ()).expect("recovers");
        assert_eq!(tree, CalcEx::Mul(Box::new(CalcEx::Num(1)), Box::new(CalcEx::Unclosed)));
        let found: Vec<_> = messages.iter().map(|message| (message.locate(input), message.text)).collect();
        assert_eq!(found, [((1, 3), "unclosed parenthesis")]);
    }

    /// 2000 pseudo-random trees, printed with random spaces, parse back equal.
    #[test]
    fn roundtrip() {
        let mut rng = StdRng::seed_from_u64(0x5EED_CA1C);
        for _ in 0..2000 {
            let tree = random(&mut rng, 0, 5);
            let mut text = String::from(" ");
            print(&tree, &mut rng, &mut text);
            text.push('\n');
            assert_eq!(parse(&text).ok(), Some(tree), "{text:?}");
        }
    }

    /// A pseudo-random tree at `level` (0: any variant; 1: `Mul`, `Div`, `Paren`, `Num`; 2: `Paren`, `Num`),
    /// nested at most `depth` deep, so that printing it without extra parentheses keeps its shape.
    fn random(rng: &mut StdRng, level: u32, depth: u32) -> CalcEx {
        if depth == 0 {
            return CalcEx::Num(rng.random_range(0..1000));
        }
        let deeper = |rng: &mut StdRng, level| Box::new(random(rng, level, depth - 1));
        match rng.random_range(2 * level..6) {
            0 => CalcEx::Add(deeper(rng, 0), deeper(rng, 1)),
            1 => CalcEx::Sub(deeper(rng, 0), deeper(rng, 1)),
            2 => CalcEx::Mul(deeper(rng, 1), deeper(rng, 2)),
            3 => CalcEx::Div(deeper(rng, 1), deeper(rng, 2)),
            4 => CalcEx::Paren(deeper(rng, 0)),
            _ => CalcEx::Num(rng.random_range(0..1000)),
        }
    }

    /// Writes `tree` with random spaces; parentheses only where its `Paren` nodes are.
    fn print(tree: &CalcEx, rng: &mut StdRng, out: &mut String) {
        let space = |rng: &mut StdRng, out: &mut String| out.push_str([" ", "", "\t", "  "][rng.random_range(0..4)]);
        let (left, op, right) = match tree {
            CalcEx::Add(left, right) => (left, '+', right),
            CalcEx::Sub(left, right) => (left, '-', right),
            CalcEx::Mul(left, right) => (left, '*', right),
            CalcEx::Div(left, right) => (left, '/', right),
            CalcEx::Paren(inner) => {
                out.push('(');
                space(rng, out);
                print(inner, rng, out);
                space(rng, out);
                out.push(')');
                return;
            }
            CalcEx::Num(value) => return out.push_str(&value.to_string()),
            CalcEx::Unclosed => unreachable!("`random` never builds error variants"),
        };
        print(left, rng, out);
        space(rng, out);
        out.push(op);
        space(rng, out);
        print(right, rng, out);
    }
}
