// Generic code's `collect()` into an `Option` or a `Result` drops the
// iterator it's given once, as std's `GenericShunt` does where `collect`
// returns: the crate's iterator with a `Drop`, of items that drop nothing,
// was closed by the collect and dropped after it too (#2561); of items that
// drop, closed once though two consumers close it (`closing-closes-again`).
#[derive(Debug)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop D{}", self.0);
    }
}

struct Counter(u32);

impl Iterator for Counter {
    type Item = Option<u32>;
    fn next(&mut self) -> Option<Option<u32>> {
        self.0 += 1;
        match self.0 {
            1 | 2 => Some(Some(self.0)),
            3 => Some(None),
            _ => None,
        }
    }
}

impl Drop for Counter {
    fn drop(&mut self) {
        println!("drop Counter at {}", self.0);
    }
}

struct Checked(u32);

impl Iterator for Checked {
    type Item = Result<u32, u32>;
    fn next(&mut self) -> Option<Result<u32, u32>> {
        self.0 += 1;
        match self.0 {
            1 | 2 => Some(Ok(self.0)),
            3 => Some(Err(3)),
            _ => None,
        }
    }
}

impl Drop for Checked {
    fn drop(&mut self) {
        println!("drop Checked at {}", self.0);
    }
}

struct Held(Vec<Option<D>>);

impl Iterator for Held {
    type Item = Option<D>;
    fn next(&mut self) -> Option<Option<D>> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.remove(0))
        }
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        println!("drop Held, {} left", self.0.len());
    }
}

fn options<I: Iterator<Item = Option<u32>>>(items: I) -> Option<Vec<u32>> {
    items.collect()
}

fn results<I: Iterator<Item = Result<u32, u32>>>(items: I) -> Result<Vec<u32>, u32> {
    items.collect()
}

fn firsts<I: Iterator<Item = Option<u32>>>(items: I) -> Option<Vec<u32>> {
    items.take(2).collect()
}

fn owned<I: Iterator<Item = Option<D>>>(items: I) -> Option<Vec<D>> {
    items.collect()
}

fn checked<I: Iterator<Item = Result<D, D>>>(items: I) -> Result<Vec<D>, D> {
    items.collect()
}

fn main() {
    println!("{:?}", options(Counter(0)));
    println!("{:?}", firsts(Counter(0)));
    println!("{:?}", results(Checked(0)));
    println!("{:?}", owned(Held(vec![Some(D(1)), Some(D(2))])));
    println!("{:?}", owned(Held(vec![Some(D(3)), None, Some(D(4))])));
    println!("{:?}", owned(vec![Some(D(5)), None, Some(D(6))].into_iter()));
    println!("{:?}", checked(vec![Ok(D(7)), Err(D(8)), Ok(D(9))].into_iter()));
}
