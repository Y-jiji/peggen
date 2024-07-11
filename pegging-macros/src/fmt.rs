use logos::*;

#[derive(Logos, Debug)]
pub enum Token {
    #[regex(r" ")]
    Space,
    #[regex(r"\{[A-Za-z0-9]+(:[0-9]+)?\}", |x| split(x.slice()))]
    Format((String, u16)),
    #[regex(r"(\{\{|\}\}|\{ \}|[^ \{\}])+", |x| escape(x.slice()))]
    Literal(String),
}

pub fn parse_fmt_string(fmt: &str) -> Result<Vec<Token>, ()> {
    Token::lexer(fmt).try_fold(vec![], |mut x, y| {x.push(y?); Ok(x)})
}

fn escape(fmt: &str) -> String {
    let mut fmt = fmt.to_string();
    fmt = fmt.replace("{ }", " ");
    fmt = fmt.replace("{{", "{");
    fmt = fmt.replace("}}", "}");
    fmt
}

fn split(fmt: &str) -> (String, u16) {
    let fmt = fmt.trim_start_matches("{").trim_end_matches("}");
    if !fmt.contains(":") {
        (fmt.to_string(), 0)
    } else {
        let bp = fmt.split(":").nth(1).unwrap().trim_start_matches("0").parse::<u16>().unwrap_or(0);
        let fmt = fmt.split(":").nth(0).unwrap().to_string();
        (fmt, bp)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse() {
        println!("{:?}", parse_fmt_string("{0} {1}").unwrap());
    }
}