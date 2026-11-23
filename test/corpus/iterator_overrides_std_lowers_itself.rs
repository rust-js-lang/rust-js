//@ library-differs
// An iterator of the crate's whose impl writes its own of `Iterator`'s
// provided methods that rust-js lowers itself, not std's compiled body,
// `position` (a `&mut self` one), `any`, `find` and `max`, runs them where
// generic code calls them, as Rust's dispatch picks; through a `&mut I`
// std's default, as std's `&mut I` passes none on, a `by_ref()`'s, of a lend of a lend too, its own; `<&mut I>::position`, std's. And one crossing from
// generic code given a dictionary to generic code given none, or back,
// keeps its own (#2421).
struct Counter(u32);

impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 += 1;
        (self.0 <= 3).then_some(self.0)
    }
    fn position<P: FnMut(u32) -> bool>(&mut self, _p: P) -> Option<usize> {
        println!("own position at {}", self.0);
        Some(99)
    }
    fn any<F: FnMut(u32) -> bool>(&mut self, _f: F) -> bool {
        println!("own any");
        true
    }
    fn find<P: FnMut(&u32) -> bool>(&mut self, _p: P) -> Option<u32> {
        println!("own find");
        Some(7)
    }
    fn max(self) -> Option<u32> {
        println!("own max");
        Some(42)
    }
    fn partition<B: Default + Extend<u32>, F: FnMut(&u32) -> bool>(self, _f: F) -> (B, B) {
        println!("own partition");
        (B::default(), B::default())
    }
}

trait Into2<T> {
    fn into2(self) -> T;
}

impl<T> Into2<T> for T {
    fn into2(self) -> T {
        self
    }
}

fn second<I: Iterator<Item = u32>>(mut it: I) -> Option<usize> {
    it.position(|n| n == 2)
}

fn second_lent<I: Iterator<Item = u32>>(it: &mut I) -> Option<usize> {
    it.position(|n| n == 2)
}

fn second_by_ref<I: Iterator<Item = u32>>(mut it: I) -> Option<usize> {
    it.by_ref().position(|n| n == 2)
}

fn second_by_ref_twice<I: Iterator<Item = u32>>(mut it: I) -> Option<usize> {
    it.by_ref().by_ref().position(|n| n == 2)
}

fn second_reborrowed<I: Iterator<Item = u32>>(mut it: I) -> Option<usize> {
    let r = &mut it;
    let s = &mut *r;
    s.position(|n| n == 2)
}

fn second_of_the_lend<I: Iterator<Item = u32>>(mut it: I) -> Option<usize> {
    let mut r = &mut it;
    <&mut I as Iterator>::position(&mut r, |n| n == 2)
}

fn has_three<I: Iterator<Item = u32>>(mut it: I) -> bool {
    it.any(|n| n == 3)
}

fn first_even<I: Iterator<Item = u32>>(mut it: I) -> Option<u32> {
    it.find(|n| n % 2 == 0)
}

fn largest<I: Iterator<Item = u32>>(it: I) -> Option<u32> {
    it.max()
}

fn evens<I: Iterator<Item = u32>>(it: I) -> (Vec<u32>, Vec<u32>) {
    it.partition(|n| n % 2 == 0)
}

fn given_to_plain<I: Iterator<Item = u32>>(it: impl Into2<I>) -> (Vec<u32>, Vec<u32>) {
    evens(it.into2())
}

fn given_evens<I: Iterator<Item = u32>>(it: impl Into2<I>) -> (Vec<u32>, Vec<u32>) {
    it.into2().partition(|n| n % 2 == 0)
}

fn plain_to_given<I: Iterator<Item = u32>>(it: I) -> (Vec<u32>, Vec<u32>) {
    given_evens::<I>(it)
}

fn main() {
    println!("{:?}", second(Counter(0)));
    println!("{:?}", second_lent(&mut Counter(0)));
    println!("{:?}", second(&mut Counter(0)));
    println!("{:?}", second_by_ref(Counter(0)));
    println!("{:?}", second_by_ref_twice(Counter(0)));
    println!("{:?}", second_reborrowed(Counter(0)));
    println!("{:?}", second_of_the_lend(Counter(0)));
    println!("{}", has_three(Counter(0)));
    println!("{:?}", first_even(Counter(0)));
    println!("{:?}", largest(Counter(0)));
    println!("{:?}", largest(Counter(0).map(|n| n * 10)));
    println!("{:?}", given_to_plain(Counter(0)));
    println!("{:?}", plain_to_given(Counter(0)));
    println!("{:?}", Counter(0).position(|n| n == 1));
}
