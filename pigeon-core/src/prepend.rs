pub trait Prepend<Extra> {
    type T;
    fn empty() -> Self;
    fn prepend(&mut self, value: Self::T, extra: &Extra);
}