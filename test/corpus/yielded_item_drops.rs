// A std trait's associated type, `<I as Iterator>::Item`, is given a drop
// where its caller's has one, whether its type parameter has one or not:
// an iterator with no destructor may give items with one, which a step
// that discards one drops; where it's the caller's own type parameter, or
// its own associated type, the caller's drop passed on. Where no caller's
// has one, it drops nothing, as a type parameter's does (ADR 0489).
struct Loud(u8);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Gen(u8);

impl Iterator for Gen {
    type Item = Loud;

    fn next(&mut self) -> Option<Loud> {
        self.0 += 1;
        if self.0 < 4 { Some(Loud(self.0)) } else { None }
    }
}

struct Make<T> {
    make: fn(u8) -> T,
    n: u8,
}

impl<T> Iterator for Make<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.n += 1;
        if self.n < 4 { Some((self.make)(self.n)) } else { None }
    }
}

struct Skipper<I> {
    inner: I,
}

impl<I: Iterator> Skipper<I> {
    fn skip(&mut self) {
        self.inner.next();
    }
}

fn second<I: Iterator>(mut iter: I) -> Option<I::Item> {
    iter.next();
    iter.next()
}

fn third<I: Iterator>(mut iter: I) -> Option<I::Item> {
    iter.next();
    iter.next();
    iter.next()
}

fn last_one<I: Iterator>(mut iter: I) -> Option<I::Item> {
    let mut last = iter.next();
    while let Some(next) = iter.next() {
        last = Some(next);
    }
    last
}

fn made_third<T, M: Iterator<Item = T>>(made: M) -> Option<T> {
    third(made)
}

fn relayed<J: Iterator>(iter: J) -> Option<J::Item> {
    last_one(iter)
}

fn counted<I: Iterator + Clone>(iter: I) -> usize {
    iter.clone().count()
}

fn main() {
    let mut skipper = Skipper { inner: Gen(0) };
    skipper.skip();
    println!("skipped");
    let kept = second(Gen(0));
    println!("kept {}", kept.map_or(0, |l| l.0));
    println!("{:?}", second(0..3));
    let made = made_third(Make { make: Loud, n: 0 });
    println!("made {}", made.map_or(0, |l| l.0));
    let relay = relayed(Gen(1));
    println!("relayed {}", relay.map_or(0, |l| l.0));
    println!("{}", counted(0..3));
    let loud = Loud(9);
    println!("{}", loud.0);
}
