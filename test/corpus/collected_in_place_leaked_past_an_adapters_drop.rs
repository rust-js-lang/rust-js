// A collect into a `Vec` in place of a `map(f)` whose `f` holds what
// panics as it's dropped at the collect's end: what it collected is the
// collect's result by then, past std's in-place guard, which rustc leaks
// as a later drop unwinds (rust-lang/rust#47949, #843).
use std::panic::catch_unwind;

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 80 {
            panic!("drop");
        }
    }
}

fn main() {
    let r = catch_unwind(|| {
        let h = D(80);
        let v: Vec<D> = vec![D(1), D(2)]
            .into_iter()
            .map(move |d| {
                let _h = &h;
                D(d.0 + 10)
            })
            .collect();
        println!("mapped {}", v.len());
    });
    println!("{}", r.is_err());
}
