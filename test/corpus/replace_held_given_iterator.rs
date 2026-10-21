// `mem::replace` of a field holding an iterator given as it is: in a
// method, the field itself; in a closure, which captures the field, the
// iterator changed in place, the one it was kept of its kind.

struct Restart<I> {
    iter: I,
    orig: I,
}

impl<I: Iterator + Clone> Restart<I> {
    fn swap_ends(&mut self) {
        let orig = std::mem::replace(&mut self.orig, self.iter.clone());
        self.iter = orig;
    }
}

impl<I: Iterator + Clone> Iterator for Restart<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.iter.next().or_else(|| {
            let orig = std::mem::replace(&mut self.orig, self.iter.clone());
            self.iter = orig;
            self.iter.next()
        })
    }
}

fn main() {
    let ranges = Restart {
        iter: 0..2u32,
        orig: 5..7u32,
    };
    let all: Vec<u32> = ranges.take(5).collect();
    println!("{all:?}");
    let mut items = Restart {
        iter: vec!['a', 'b'].into_iter(),
        orig: vec!['x'].into_iter(),
    };
    items.next();
    items.swap_ends();
    let rest: String = items.by_ref().take(3).collect();
    let kept: Vec<char> = items.orig.collect();
    println!("{rest} {kept:?}");
}
