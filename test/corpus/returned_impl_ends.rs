// An `impl DoubleEndedIterator` a function gives, a map, a filter or a chain
// of a slice's items: made knowing its back where it's made, as its caller
// can't, and stepped from its back by `rev`, `next_back`, `rfind`, `rfold` and
// `rposition` (#184).

struct Bag(Vec<i32>);
impl Bag {
    fn odd_squares(&self) -> impl DoubleEndedIterator<Item = i32> + '_ {
        self.0.iter().filter(|x| *x % 2 == 1).map(|x| x * x)
    }
    fn evens(&self) -> impl DoubleEndedIterator<Item = &i32> + '_ {
        self.0.iter().filter(|x| *x % 2 == 0)
    }
    fn doubled(&self) -> impl DoubleEndedIterator<Item = i32> + ExactSizeIterator + '_ {
        self.0.iter().map(|x| x * 2)
    }
    fn plain(&self) -> impl DoubleEndedIterator<Item = &i32> + '_ {
        self.0.iter()
    }
    fn ranged(&self) -> impl DoubleEndedIterator<Item = usize> {
        0..self.0.len()
    }
    fn chained(&self) -> impl DoubleEndedIterator<Item = i32> + '_ {
        self.0.iter().copied().chain([100, 200])
    }
}
fn main() {
    let b = Bag(vec![1, 2, 3, 4, 6]);
    println!("{:?}", b.odd_squares().rev().collect::<Vec<_>>());
    println!("{:?}", b.evens().rev().collect::<Vec<_>>());
    let mut e = b.evens();
    println!("{:?} {:?} {:?}", e.next_back(), e.next(), e.collect::<Vec<_>>());
    println!("{:?} {}", b.doubled().rev().collect::<Vec<_>>(), b.doubled().len());
    let mut d = b.doubled();
    d.next_back();
    println!("{} {:?}", d.len(), d.rev().next());
    println!("{:?} {:?}", b.plain().rev().nth(1), b.plain().next_back());
    println!("{:?}", b.ranged().rev().step_by(2).collect::<Vec<_>>());
    println!("{:?}", b.chained().rev().take(3).collect::<Vec<_>>());
    println!("{:?}", b.chained().rfind(|x| x % 3 == 0));
    println!("{:?}", b.evens().rfold(0, |a, x| a * 10 + x));
    println!("{:?}", b.doubled().rposition(|x| x == 4));
}
