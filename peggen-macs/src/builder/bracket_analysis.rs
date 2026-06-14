use std::collections::HashSet;
use crate::rule_ast::*;
use crate::builder::Rule;

#[derive(Debug, Clone)]
pub(crate) struct BracketPair {
    pub open: String,
    pub close: String,
}

fn first_terminal(expr: &RuleExpr) -> Option<&str> {
    match expr {
        RuleExpr::Literal(s) if !s.is_empty() => Some(s),
        RuleExpr::Seq(elems) if !elems.is_empty() => first_terminal(&elems[0]),
        _ => None,
    }
}

fn last_terminal(expr: &RuleExpr) -> Option<&str> {
    match expr {
        RuleExpr::Literal(s) if !s.is_empty() => Some(s),
        RuleExpr::Seq(elems) if !elems.is_empty() => last_terminal(elems.last().unwrap()),
        _ => None,
    }
}

fn count_terminal(expr: &RuleExpr, target: &str) -> usize {
    match expr {
        RuleExpr::Literal(s) if s == target => 1,
        RuleExpr::Seq(elems) => elems.iter().map(|e| count_terminal(e, target)).sum(),
        RuleExpr::Choice(a, b) => count_terminal(a, target).max(count_terminal(b, target)),
        RuleExpr::Rep(e, _) | RuleExpr::Not(e) | RuleExpr::And(e) => count_terminal(e, target),
        RuleExpr::SepRep { expr, sep, .. } => count_terminal(expr, target) + count_terminal(sep, target),
        _ => 0,
    }
}

/// Condition 1: Consistent Pairing
/// Every rule must use L and R in equal counts (or neither).
fn check_consistent_pairing(rules: &[Rule], open: &str, close: &str) -> bool {
    for rule in rules {
        let oc = count_terminal(&rule.body, open);
        let cc = count_terminal(&rule.body, close);
        if oc != cc {
            return false;
        }
    }
    true
}

/// Condition 3: No Prefix Ambiguity
/// No delimiter in the pair set is a prefix of another.
fn check_no_prefix_ambiguity(pairs: &[BracketPair]) -> Vec<bool> {
    let all_delims: Vec<&str> = pairs.iter()
        .flat_map(|p| [p.open.as_str(), p.close.as_str()])
        .collect();
    pairs.iter().map(|pair| {
        for &d in &all_delims {
            if d != pair.open && (d.starts_with(&pair.open) || pair.open.starts_with(d)) {
                return false;
            }
            if d != pair.close && (d.starts_with(&pair.close) || pair.close.starts_with(d)) {
                return false;
            }
        }
        true
    }).collect()
}

pub(crate) fn analyze_bracket_pairs(rules: &[Rule]) -> Vec<BracketPair> {
    // Step 1: Find candidate pairs from rules starting and ending with distinct literals
    let mut candidates: Vec<(String, String)> = Vec::new();
    for rule in rules {
        if let (Some(f), Some(l)) = (first_terminal(&rule.body), last_terminal(&rule.body)) {
            if f != l {
                let pair = (f.to_string(), l.to_string());
                if !candidates.contains(&pair) {
                    candidates.push(pair);
                }
            }
        }
    }

    // Step 2: Condition 1 — Consistent Pairing + uniqueness
    let mut valid: Vec<BracketPair> = Vec::new();
    let mut used_opens: HashSet<String> = HashSet::new();
    let mut used_closes: HashSet<String> = HashSet::new();

    for (open, close) in &candidates {
        if !check_consistent_pairing(rules, open, close) {
            continue;
        }
        // Check no other candidate shares this open or close
        let mut unique = true;
        for (o, c) in &candidates {
            if (o == open && c != close) || (c == close && o != open) {
                unique = false;
                break;
            }
        }
        if !unique { continue; }
        if used_opens.contains(open) || used_closes.contains(close) { continue; }

        used_opens.insert(open.clone());
        used_closes.insert(close.clone());
        valid.push(BracketPair { open: open.clone(), close: close.clone() });
    }

    // Step 3: Condition 3 — No Prefix Ambiguity
    let keep = check_no_prefix_ambiguity(&valid);
    valid.into_iter().zip(keep).filter(|(_, k)| *k).map(|(p, _)| p).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::Rule;
    use std::collections::HashMap;

    fn make_rule(body: RuleExpr) -> Rule {
        Rule {
            group: 0,
            tags: vec![],
            named: false,
            trace: false,
            error: false,
            body,
            variant: syn::Ident::new("Test", proc_macro2::Span::call_site()),
            fields: HashMap::new(),
        }
    }

    #[test]
    fn test_json_brackets() {
        let rules = vec![
            make_rule(RuleExpr::Literal("null".into())),
            make_rule(RuleExpr::Seq(vec![
                RuleExpr::Literal("[".into()),
                RuleExpr::Literal("]".into()),
            ])),
            make_rule(RuleExpr::Seq(vec![
                RuleExpr::Literal("{".into()),
                RuleExpr::Literal("}".into()),
            ])),
        ];
        let pairs = analyze_bracket_pairs(&rules);
        assert_eq!(pairs.len(), 2);
        let opens: Vec<&str> = pairs.iter().map(|p| p.open.as_str()).collect();
        assert!(opens.contains(&"["));
        assert!(opens.contains(&"{"));
    }

    #[test]
    fn test_self_pairing_excluded() {
        let rules = vec![
            make_rule(RuleExpr::Seq(vec![
                RuleExpr::Literal("\"".into()),
                RuleExpr::Literal("\"".into()),
            ])),
        ];
        let pairs = analyze_bracket_pairs(&rules);
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_inconsistent_pairing() {
        let rules = vec![
            make_rule(RuleExpr::Seq(vec![
                RuleExpr::Literal("{".into()),
                RuleExpr::Literal("}".into()),
            ])),
            make_rule(RuleExpr::Seq(vec![
                RuleExpr::Literal("{".into()),
                RuleExpr::Literal(";".into()),
            ])),
        ];
        let pairs = analyze_bracket_pairs(&rules);
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_prefix_ambiguity() {
        let rules = vec![
            make_rule(RuleExpr::Seq(vec![
                RuleExpr::Literal("{".into()),
                RuleExpr::Literal("}".into()),
            ])),
            make_rule(RuleExpr::Seq(vec![
                RuleExpr::Literal("{{".into()),
                RuleExpr::Literal("}}".into()),
            ])),
        ];
        let pairs = analyze_bracket_pairs(&rules);
        assert!(pairs.is_empty());
    }
}
