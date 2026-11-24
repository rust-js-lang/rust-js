// The crate's own iterator chained after a `Vec`'s, given generic code that
// asks its hint, `size_hint()` or a `collect()` that reserves by it: the
// chain is made knowing its hint, and its side of the crate's, which has
// no form that knows it, is given as it came, stepped by its own `next`
// and asked its own `size_hint`, never taken out of what steps it (#2262).

struct Own(u32, u32);

impl Iterator for Own {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        println!("own next");
        if self.0 == self.1 {
            return None;
        }
        self.0 += 1;
        Some(self.0)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.1 - self.0) as usize;
        println!("own size_hint {n}");
        (n, Some(n))
    }
}

fn hinted<I: Iterator>(it: I) -> (usize, Option<usize>) {
    it.size_hint()
}

fn gathered<I: Iterator<Item = u32>>(it: I) -> Vec<u32> {
    it.collect()
}

fn main() {
    println!("{:?}", hinted(vec![7].into_iter().chain(Own(0, 4))));
    println!("{:?}", hinted(Own(1, 3).chain(Own(0, 4))));
    let v = gathered(vec![7].into_iter().chain(Own(0, 4)).map(|x| x + 1));
    println!("{v:?} {}", v.capacity());
}
