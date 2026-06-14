#![no_main]
use libfuzzer_sys::fuzz_target;
use peggen::*;

#[derive(Debug, Parse)]
#[regex(_ = r"\s*")]
#[regex(id = r"[a-z0-9]")]
pub enum Expr {
    #[tag(add)]
    #[rule($0@add _ "+" _ $1@mul)]
    Add(Box<Expr>, Box<Expr>),
    #[tag(add)]
    #[rule($0@add _ "-" _ $1@mul)]
    Sub(Box<Expr>, Box<Expr>),
    #[tag(add, mul)]
    #[rule($0@mul _ "*" _ $1@atom)]
    Mul(Box<Expr>, Box<Expr>),
    #[tag(add, mul)]
    #[rule($0@mul _ "/" _ $1@atom)]
    Div(Box<Expr>, Box<Expr>),
    #[tag(add, mul, atom)]
    #[rule($0:id)]
    Ident(String),
    #[tag(add, mul, atom)]
    #[rule("(" _ $0@add _ ")")]
    Scope(Box<Expr>),
}

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let mut parser = Parser::<Expr>::new();
        let opt = parser.parse(input);
        let reff = parser.ref_parse(input);
        assert_eq!(opt.is_ok(), reff.is_ok());
    }
});
