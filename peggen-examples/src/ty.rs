use std::fmt::Debug;
use peggen::*;

#[derive(Debug, ParseImpl, Space, Num, EnumAstImpl)]
pub enum Ty {
    #[rule(r"{0}")]
    Symbol(Id),
    #[rule(r"int")]
    Int{},
    #[rule(r"\{[0: {0} : {1} ][*0: , {0} : {1} ]\}")]
    Struct(RVec<(Id, Ty)>),
}

#[derive(Debug, ParseImpl, Space, Num, EnumAstImpl)]
#[rule("{0:`[A-Za-z]+`!`int`}")]
pub struct Id(String);

#[cfg(test)]
mod test {
    use peggen::*;
    use super::*;

    #[test]
    fn ty() {
        let ty = Parser::<Ty>::parse("{x: integer}").unwrap();
        println!("{ty:?}");
    }
}