// A `filter()` made in place, `$filterEnds`, of a box of the crate's
// iterator or of what generic code is given, folded from its back by
// `rfold`, drops its predicate and the consumer's closure before what's
// left of its source, as std's `Filter::rfold` moves both into its
// source's `rfold`, which drops them first (#734, #2214).
struct D(&'static str);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn zero(_: &D) -> u32 {
    0
}

/// Counts down from `left`, holding what prints as it's dropped.
struct Own {
    left: u32,
    _held: D,
}

impl Iterator for Own {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        (self.left > 0).then(|| {
            self.left -= 1;
            self.left
        })
    }
}

impl DoubleEndedIterator for Own {
    fn next_back(&mut self) -> Option<u32> {
        self.next()
    }
}

fn own(left: u32) -> Own {
    Own { left, _held: D("own") }
}

fn odd_sum<I: DoubleEndedIterator<Item = u32>>(it: I) -> u32 {
    let g = D("generic filter");
    let h = D("generic f");
    it.filter(move |&x| x % 2 == 1 || zero(&g) > 0)
        .rfold(0, move |a, x| a + x + zero(&h))
}

fn main() {
    let g = D("filter");
    let h = D("f");
    let n = Box::new(own(4))
        .filter(move |&x| x % 2 == 1 || zero(&g) > 0)
        .rfold(0, move |a, x| a + x + zero(&h));
    println!("boxed {n}");
    println!("generic {}", odd_sum(own(4)));
    println!("generic boxed {}", odd_sum(Box::new(own(4))));
}
