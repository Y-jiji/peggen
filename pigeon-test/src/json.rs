use std::fmt::Debug;

use pigeon::{AstImpl, Num, ParseImpl, Prepend, Space};

#[derive(Debug, Num, ParseImpl, AstImpl, Space)]
pub enum Json {
    #[rule("{0:`[0-9]|([1-9][0-9]*)`}")]
    Int(u64),
    #[rule(r"\{ [*0: {0:`[a-zA-Z]+`} : {1} , ][?0: {0:`[a-zA-Z]+`} : {1} ] \}")]
    Obj(RVec<(String, Json)>),
    #[rule(r"\[ [*0: {0} , ][?0: {0} ] \]")]
    Arr(RVec<Json>),
}

#[derive(Prepend)]
pub struct RVec<T>(Vec<T>);

impl<T, Extra> Prepend<Extra> for RVec<T> {
    type T = T;
    fn empty() -> Self {
        Self(vec![])
    }
    fn prepend(&mut self, value: Self::T, extra: &Extra) {
        self.0.push(value);
    }
}

impl<T: Debug> Debug for RVec<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.0.iter().rev()).finish()
    }
}

#[cfg(test)]
mod test {
    use pigeon::*;
    use super::*;

    #[test]
    fn json() {
        let json = r"{x: 1, y: [2, 3]}";
        let json = Parser::<Json>::parse(json).unwrap();
        println!("{json:?}");
    }
}