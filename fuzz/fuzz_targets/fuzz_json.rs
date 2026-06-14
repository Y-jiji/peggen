#![no_main]
use libfuzzer_sys::fuzz_target;
use peggen::*;

#[derive(Debug, Parse)]
#[regex(_ = r"\s*")]
#[regex(num = r"0|-?[1-9][0-9]*")]
#[regex(flt = r"-?(0|[1-9][0-9]*)\.([0-9]+)")]
#[regex(str = r#"[^"]*"#)]
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

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let mut parser = Parser::<Json>::new();
        let opt = parser.parse(input);
        let reff = parser.ref_parse(input);
        assert_eq!(opt.is_ok(), reff.is_ok());
    }
});
