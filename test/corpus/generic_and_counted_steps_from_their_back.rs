// `len()` of a `step_by(n)` made in place, of a slice's items, a skip's or a
// map's of them, its source's count `n` apart, `Math.ceil(v.length / n)`, a
// `rev()`'s of one too;
// and a map of a type parameter a struct holds, given as it is, stepped
// from its back by `nth_back` and `rfind`, which asks `I`'s back of the
// dictionary its function is given, `IDoubleEndedIterator`, of numbers,
// the crate's iterator whose `next_back` is its own and a `Vec`'s items
// with a destructor (#811).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Held<I>(I);

impl<I: DoubleEndedIterator<Item = D>> Held<I> {
    fn mapped_nth_back(self) -> Option<u32> {
        self.0.map(|d| d.0).nth_back(1)
    }
}

struct Nums<I>(I);

impl<I: DoubleEndedIterator<Item = u32>> Nums<I> {
    fn mapped_nth_back(self) -> Option<u32> {
        self.0.map(|x| x * 2).nth_back(1)
    }

    fn mapped_rfind(self) -> Option<u32> {
        self.0.map(|x| x + 1).rfind(|x| x % 2 == 0)
    }
}

struct Count {
    lo: u32,
    hi: u32,
}

impl Iterator for Count {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        (self.lo < self.hi).then(|| {
            self.lo += 1;
            self.lo
        })
    }
}

impl DoubleEndedIterator for Count {
    fn next_back(&mut self) -> Option<u32> {
        println!("own back {}", self.hi);
        (self.lo < self.hi).then(|| {
            self.hi -= 1;
            self.hi + 1
        })
    }
}

fn main() {
    let v = vec![1u32, 2, 3, 4, 5, 6, 7, 8];
    println!("{}", v.iter().step_by(3).len());
    println!("{}", v.iter().step_by(1).len());
    println!("{}", v[..0].iter().step_by(2).len());
    println!("{}", v.iter().skip(1).step_by(3).len());
    println!("{}", v.iter().map(|x| x * 2).step_by(4).len());
    println!("{}", v.iter().step_by(3).rev().len());
    println!("{}", v.iter().skip(2).step_by(4).rev().len());
    let mut it = v.iter().step_by(3);
    it.next();
    println!("{}", it.len());
    println!(
        "{:?}",
        Held((1..5).map(D).collect::<Vec<_>>().into_iter()).mapped_nth_back()
    );
    println!("{:?}", Nums(vec![1, 2, 3, 4].into_iter()).mapped_nth_back());
    println!("{:?}", Nums(vec![1, 2, 3, 4].into_iter()).mapped_rfind());
    println!("{:?}", Nums(Count { lo: 0, hi: 4 }).mapped_nth_back());
    println!("end");
}
