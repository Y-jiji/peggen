pub trait Prepend<Extra> {
    type Item;
    fn empty() -> Self;
    fn prepend(&mut self, value: Self::Item, extra: &Extra);
}