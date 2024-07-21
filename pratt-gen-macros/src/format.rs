use logos::*;

#[derive(Debug)]
pub enum Format {
    // [0: {0} + {1} + [2: {0}] , ] ({PUSH(0):...}) {1}
    Repeat((String, Vec<Format>)),
    // 
    Symbol((String, usize)),
    // {0:`...`}
    RegExp((String, String)),
    // xx
    Token(String),
    // 
    Space,
}

impl Format {
    pub fn parse() -> Format {
        todo!()
    }
}