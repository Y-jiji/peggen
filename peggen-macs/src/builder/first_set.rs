use crate::rule_ast::*;
use crate::attr_parser::GrammarContext;

#[derive(Debug, Clone)]
pub(crate) enum FirstSet {
    Known(Vec<u8>),
    Unknown,
}

impl FirstSet {
    pub fn union(self, other: FirstSet) -> FirstSet {
        match (self, other) {
            (FirstSet::Known(mut a), FirstSet::Known(b)) => {
                for byte in b {
                    if !a.contains(&byte) { a.push(byte); }
                }
                FirstSet::Known(a)
            }
            _ => FirstSet::Unknown,
        }
    }
}

pub(crate) fn compute_first_set(expr: &RuleExpr, ctx: &GrammarContext) -> FirstSet {
    match expr {
        RuleExpr::Literal(s) if !s.is_empty() => FirstSet::Known(vec![s.as_bytes()[0]]),
        RuleExpr::Literal(_) => FirstSet::Unknown,
        RuleExpr::Seq(elems) if !elems.is_empty() => compute_first_set(&elems[0], ctx),
        RuleExpr::Seq(_) => FirstSet::Unknown,
        RuleExpr::Field(_) | RuleExpr::FieldTag(_, _) => FirstSet::Unknown,
        RuleExpr::FieldRegex(_, name) | RuleExpr::SubruleRef(name) => {
            if let Some(sub_expr) = ctx.subrules.get(name) {
                compute_first_set(sub_expr, ctx)
            } else if let Some(pattern) = ctx.regexes.get(name) {
                match regex_first_bytes(pattern) {
                    Some(bytes) => FirstSet::Known(bytes),
                    None => FirstSet::Unknown,
                }
            } else {
                FirstSet::Unknown
            }
        }
        RuleExpr::FieldMulti(_, name) => {
            ctx.subrules.get(name)
                .map(|e| compute_first_set(e, ctx))
                .unwrap_or(FirstSet::Unknown)
        }
        RuleExpr::Choice(a, b) => compute_first_set(a, ctx).union(compute_first_set(b, ctx)),
        RuleExpr::Rep(inner, RepKind::OneOrMore) => compute_first_set(inner, ctx),
        RuleExpr::Rep(_, _) => FirstSet::Unknown,
        RuleExpr::SepRep { expr, at_least_one: true, .. } => compute_first_set(expr, ctx),
        RuleExpr::SepRep { .. } => FirstSet::Unknown,
        RuleExpr::Not(_) | RuleExpr::And(_) => FirstSet::Unknown,
    }
}

pub(crate) fn regex_first_bytes(pattern: &str) -> Option<Vec<u8>> {
    let bytes = pattern.as_bytes();
    let alts = split_top_level(bytes);
    let mut result = Vec::new();
    for alt in alts {
        let first = alt_first_bytes(alt)?;
        for b in first {
            if !result.contains(&b) { result.push(b); }
        }
    }
    if result.is_empty() { None } else { Some(result) }
}

fn split_top_level(bytes: &[u8]) -> Vec<&[u8]> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut depth = 0i32;
    let mut in_class = false;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => { i += 2; continue; }
            b'[' if !in_class => { in_class = true; }
            b']' if in_class => { in_class = false; }
            b'(' if !in_class => { depth += 1; }
            b')' if !in_class => { depth -= 1; }
            b'|' if !in_class && depth == 0 => {
                result.push(&bytes[start..i]);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    result.push(&bytes[start..]);
    result
}

fn alt_first_bytes(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'^' || bytes[i] == b'$' { i += 1; continue; }
        let (chars, end) = parse_element(bytes, i)?;
        if chars.is_empty() { i = end; continue; }
        let (optional, next) = quantifier_at(bytes, end);
        if optional {
            let rest = alt_first_bytes(&bytes[next..]);
            return match rest {
                Some(rest_chars) => {
                    let mut combined = chars;
                    for b in rest_chars { if !combined.contains(&b) { combined.push(b); } }
                    Some(combined)
                }
                None => None,
            };
        }
        return Some(chars);
    }
    None
}

fn quantifier_at(bytes: &[u8], pos: usize) -> (bool, usize) {
    if pos >= bytes.len() { return (false, pos); }
    match bytes[pos] {
        b'*' | b'?' => {
            let next = if pos + 1 < bytes.len() && bytes[pos + 1] == b'?' { pos + 2 } else { pos + 1 };
            (true, next)
        }
        b'+' => {
            let next = if pos + 1 < bytes.len() && bytes[pos + 1] == b'?' { pos + 2 } else { pos + 1 };
            (false, next)
        }
        b'{' => {
            let mut j = pos + 1;
            let mut min = 0usize;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                min = min * 10 + (bytes[j] - b'0') as usize;
                j += 1;
            }
            while j < bytes.len() && bytes[j] != b'}' { j += 1; }
            if j < bytes.len() { j += 1; }
            (min == 0, j)
        }
        _ => (false, pos),
    }
}

fn parse_element(bytes: &[u8], pos: usize) -> Option<(Vec<u8>, usize)> {
    if pos >= bytes.len() { return None; }
    match bytes[pos] {
        b'\\' => {
            if pos + 1 >= bytes.len() { return None; }
            match bytes[pos + 1] {
                b'b' | b'B' => Some((vec![], pos + 2)),
                b'd' => Some(((b'0'..=b'9').collect(), pos + 2)),
                b'D' => {
                    let v: Vec<u8> = (0u8..128).filter(|b| !b.is_ascii_digit()).collect();
                    Some((v, pos + 2))
                }
                b'w' => {
                    let mut v: Vec<u8> = (b'0'..=b'9').collect();
                    v.extend(b'a'..=b'z'); v.extend(b'A'..=b'Z'); v.push(b'_');
                    Some((v, pos + 2))
                }
                b's' => Some((vec![b' ', b'\t', b'\n', b'\r'], pos + 2)),
                ch => Some((vec![ch], pos + 2)),
            }
        }
        b'[' => parse_char_class(bytes, pos),
        b'(' => {
            let mut depth = 1i32;
            let mut j = pos + 1;
            if j < bytes.len() && bytes[j] == b'?' {
                j += 1;
                while j < bytes.len() && bytes[j] != b':' && bytes[j] != b')' { j += 1; }
                if j < bytes.len() && bytes[j] == b':' { j += 1; }
            }
            let inner_start = j;
            while j < bytes.len() {
                match bytes[j] {
                    b'\\' => { j += 2; continue; }
                    b'(' => { depth += 1; }
                    b')' => { depth -= 1; if depth == 0 { break; } }
                    _ => {}
                }
                j += 1;
            }
            let inner = std::str::from_utf8(&bytes[inner_start..j]).ok()?;
            let first = regex_first_bytes(inner)?;
            Some((first, j + 1))
        }
        b'.' => None,
        ch => Some((vec![ch], pos + 1)),
    }
}

fn parse_char_class(bytes: &[u8], pos: usize) -> Option<(Vec<u8>, usize)> {
    let mut i = pos + 1;
    let negated = i < bytes.len() && bytes[i] == b'^';
    if negated { i += 1; }
    if i < bytes.len() && bytes[i] == b']' { i += 1; }
    let mut chars = Vec::new();
    while i < bytes.len() && bytes[i] != b']' {
        if bytes[i] == b'\\' {
            i += 1;
            if i >= bytes.len() { return None; }
            match bytes[i] {
                b'd' => chars.extend(b'0'..=b'9'),
                b'w' => { chars.extend(b'0'..=b'9'); chars.extend(b'a'..=b'z'); chars.extend(b'A'..=b'Z'); chars.push(b'_'); }
                b's' => chars.extend(&[b' ', b'\t', b'\n', b'\r']),
                ch => chars.push(ch),
            }
            i += 1;
        } else if i + 2 < bytes.len() && bytes[i + 1] == b'-' && bytes[i + 2] != b']' {
            for ch in bytes[i]..=bytes[i + 2] { chars.push(ch); }
            i += 3;
        } else {
            chars.push(bytes[i]);
            i += 1;
        }
    }
    if i >= bytes.len() { return None; }
    i += 1;
    if negated {
        let result: Vec<u8> = (0u8..128).filter(|b| !chars.contains(b)).collect();
        Some((result, i))
    } else {
        if chars.is_empty() { return None; }
        Some((chars, i))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_first_bytes() {
        assert_eq!(regex_first_bytes("false|true"), Some(vec![b'f', b't']));

        let num = regex_first_bytes("0|-?[1-9][0-9]*").unwrap();
        assert!(num.contains(&b'0'));
        assert!(num.contains(&b'-'));
        assert!(num.contains(&b'1'));
        assert!(num.contains(&b'9'));

        let flt = regex_first_bytes("-?(0|[1-9][0-9]*)\\.([0-9]+)").unwrap();
        assert!(flt.contains(&b'-'));
        assert!(flt.contains(&b'0'));
        assert!(flt.contains(&b'1'));

        let alpha = regex_first_bytes("[A-Za-z]+").unwrap();
        assert!(alpha.contains(&b'A'));
        assert!(alpha.contains(&b'z'));

        let id = regex_first_bytes("[a-z0-9]").unwrap();
        assert!(id.contains(&b'a'));
        assert!(id.contains(&b'9'));

        assert_eq!(regex_first_bytes("int\\b"), Some(vec![b'i']));

        assert_eq!(regex_first_bytes("\\s*"), None);
        assert_eq!(regex_first_bytes("[^\"]*"), None);
    }
}
