use std::fmt::Debug;
use pigeon::*;

#[derive(Debug, Num, ParseImpl, EnumAstImpl, Space)]
pub enum Json {
    #[rule("{0:`false|true`}")]
    Bool(bool),
    #[rule("{0:`0|-?[1-9][0-9]*`}")]
    Int(i64),
    #[rule(r"\{ [*0: {0:`[a-zA-Z]+`} : {1} , ][?0: {0:`[a-zA-Z]+`} : {1} ] \}")]
    Obj(RVec<(String, Json)>),
    #[rule(r"\[ [*0: {0} , ][?0: {0} ] \]")]
    Arr(RVec<Json>),
}

#[cfg(test)]
mod test {
    use pigeon::*;
    use super::*;

    #[test]
    fn json() {
        let json = r"{x: 1, y: [2, 3], z: false}";
        let json = Parser::<Json>::parse(json).unwrap();
        println!("{json:?}");
    }
}