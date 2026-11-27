/// One recorded message: the input it covers, its kind and its text.
#[derive(Debug, Clone, Copy)]
pub struct Message<'a> {
    pub span: &'a str,
    pub kind: MessageKind,
    pub text: &'static str,
}

/// What a message reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    /// An `#[error]` variant matched `span`, and parsing went on.
    Error,
    /// The parse ended: no `#[error]` variant recovered the failed variant at `span`, or `span` was left unparsed.
    Fatal,
}

impl<'a> Message<'a> {
    /// Line and column of the start of `span` inside `input`, both from 1; the column counts chars.
    /// Panics if `span` is not inside `input`.
    /// Caller obligation: `input` is the string given to `parse`.
    pub fn locate(&self, input: &str) -> (usize, usize) {
        let base = input.as_ptr() as usize;
        let from = (self.span.as_ptr() as usize).wrapping_sub(base);
        let inside = from <= input.len() && self.span.len() <= input.len() - from && input.is_char_boundary(from);
        assert!(inside, "peggen: `Message::locate` got an input that does not hold this message's span");
        let head = &input[..from];
        let line = head.matches('\n').count() + 1;
        let column = head.rsplit('\n').next().unwrap_or("").chars().count() + 1;
        (line, column)
    }
}
