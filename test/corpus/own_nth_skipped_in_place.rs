// An in-place `skip()` or `take()` of the crate's iterator with its own
// `nth`, of items with a destructor, is asked as std's `Skip` and `Take`
// ask it: `skip(n).nth(k)` is its source's `nth(n + k)`, through a `take()`
// or a compiled `fuse()` too, not each item stepped and dropped.

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Count(u32, u32);

impl Iterator for Count {
    type Item = Loud;

    fn next(&mut self) -> Option<Loud> {
        self.0 += 1;
        (self.0 <= self.1).then(|| Loud(self.0))
    }

    fn nth(&mut self, n: usize) -> Option<Loud> {
        println!("nth {n}");
        self.0 += n as u32;
        self.next()
    }
}

fn main() {
    println!("{:?}", Count(0, 8).skip(1).nth(2).map(|l| l.0));
    println!("{:?}", Count(0, 8).skip(2).nth(0).map(|l| l.0));
    println!("{:?}", Count(80, 86).fuse().take(5).skip(1).nth(1).map(|l| l.0));
    println!("{:?}", Count(70, 76).fuse().take(4).skip(2).next().map(|l| l.0));
    println!(
        "{:?}",
        Count(0, 6).fuse().take(3).chain(Count(10, 13)).nth(4).map(|l| l.0)
    );
    println!("{:?}", Count(20, 26).take(3).chain(Count(30, 33)).nth(1).map(|l| l.0));
    println!("{:?}", Count(40, 46).fuse().take(4).nth(2).map(|l| l.0));
    println!(
        "{:?}",
        Count(50, 56).skip(1).take(3).chain(Count(60, 63)).nth(4).map(|l| l.0)
    );
}
