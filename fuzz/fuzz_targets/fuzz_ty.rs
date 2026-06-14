#![no_main]
use libfuzzer_sys::fuzz_target;
use peggen::*;

#[derive(Debug, Parse)]
#[regex(_ = r"\s*")]
#[subrule(field = $0 _ ":" _ $1)]
pub enum Ty {
    #[rule($0)]
    Symbol(Id),
    #[rule("int")]
    Int {},
    #[rule("{" _ $0:field *% (_ "," _) _ "}")]
    Struct(Vec<(Id, Ty)>),
}

#[derive(Debug, Parse)]
#[regex(kw = r"int\b")]
#[regex(alpha = r"[A-Za-z]+")]
#[rule(!kw $0:alpha)]
pub struct Id(String);

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let mut parser = Parser::<Ty>::new();
        let opt = parser.parse(input);
        let reff = parser.ref_parse(input);
        assert_eq!(opt.is_ok(), reff.is_ok());
    }
});
