// A `flatten()` asks the hint of the inner iterator it's in, std's
// `FlattenCompat::size_hint`, which `Filter::count` asks first and a
// collect asks for its room (#2207). Of inner iterators of the crate's
// with their own `size_hint`, that's the crate's code, which runs where
// the flatten is made in place too: a `Vec`'s, an `Option`'s, boxed as
// `dyn`, inside a zip or another flatten, or given to generic code.

/// Counts up to `.1` from `.0`, from either end, with its own `size_hint`.
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

impl DoubleEndedIterator for Own {
    fn next_back(&mut self) -> Option<u32> {
        println!("own next_back");
        if self.0 == self.1 {
            return None;
        }
        self.1 -= 1;
        Some(self.1 + 1)
    }
}

fn evens<I: Iterator<Item = u32>>(it: I) -> usize {
    it.filter(|x| x % 2 == 0).count()
}

fn main() {
    let owns = vec![Own(0, 2), Own(0, 1)];
    let mut it = owns.into_iter().flatten();
    it.next();
    println!("of a vec: {}", it.filter(|x| x % 2 == 0).count());

    let mut it = vec![Own(0, 2), Own(0, 1)].into_iter().flatten();
    it.next();
    let rest: Vec<u32> = it.collect();
    println!("collected: {rest:?}");

    let mut it = Some(Own(0, 3)).into_iter().flatten();
    it.next();
    println!("of an option: {}", it.filter(|x| x % 2 == 0).count());

    let boxed: Vec<Box<dyn Iterator<Item = u32>>> = vec![Box::new(Own(0, 3))];
    let mut it = boxed.into_iter().flatten();
    it.next();
    println!("of boxed: {}", it.filter(|x| x % 2 == 0).count());

    let mut it = vec![Own(0, 2)].into_iter().flatten().zip(0..5);
    it.next();
    println!("zipped: {}", it.filter(|(x, _)| x % 2 == 0).count());

    let mut it = vec![vec![Own(0, 2)].into_iter().flatten()].into_iter().flatten();
    it.next();
    println!("of flattens: {}", it.filter(|x| x % 2 == 0).count());

    let mut it = vec![Own(0, 2), Own(0, 3)].into_iter().flatten();
    it.next_back();
    println!("from its back: {}", it.filter(|x| x % 2 == 0).count());

    let mut it = vec![Own(0, 2), Own(0, 1)].into_iter().flatten();
    it.next();
    println!("in generic code: {}", evens(it));

    let mut it = vec![Own(0, 2), Own(0, 1)].into_iter().flatten();
    it.next();
    println!("its hint: {:?}", it.size_hint());
}
