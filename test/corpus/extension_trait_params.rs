//@ library-differs
// A type parameter bound by the crate's trait that extends `Iterator`,
// `T: Sorted`, is given as it is, as `Sorted`'s impl takes it, and given
// `Iterator`'s dictionary too where it's stepped, as one bound by std's
// extension of it is; and a `&mut dyn` of one is given its value, not its
// pair, as generic code holds it (ADR 0511).
trait Sorted: Iterator {
    fn sorted(self) -> Vec<Self::Item>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        let mut v: Vec<Self::Item> = self.collect();
        v.sort();
        v
    }
}
impl<I: Iterator> Sorted for I {}
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
fn through<T: Sorted>(t: T) -> Vec<T::Item>
where
    T::Item: Ord,
{
    t.sorted()
}
fn counted<T: Sorted>(t: T) -> usize {
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
    println!("{:?}", Count(5).sorted());
    println!("{} {}", counted(Count(3)), counted(vec![1, 2].into_iter()));
    println!("{} {}", summed(Count(3)), summed(vec![1u8, 2].into_iter()));
    println!("{} {}", dynamic(&mut Count(4)), dynamic(&mut vec![4u8, 5].into_iter()));
    let mut c = Count(5);
    println!("{} {:?}", c.total(), c.next());
}
