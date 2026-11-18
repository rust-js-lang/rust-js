// An iterator of the crate's whose impls write their own provided methods,
// `ExactSizeIterator::len`, `Iterator`'s `size_hint`, `count`, `last`,
// `nth` and `fold`, `DoubleEndedIterator`'s `nth_back` and `rfold`, each
// saying it ran and answering what the default wouldn't: given to generic
// code through its bounds, its dictionary's entry for each is the impl's own
// wherever the impl writes one, by value, through a `&mut` and reversed, as
// std's `&mut I` and `Rev` pass each on (#2073).
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Own {
    left: u32,
    _held: Loud,
}

fn own(left: u32, name: &'static str) -> Own {
    Own {
        left,
        _held: Loud(name),
    }
}

impl Iterator for Own {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        (self.left > 0).then(|| {
            self.left -= 1;
            self.left
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("own size_hint");
        (self.left as usize, Some(self.left as usize))
    }
    fn count(self) -> usize {
        println!("own count");
        100 + self.left as usize
    }
    fn last(self) -> Option<u32> {
        println!("own last");
        Some(200 + self.left)
    }
    fn nth(&mut self, n: usize) -> Option<u32> {
        println!("own nth {n}");
        Some(300 + n as u32)
    }
    fn fold<B, F: FnMut(B, u32) -> B>(self, init: B, mut f: F) -> B {
        println!("own fold");
        f(init, 400)
    }
}

impl DoubleEndedIterator for Own {
    fn next_back(&mut self) -> Option<u32> {
        self.next()
    }
    fn nth_back(&mut self, n: usize) -> Option<u32> {
        println!("own nth_back {n}");
        Some(500 + n as u32)
    }
    fn rfold<B, F: FnMut(B, u32) -> B>(self, init: B, mut f: F) -> B {
        println!("own rfold");
        f(init, 600)
    }
}

impl ExactSizeIterator for Own {
    fn len(&self) -> usize {
        println!("own len");
        700 + self.left as usize
    }
}

fn len_of<I: ExactSizeIterator>(it: &I) -> usize {
    it.len()
}

fn hint_of<I: Iterator>(it: &I) -> (usize, Option<usize>) {
    it.size_hint()
}

fn count_of<I: Iterator>(it: I) -> usize {
    it.count()
}

fn last_of<I: Iterator>(it: I) -> Option<I::Item> {
    it.last()
}

fn nth_of<I: Iterator>(it: &mut I, n: usize) -> Option<I::Item> {
    it.nth(n)
}

fn nth_back_of<I: DoubleEndedIterator>(it: &mut I, n: usize) -> Option<I::Item> {
    it.nth_back(n)
}

fn sum_of<I: Iterator<Item = u32>>(it: I) -> u32 {
    it.fold(0, |a, b| a + b)
}

fn rsum_of<I: DoubleEndedIterator<Item = u32>>(it: I) -> u32 {
    it.rfold(0, |a, b| a + b)
}

fn main() {
    println!("{}", len_of(&own(3, "len")));
    println!("{:?}", hint_of(&own(3, "hint")));
    println!("{}", count_of(own(3, "count")));
    println!("{:?}", last_of(own(3, "last")));
    println!("{:?}", nth_of(&mut own(3, "nth"), 1));
    println!("{:?}", nth_back_of(&mut own(3, "nth_back"), 1));
    println!("{}", sum_of(own(3, "fold")));
    println!("{}", rsum_of(own(3, "rfold")));

    // Through a `&mut`, which passes on `len`, `size_hint`, `nth` and
    // `nth_back`, and folds by `try_fold`, stepping by `next`.
    let mut lent = own(3, "lent");
    println!("{}", len_of(&&mut lent));
    println!("{:?}", hint_of(&lent.by_ref()));
    println!("{:?}", nth_of(&mut lent.by_ref(), 0));
    println!("{:?}", nth_back_of(&mut lent.by_ref(), 0));
    println!("{}", sum_of(lent.by_ref()));
    println!("{}", count_of(lent.by_ref()));

    // Reversed: `Rev` passes `len` and `size_hint` on, its `fold` the
    // source's `rfold`, its `nth` the source's `nth_back`.
    println!("{}", len_of(&own(3, "rev len").rev()));
    println!("{:?}", hint_of(&own(3, "rev hint").rev()));
    println!("{:?}", nth_of(&mut own(3, "rev nth").rev(), 1));
    println!("{:?}", nth_back_of(&mut own(3, "rev nth_back").rev(), 1));
    println!("{}", sum_of(own(3, "rev fold").rev()));
    println!("{}", rsum_of(own(3, "rev rfold").rev()));
}
