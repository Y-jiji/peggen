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
        let json = include_str!("../samples/sample.json");
        let json = Parser::<Json>::parse(json).unwrap();
    }

    #[test]
    fn json_bench() {
        let x = std::time::SystemTime::now();
        for i in 0..100 { json() };
        println!("peggen: {}", x.elapsed().unwrap().as_nanos() / 10000);
    }
}
