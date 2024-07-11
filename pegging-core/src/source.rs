#[derive(Debug, Clone, Copy)]
pub struct Source<'a> {
    pub split: usize,
    pub inner: &'a str,
}

impl<'a> Source<'a> {
    pub fn proceed(&self, by: usize) -> Self {
        Source { split: self.split + by, inner: self.inner }
    }
}

impl<'a, T: std::ops::RangeBounds<usize>> std::ops::Index<T> for Source<'a> {
    type Output = str;
    fn index(&self, index: T) -> &Self::Output {
        use std::ops::Bound::*;
        let start = match index.start_bound() {
            Excluded(x) => self.split + x + 1,
            Included(x) => self.split + x,
            Unbounded => self.split,
        };
        let end = match index.end_bound() {
            Excluded(x) => self.split + x.max(&1) - 1,
            Included(x) => self.split + x,
            Unbounded => self.inner.len(),
        }.min(self.inner.len());
        &self.inner[start..end]
    }
}
