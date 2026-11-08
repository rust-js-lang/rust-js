//@ library-differs
// A type parameter bound by the crate's trait that extends `Iterator`,
// `T: Reversed`, is given as it is, as `Reversed`'s impl takes it, and given
// `Iterator`'s dictionary too where it's stepped, as one bound by std's
// extension of it is; and a `&mut dyn` of one is given its value, not its
// pair, as generic code holds it (ADR 0511). Its sort, which a caller
// bounded by `Sorted` can't give what std's sort picks by `T::Item`, is
// `extension_trait_sorted_refused`'s (#934).
trait Reversed: Iterator {
    fn reversed(self) -> Vec<Self::Item>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        let mut v: Vec<Self::Item> = self.collect();
        v.reverse();
        v
    }
}
impl<I: Iterator> Reversed for I {}
trait Summed: Iterator<Item = u8> {
    fn total(&mut self) -> u32 {
        let mut n = 0;
        while let Some(x) = self.next() {
            n += u32::from(x);
        }
        n
    }
}
impl Summed for Count {}
impl Summed for std::vec::IntoIter<u8> {}
struct Count(u8);
impl Iterator for Count {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        self.0 = self.0.checked_sub(1)?;
        Some(self.0 * 3 % 5)
    }
}
fn through<T: Reversed>(t: T) -> Vec<T::Item>
where
    T::Item: Ord,
{
    t.reversed()
}
fn counted<T: Reversed>(t: T) -> usize {
    t.count()
}
fn summed<T: Summed>(mut t: T) -> u32 {
    t.total()
}
fn dynamic(s: &mut dyn Summed) -> u32 {
    s.total()
}
fn main() {
    println!("{:?} {:?}", through(vec![5, 3, 9, 1].into_iter()), through(Count(4)));
    println!("{:?}", Count(5).reversed());
    println!("{} {}", counted(Count(3)), counted(vec![1, 2].into_iter()));
    println!("{} {}", summed(Count(3)), summed(vec![1u8, 2].into_iter()));
    println!("{} {}", dynamic(&mut Count(4)), dynamic(&mut vec![4u8, 5].into_iter()));
    let mut c = Count(5);
    println!("{} {:?}", c.total(), c.next());
}
