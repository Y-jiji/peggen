use std::fmt::Debug;
use pratt_gen::*;

#[derive(Debug, Clone, Copy, ParseImpl, Space)]
pub enum Json<'a> {
    #[parse("{0}")]
    Int(i64),
    #[parse("{0}")]
    Float(f64),
    // TODO: implement string escape
    // #[parse("{0}")]
    // Str(&'a str),
    #[parse("{{ {0} }}")]
    Obj(&'a Obj<'a>),
    #[parse("[ {0} ]")]
    Arr(&'a Arr<'a>),
}

#[derive(Clone, Copy, ParseImpl, Space)]
pub enum Obj<'a> {
    #[parse("{0:`[a-zA-Z_][a-zA-Z0-9_]+`} : {1} , {2}")]
    Next(&'a str, Json<'a>, &'a Self),
    #[parse("{0:`[a-zA-Z_][a-zA-Z0-9_]+`} : {1}")]
    Just(&'a str, Json<'a>),
    #[parse("")]
    Null(),
}

#[derive(Clone, Copy, ParseImpl, Space)]
pub enum Arr<'a> {
    #[parse("{0} , {1}")]
    Next(Json<'a>, &'a Arr<'a>),
    #[parse("{0}")]
    Just(Json<'a>),
    #[parse("")]
    Null(),
}

impl<'a> Iterator for Obj<'a> {
    type Item = (&'a str, Json<'a>);
    fn next(&mut self) -> Option<Self::Item> {
        match *self {
            Self::Next(id, json, that) => {
                *self = *that;
                return Some((id, json));
            }
            Self::Just(id, json) => {
                *self = Self::Null();
                return Some((id, json));
            }
            Self::Null() => None
        }
    }
}

impl<'a> Debug for Obj<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(*self).finish()
    }
}

impl<'a> Iterator for Arr<'a> {
    type Item = Json<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        match *self {
            Self::Next(json, that) => {
                *self = *that;
                return Some(json);
            }
            Self::Just(json) => {
                *self = Self::Null();
                return Some(json);
            }
            Self::Null() => None
        }
    }
}

impl<'a> Debug for Arr<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(*self).finish()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn json() {
        let source = Source::new(r#"["我是你爹", 10, {x:"y", y:[10, 11]}]"#);
        let out_arena = Arena::new();
        let err_arena = Arena::new();
        println!("{:?}", parse::<Json>(source, &out_arena, &err_arena));
    }
}