use super::*;

/// ### Brief
/// A string in arena, only allow push when it is the tail of arena. 
/// It is a unique pointer to memory. Thus, This type is not Copy. 
pub struct AStr<'a> {
    begin: &'a str,
    arena: &'a Arena,
}

