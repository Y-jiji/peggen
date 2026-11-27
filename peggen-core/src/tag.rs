use std::sync::atomic::{AtomicUsize, Ordering};

/// One matched element. `Matcher::scope` pushes it first, so a parent comes before its children.
#[derive(Debug, Clone, Copy)]
pub enum Tag<'a> {
    /// A regex match: the matched slice.
    Text(&'a str),
    /// One `scope`: its rule, and the index one past its last inner tag.
    Rule { rule: Rule, stop: usize },
    /// One `repeat`: how many items matched.
    Many { count: usize },
}

/// A global rule number. Each rule takes one from `RULE_COUNT` on its first call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rule(usize);

/// The next free rule number.
static RULE_COUNT: AtomicUsize = AtomicUsize::new(0);

impl Rule {
    /// Takes the next number from `RULE_COUNT`.
    pub fn next() -> Rule {
        Rule(RULE_COUNT.fetch_add(1, Ordering::Relaxed))
    }
}
