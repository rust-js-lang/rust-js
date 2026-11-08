// A collect into a `Vec` in place of an `enumerate().skip(1)` then a
// `map(f)` whose `f` holds what panics as it's dropped at the collect's
// end: only what's left of the source is dropped under std's in-place
// guard, reached through each adapter's `SourceIter::as_inner`; what it
// collected is leaked as the adapters' drop unwinds (#852).
use std::panic::catch_unwind;

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 >= 80 {
            panic!("drop");
        }
    }
}

fn main() {
    let r = catch_unwind(|| {
        let h = D(80);
        let v: Vec<D> = vec![D(1), D(2), D(3)]
            .into_iter()
            .enumerate()
            .skip(1)
            .map(move |(i, d)| {
                let _h = &h;
                D(d.0 + i as u32)
            })
            .collect();
        println!("mapped {}", v.len());
    });
    println!("{}", r.is_err());
}
