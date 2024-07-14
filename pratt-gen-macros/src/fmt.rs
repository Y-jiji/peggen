use logos::*;

#[derive(Logos, Debug)]
pub enum Token {
    #[regex(r" ")]
    Space,
    #[regex(r"\{[A-Za-z0-9]+(:[0-9]+)?\}", |x| split(x.slice()))]
    Format((String, u16)),
    #[regex(r"\{[A-Za-z0-9]+:`[^`]*`\}", |x| regex(x.slice()))]
    Regex((String, String)),
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

fn regex(fmt: &str) -> (String, String) {
    let fmt = fmt.trim_start_matches("{").trim_end_matches("}");
    let hole = fmt.split(":").nth(0).unwrap().to_string();
    let regex = &fmt[hole.len()+2..fmt.len()-1];
    let regex = if !regex.starts_with("^") {
        format!("^{regex}")
    } else {
        regex.to_string()
    };
    (hole, regex)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse() {
        println!("{:?}", parse_fmt_string("{0} {1}").unwrap());
    }
}