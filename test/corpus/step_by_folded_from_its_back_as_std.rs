// A `step_by(n)` folded or found from its back, `rfold` and `rfind`, as
// std's `StepBy` steps it (`spec_rfold`, `spec_try_rfold`): its last item
// once, which asks its source's length, then each its source's
// `nth_back(n - 1)`, the crate's own, which drops what it passes itself;
// so of one under a `take(n)`, a `fuse()`, an `enumerate()`, a `skip(n)`
// or another `step_by(n)`, and of a `Vec`'s, whose `nth_back` drops what
// it passes as one slice, front to back. It asked the length at each step
// and dropped each item it passed itself (#834). And given to generic code,
// std's compiled struct is given its compiled `rfold` and `nth_back`, which
// ask its source's `nth_back`, of items without a destructor too: it was
// folded by its `next_back`, which asks the length each time.
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Count {
    front: u32,
    back: u32,
}

impl Iterator for Count {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        if self.front == self.back {
            return None;
        }
        self.front += 1;
        Some(D(self.front))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = (self.back - self.front) as usize;
        println!("size_hint {left}");
        (left, Some(left))
    }
}

impl DoubleEndedIterator for Count {
    fn next_back(&mut self) -> Option<D> {
        if self.front == self.back {
            return None;
        }
        self.back -= 1;
        Some(D(self.back + 1))
    }

    fn nth_back(&mut self, n: usize) -> Option<D> {
        println!("nth_back {n}");
        for _ in 0..n {
            self.next_back()?;
        }
        self.next_back()
    }
}

impl ExactSizeIterator for Count {}

fn count(back: u32) -> Count {
    Count { front: 0, back }
}

fn ds() -> Vec<D> {
    (1..=5).map(D).collect()
}

fn sum(sum: u32, d: D) -> u32 {
    sum * 10 + d.0
}

fn folded_back<I: DoubleEndedIterator<Item = D>>(it: I) -> u32 {
    it.rfold(0, sum)
}

fn second_back<I: DoubleEndedIterator<Item = D>>(mut it: I) -> Option<u32> {
    it.nth_back(1).map(|d| d.0)
}

struct Nums {
    front: u32,
    back: u32,
}

impl Iterator for Nums {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.front == self.back {
            return None;
        }
        self.front += 1;
        Some(self.front)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = (self.back - self.front) as usize;
        println!("size_hint {left}");
        (left, Some(left))
    }
}

impl DoubleEndedIterator for Nums {
    fn next_back(&mut self) -> Option<u32> {
        if self.front == self.back {
            return None;
        }
        self.back -= 1;
        Some(self.back + 1)
    }

    fn nth_back(&mut self, n: usize) -> Option<u32> {
        println!("nth_back {n}");
        for _ in 0..n {
            self.next_back()?;
        }
        self.next_back()
    }
}

impl ExactSizeIterator for Nums {}

fn nums(back: u32) -> Nums {
    Nums { front: 0, back }
}

fn numbers_folded_back<I: DoubleEndedIterator<Item = u32>>(it: I) -> u32 {
    it.rfold(0, |n, x| n * 10 + x)
}

fn numbers_second_back<I: DoubleEndedIterator<Item = u32>>(mut it: I) -> Option<u32> {
    it.nth_back(1)
}

fn numbers_found_back<I: DoubleEndedIterator<Item = u32>>(mut it: I) -> Option<u32> {
    it.rfind(|x| *x < 3)
}

fn main() {
    println!("{}", count(5).step_by(2).rfold(0, sum));
    let found = count(5).step_by(2).rfind(|d| d.0 < 3).map(|d| d.0);
    println!("{found:?}");
    println!("{}", count(5).take(4).step_by(2).rfold(0, sum));
    println!("{}", count(5).fuse().step_by(2).rfold(0, sum));
    let indexed = count(5)
        .enumerate()
        .step_by(2)
        .rfold(0, |n, (i, d)| n * 10 + i as u32 * d.0);
    println!("{indexed}");
    println!("{}", count(5).skip(1).step_by(2).rfold(0, sum));
    println!("{}", count(9).step_by(2).step_by(2).rfold(0, sum));
    println!("{}", ds().into_iter().step_by(2).step_by(2).rfold(0, sum));
    let reversed = ds().into_iter().rev().step_by(2).step_by(2).rfold(0, sum);
    println!("{reversed}");
    println!("{}", folded_back(count(5).step_by(2)));
    println!("{}", folded_back(ds().into_iter().step_by(2)));
    println!("{:?}", second_back(count(7).step_by(2)));
    println!("{:?}", second_back(ds().into_iter().step_by(2)));
    println!("{}", nums(5).step_by(2).rfold(0, |n, x| n * 10 + x));
    println!("{:?}", nums(5).step_by(2).rfind(|x| *x < 3));
    println!("{}", numbers_folded_back(nums(5).step_by(2)));
    println!("{:?}", numbers_second_back(nums(7).step_by(2)));
    println!("{:?}", numbers_found_back(nums(5).step_by(2)));
    println!("{}", numbers_folded_back(nums(5).fuse().step_by(2)));
    println!("{}", numbers_folded_back(nums(9).step_by(2).step_by(2)));
}
