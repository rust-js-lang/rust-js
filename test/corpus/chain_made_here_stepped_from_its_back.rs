// A chain made where it's stepped from its back, `a.chain(b).nth_back(1)`,
// its own `next_back` and `rfind` too: made knowing its back, its sides
// made knowing theirs, as one an adapter asks beneath it is, in generic code
// and of ranges, a `Vec`'s items, what makes an iterator, the crate's own
// double-ended iterator, and items with a destructor, each passed over
// dropped as std's `nth_back` drops it (#1118).
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Down(u32);

impl Iterator for Down {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        (self.0 > 0).then(|| {
            self.0 -= 1;
            self.0 + 100
        })
    }
}

impl DoubleEndedIterator for Down {
    fn next_back(&mut self) -> Option<u32> {
        self.next().map(|n| n + 1000)
    }
}

fn nth_back<I: DoubleEndedIterator<Item = u32>>(a: I, b: I) -> Option<u32> {
    a.chain(b).nth_back(1)
}

fn last_of<I: DoubleEndedIterator<Item = u32>>(a: I, b: I) -> Option<u32> {
    a.chain(b).next_back()
}

fn found<I: DoubleEndedIterator<Item = u32>>(a: I, b: I) -> Option<u32> {
    a.chain(b).rfind(|x| x % 2 == 1)
}

fn main() {
    println!("{:?}", nth_back(1..3, 7..9));
    println!("{:?}", nth_back(vec![1, 2].into_iter(), vec![3].into_iter()));
    println!("{:?}", nth_back(1..2, 2..2));
    println!("{:?}", last_of(5..5, 6..6));
    println!("{:?}", found(1..4, 4..7));
    println!("{:?}", (1u32..3).chain(7..9).nth_back(3));
    println!("{:?}", (1u32..3).chain(7..9).nth_back(4));
    let v = vec![1, 2, 3];
    let w = [10, 20];
    println!("{:?}", v.iter().chain(&w).next_back());
    println!("{:?}", v.iter().copied().chain(w).rfind(|x| *x < 15));
    println!("{:?}", Down(2).chain(Down(3)).nth_back(1));
    let louds = vec![Loud(1), Loud(2)];
    let more = vec![Loud(3), Loud(4)];
    println!("{:?}", louds.into_iter().chain(more).nth_back(2).map(|l| l.0));
    println!("end");
}
