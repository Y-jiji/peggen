/// One recorded failure: the input left where it was recorded, and its message.
#[derive(Debug, Clone, Copy)]
pub struct Failure<'a> {
    pub place: &'a str,
    pub cause: &'static str,
}

impl<'a> Failure<'a> {
    /// Line and column of `place` inside `input`, both from 1; the column counts chars.
    /// Panics if `place` is not the tail of `input`.
    /// Caller obligation: `input` is the string given to `parse`.
    pub fn locate(&self, input: &str) -> (usize, usize) {
        let offset = input.len().checked_sub(self.place.len());
        let tail = offset.filter(|&at| input.is_char_boundary(at)).map(|at| &input[at..]);
        let Some(tail) = tail.filter(|tail| std::ptr::eq(tail.as_ptr(), self.place.as_ptr())) else {
            panic!("peggen: `Failure::locate` got an input that does not end with this failure's place");
        };
        let head = &input[..input.len() - tail.len()];
        let line = head.matches('\n').count() + 1;
        let column = head.rsplit('\n').next().unwrap_or("").chars().count() + 1;
        (line, column)
    }
}
