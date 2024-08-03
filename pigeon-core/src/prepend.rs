pub trait Prepend<Extra: Copy> {
    type Item;
    fn empty(with: Extra) -> Self;
    fn prepend(&mut self, value: Self::Item, with: Extra);
}