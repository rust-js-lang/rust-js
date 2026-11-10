// The crate's own `from_iter` holds what generic code's `collect()` gives
// it, and drops it at its end, after its loop: not where it's stepped to
// its end, as JS's protocol, which generic code's `Iterator.from(it)` gave
// it, would close it (#1239).
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
        (self.n < 2).then(|| {
            self.n += 1;
            self.n
        })
    }
}

struct Sum(u32);

impl FromIterator<(u32, u32)> for Sum {
    fn from_iter<I: IntoIterator<Item = (u32, u32)>>(items: I) -> Self {
        let mut it = items.into_iter();
        let mut sum = 0;
        while let Some((a, b)) = it.next() {
            sum += a * b;
        }
        println!("summed {sum}");
        Sum(sum)
    }
}

fn collected<I: Iterator, B: FromIterator<I::Item>>(it: I) -> B {
    it.collect()
}

fn main() {
    let Sum(sum) = collected(Count { n: 0, _loud: Loud("zip") }.zip(50..));
    println!("sum {sum}");
    let Sum(sum) = collected(Count { n: 0, _loud: Loud("map") }.map(|n| (n, n)));
    println!("sum {sum}");
}
