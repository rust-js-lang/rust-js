// The crate's own iterator, given to generic code that collects it into the
// crate's `FromIterator`: `from_iter` is given what it holds closed, so
// it drops it by closing it, where it reads the iterator's fields (#1286).
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Count {
    n: u32,
    _loud: Loud,
}

impl Iterator for Count {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        (self.n < 3).then(|| {
            self.n += 1;
            self.n
        })
    }
}

struct Summed(u32);

impl FromIterator<u32> for Summed {
    fn from_iter<I: IntoIterator<Item = u32>>(items: I) -> Self {
        let mut sum = 0;
        let mut it = items.into_iter();
        while let Some(n) = it.next() {
            sum += n;
        }
        println!("summed {sum}");
        Summed(sum)
    }
}

// What stops early: what's left drops where `from_iter` returns.
struct First(Option<u32>);

impl FromIterator<u32> for First {
    fn from_iter<I: IntoIterator<Item = u32>>(items: I) -> Self {
        let first = items.into_iter().next();
        println!("first {first:?}");
        First(first)
    }
}

// What panics holding it: it drops as the panic unwinds.
struct Refused;

impl FromIterator<u32> for Refused {
    fn from_iter<I: IntoIterator<Item = u32>>(items: I) -> Self {
        let mut it = items.into_iter();
        let n = it.next();
        if n == Some(1) {
            panic!("refused {n:?}");
        }
        Refused
    }
}

fn collected<I: Iterator, B: FromIterator<I::Item>>(it: I) -> B {
    it.collect()
}

fn doubled<I: Iterator<Item = u32>, B: FromIterator<u32>>(it: I) -> B {
    it.map(|n| n * 2).collect()
}

fn main() {
    let count = |name| Count { n: 0, _loud: Loud(name) };
    let Summed(sum) = collected(count("all"));
    println!("{sum}");
    let First(first) = collected(count("first"));
    println!("{first:?}");
    let Summed(sum) = doubled(count("doubled"));
    println!("{sum}");
    let Summed(sum) = collected(vec![4, 5].into_iter());
    println!("{sum}");
    let refused = std::panic::catch_unwind(|| {
        let _: Refused = collected(count("refused"));
    });
    println!("{}", refused.is_err());
}
