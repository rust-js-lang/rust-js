// Generic code's `last()` of an iterator, through a `skip`, a `fuse`, a
// `peekable` and a `chain`, asks the type's own `last`, the crate's where
// its impl writes one, as std's adapters pass it on; of one without, std's,
// its last item; and a `fuse`'s `nth` the crate's own `nth` (#496).
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

    fn nth(&mut self, n: usize) -> Option<u32> {
        println!("own nth {n}");
        self.0 += n as u32;
        self.next()
    }

    fn last(self) -> Option<u32> {
        println!("own last");
        Some(5)
    }
}

fn plain<I: Iterator>(it: I) -> Option<I::Item> {
    it.last()
}

fn skip<I: Iterator>(it: I) -> Option<I::Item> {
    it.skip(1).last()
}

fn fuse<I: Iterator>(it: I) -> Option<I::Item> {
    it.fuse().last()
}

fn peek<I: Iterator>(it: I) -> Option<I::Item> {
    it.peekable().last()
}

fn chain<I: Iterator>(a: I, b: I) -> Option<I::Item> {
    a.chain(b).skip(1).last()
}

fn mapped<I: Iterator<Item = u32>>(it: I) -> Option<u32> {
    it.map(|x| x + 1).last()
}

fn fuse_nth<I: Iterator>(it: I) -> Option<I::Item> {
    it.fuse().nth(1)
}

fn nth_of<I: Iterator>(mut it: I) -> Option<I::Item> {
    it.nth(1)
}

fn main() {
    println!("{:?}", plain(Count(0)));
    println!("{:?}", skip(Count(0)));
    println!("{:?}", fuse(Count(0)));
    println!("{:?}", peek(Count(0)));
    println!("{:?}", chain(Count(0), Count(3)));
    println!("{:?}", mapped(Count(0)));
    println!("{:?}", fuse_nth(Count(0)));
    println!("{:?}", plain(vec![1, 2, 3].into_iter()));
    println!("{:?}", skip(vec![1, 2, 3].into_iter()));
    println!("{:?}", skip([1, 2, 3].iter()));
    println!("{:?}", fuse([1, 2, 3].iter()));
    println!("{:?}", chain(vec![1].into_iter(), vec![2, 3].into_iter()));
    println!("{:?}", plain(None::<u8>.into_iter()));
    let m = std::collections::HashMap::from([(1, 'a')]);
    println!("{:?}", plain(m.keys()));
    println!("{:?}", nth_of("abc".chars()));
    println!("{:?}", nth_of(m.values()));
}
