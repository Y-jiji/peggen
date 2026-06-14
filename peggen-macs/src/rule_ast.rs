#[derive(Debug, Clone)]
pub(crate) enum RuleExpr {
    Literal(String),
    Field(FieldRef),
    FieldRegex(FieldRef, String),
    FieldTag(FieldRef, String),
    FieldMulti(Vec<FieldRef>, String),
    SubruleRef(String),
    Seq(Vec<RuleExpr>),
    Choice(Box<RuleExpr>, Box<RuleExpr>),
    Rep(Box<RuleExpr>, RepKind),
    SepRep {
        expr: Box<RuleExpr>,
        sep: Box<RuleExpr>,
        at_least_one: bool,
    },
    Not(Box<RuleExpr>),
    And(Box<RuleExpr>),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FieldRef {
    Named(String),
    Positional(usize),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum RepKind {
    ZeroOrMore,
    OneOrMore,
    Optional,
}

impl FieldRef {
    pub fn key(&self) -> String {
        match self {
            FieldRef::Named(n) => n.clone(),
            FieldRef::Positional(i) => format!("{i}"),
        }
    }
}

impl RuleExpr {
    pub fn has_field_refs(&self) -> bool {
        match self {
            RuleExpr::Field(_) | RuleExpr::FieldRegex(..) | RuleExpr::FieldTag(..) | RuleExpr::FieldMulti(..) => true,
            RuleExpr::Literal(_) | RuleExpr::SubruleRef(_) => false,
            RuleExpr::Seq(elems) => elems.iter().any(|e| e.has_field_refs()),
            RuleExpr::Choice(a, b) => a.has_field_refs() || b.has_field_refs(),
            RuleExpr::Rep(e, _) => e.has_field_refs(),
            RuleExpr::SepRep { expr, sep, .. } => expr.has_field_refs() || sep.has_field_refs(),
            RuleExpr::Not(e) | RuleExpr::And(e) => e.has_field_refs(),
        }
    }

    pub fn collect_field_refs(&self, out: &mut Vec<FieldRef>) {
        match self {
            RuleExpr::Field(f) | RuleExpr::FieldRegex(f, _) | RuleExpr::FieldTag(f, _) => {
                out.push(f.clone());
            }
            RuleExpr::FieldMulti(frefs, _) => {
                out.extend(frefs.iter().cloned());
            }
            RuleExpr::Literal(_) | RuleExpr::SubruleRef(_) => {}
            RuleExpr::Seq(elems) => {
                for e in elems {
                    e.collect_field_refs(out);
                }
            }
            RuleExpr::Choice(a, b) => {
                a.collect_field_refs(out);
                b.collect_field_refs(out);
            }
            RuleExpr::Rep(e, _) => e.collect_field_refs(out),
            RuleExpr::SepRep { expr, .. } => expr.collect_field_refs(out),
            RuleExpr::Not(e) | RuleExpr::And(e) => e.collect_field_refs(out),
        }
    }
}
