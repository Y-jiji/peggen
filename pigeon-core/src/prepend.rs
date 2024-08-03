pub trait Prepend<Extra: Copy> {
    type Item;
    fn empty(extra: Extra) -> Self;
    fn prepend(&mut self, value: Self::Item, extra: Extra);
}