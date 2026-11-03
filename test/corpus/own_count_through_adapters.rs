// The crate's own `count`, asked as std asks it: of the iterator itself,
// through a `skip` (after its `nth(n - 1)`), a `fuse`, a `peekable`, a
// `chain`'s sides and an `enumerate`, and of generic code's; not through a
// `map`, a `take` or a `&mut`, whose `count` is std's (#505). Of one with a
// destructor, what it holds is its own `count`'s to drop, never again.
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Count(u32);

impl Iterator for Count {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 < 5 {
            self.0 += 1;
            Some(self.0)
        } else {
            None
        }
    }

    fn count(self) -> usize {
        println!("own count from {}", self.0);
        5 - self.0 as usize
    }
}

struct Counts(Vec<D>);

impl Iterator for Counts {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.remove(0))
        }
    }

    fn count(self) -> usize {
        println!("counts' own count");
        self.0.len()
    }
}

fn counted<I: Iterator>(it: I) -> usize {
    it.count()
}

fn skip_counted<I: Iterator>(it: I) -> usize {
    it.skip(1).count()
}

fn fuse_counted<I: Iterator>(it: I) -> usize {
    it.fuse().count()
}

fn ds() -> Vec<D> {
    vec![D(1), D(2), D(3)]
}

fn main() {
    println!("{}", Count(0).count());
    println!("{}", Count(0).skip(1).count());
    println!("{}", Count(0).skip(9).count());
    println!("{}", Count(0).fuse().count());
    println!("{}", Count(0).peekable().count());
    println!("{}", Count(0).chain(Count(3)).count());
    println!("{}", Count(0).enumerate().count());
    println!("{}", Count(0).map(|x| x).count());
    println!("{}", Count(0).take(2).count());
    println!("{}", Count(0).fuse().skip(1).count());
    println!("{}", counted(Count(0)));
    println!("{}", skip_counted(Count(0)));
    println!("{}", fuse_counted(Count(0)));
    println!("{}", counted(vec![1, 2].into_iter()));
    println!("{}", Counts(ds()).count());
    println!("{}", Counts(ds()).skip(1).count());
    println!("{}", Counts(ds()).skip(5).count());
    println!("{}", Counts(ds()).fuse().count());
    println!("{}", Counts(ds()).chain(Counts(ds())).skip(1).count());
    println!("{}", Counts(ds()).enumerate().count());
    let mut it = Count(0);
    it.next();
    println!("{}", it.by_ref().count());
}
