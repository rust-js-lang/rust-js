// A flatten of a source that isn't a `FusedIterator`, the crate's own
// iterator boxed, drops it at its first `None` with its function, as
// std's `Fuse` drops its `Map` there, where the function dropped as the
// flatten did; one of a fused source keeps both until it's dropped, as
// `Fuse`'s specialization does (#2242).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Every {
    at: u32,
    end: u32,
}

impl Drop for Every {
    fn drop(&mut self) {
        println!("drop src");
    }
}

impl Iterator for Every {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.at == self.end {
            return None;
        }
        self.at += 1;
        Some(self.at - 1)
    }
}

fn main() {
    let g = D(90);
    let mut f = Box::new(Every { at: 1, end: 3 }).flat_map(move |x| {
        let _g = &g;
        vec![D(x * 10)]
    });
    while let Some(x) = f.next() {
        println!("got {}", x.0);
    }
    println!("boxed ended, held");
    drop(f);
    let g = D(91);
    let mut f = Box::new(Every { at: 1, end: 4 }).take(2).flat_map(move |x| {
        let _g = &g;
        vec![D(x * 10), D(x * 10 + 1)]
    });
    let first = f.next().map(|d| d.0);
    println!("first {first:?}");
    let rest: u32 = f.by_ref().map(|d| d.0).sum();
    println!("taken ended, held {rest}");
    drop(f);
    let mut f = Box::new(Every { at: 1, end: 3 }).flat_map(|x| vec![D(x * 10)]);
    let n = f.by_ref().count();
    println!("counted {n}, held");
    drop(f);
}
