// A type parameter's iterator generic code holds with its dictionary, one a
// struct's field holds or an `impl Into<I>` gives, consumed by std's
// compiled provided method, `partition` or `unzip` into generic sides or
// `peekable`, is the JS iterator that dictionary steps, as the method steps
// `self`, never the value itself (#2423).
struct Wrap<L>(L);

fn split<L: Iterator<Item = u32>, B: Default + Extend<u32>>(w: Wrap<L>) -> (B, B) {
    w.0.partition(|n| n % 2 == 0)
}

fn split_into<I: Iterator<Item = u32>, B: Default + Extend<u32>>(it: impl Into<I>) -> (B, B) {
    it.into().partition(|n| n % 2 == 1)
}

fn pairs<L: Iterator<Item = (u32, char)>, A: Default + Extend<u32>, B: Default + Extend<char>>(w: Wrap<L>) -> (A, B) {
    w.0.unzip()
}

fn first<L: Iterator<Item = u32>>(w: Wrap<L>) -> Option<u32> {
    let mut p = w.0.peekable();
    let seen = p.peek().copied();
    println!("next after peek: {:?}", p.next());
    seen
}

fn main() {
    let (even, odd): (Vec<u32>, Vec<u32>) = split(Wrap(1..5));
    println!("{even:?} {odd:?}");
    let (odd, even): (Vec<u32>, Vec<u32>) = split_into::<std::vec::IntoIter<u32>, _>(vec![1, 2, 3].into_iter());
    println!("{odd:?} {even:?}");
    let (numbers, letters): (Vec<u32>, String) = pairs(Wrap([(1, 'a'), (2, 'b')].into_iter()));
    println!("{numbers:?} {letters}");
    println!("{:?}", first(Wrap(7..9)));
}
