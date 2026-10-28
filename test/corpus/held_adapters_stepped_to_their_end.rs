// An adapter a variable holds, of the crate's own iterator that drops
// something, stepped to its end by `next()`, drops its source with the
// variable, where Rust drops it, not at its first `None` (#392): a zip's,
// a chain's back, a map's and a `take`'s; one moved into `drop`, one
// dropped at its scope's end, and one stepped past its `None` again;
// a function's parameter, and a generic body's iterator of what it's
// given, `FromIterator`'s.
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
        if self.n < 2 {
            self.n += 1;
            Some(self.n)
        } else {
            None
        }
    }
}

fn count(name: &'static str) -> Count {
    Count { n: 0, _loud: Loud(name) }
}

fn consume(mut it: impl Iterator<Item = (u32, u32)>) {
    while let Some((a, b)) = it.next() {
        println!("consumed {a} {b}");
    }
    println!("consume done");
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

fn main() {
    let mut zipped = count("zip").zip(10..);
    while let Some((a, b)) = zipped.next() {
        println!("{a} {b}");
    }
    println!("zip done");
    drop(zipped);

    {
        let mut chained = [7].into_iter().chain(count("chain"));
        while let Some(x) = chained.next() {
            println!("{x}");
        }
        println!("chain done");
    }

    {
        let mut mapped = count("map").map(|x| x * 3);
        while let Some(x) = mapped.next() {
            println!("{x}");
        }
        println!("map done {:?}", mapped.next());
    }

    let mut taken = count("take").take(5);
    while let Some(x) = taken.next() {
        println!("{x}");
    }
    println!("take done");

    consume(count("param").zip(20..));
    let mut stepped = count("moved").zip(30..);
    println!("{:?}", stepped.next());
    consume(stepped);
    let mut collected = count("collected").zip(40..);
    println!("{:?}", collected.next());
    println!("{:?}", collected.collect::<Vec<_>>());
    let Sum(sum) = count("from_iter").zip(50..).collect();
    println!("sum {sum}");
}
