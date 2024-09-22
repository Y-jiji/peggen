use std::fmt::Debug;
use peggen::*;


#[derive(Debug, ParseImpl, Space, Num, EnumAstImpl)]
pub enum Json {
    #[rule(r"null")]
    Null,
    #[rule(r"{0:`false|true`}")]
    Bool(bool),
    #[rule(r"{0:`-?(0|[1-9][0-9]*)\.([0-9]+)`}")]
    Flt(f32),
    #[rule("{0:`0|-?[1-9][0-9]*`}")]
    Num(i32),
    #[rule(r#""{0:`[^"]*`}""#)]
    Str(String),
    #[rule(r#"\{ [*0: "{0:`[^"]*`}" : {1} , ][?0: "{0:`[^"]*`}" : {1} ] \}"#)]
    Obj(RVec<(String, Json)>),
    #[rule(r"\[ [*0: {0} , ][?0: {0} ] \]")]
    Arr(RVec<Json>)
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
        // 4782390 ns/iter
        // 867913 ns/iter
        let x = std::time::SystemTime::now();
        for i in 0..10000 { json() };
        println!("{}", x.elapsed().unwrap().as_nanos() / 10000);
    }

}