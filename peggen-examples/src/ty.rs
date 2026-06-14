use std::fmt::Debug;
use peggen::*;

#[derive(Debug, ParseImpl, Num, EnumAstImpl)]
#[regex(_ = r"\s*")]
#[subrule(field = $0 _ ":" _ $1)]
pub enum Ty {
    #[rule($0)]
    Symbol(Id),
    #[rule("int")]
    Int{},
    #[rule("{" _ $0:field *% (_ "," _) _ "}")]
    Struct(Vec<(Id, Ty)>),
}

#[derive(Debug, ParseImpl, Num, EnumAstImpl)]
#[regex(kw = r"int\b")]
#[regex(alpha = r"[A-Za-z]+")]
#[rule(!kw $0:alpha)]
pub struct Id(String);

#[cfg(test)]
mod test {
    use peggen::*;
    use super::*;

    #[test]
    fn ty() {
        let ty = Parser::<Ty>::parse("{x: integer, y: int}").unwrap();
        println!("{ty:?}");
    }
}
