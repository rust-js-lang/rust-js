// A collect into a `Vec` in place of a `filter(p)` then a `map(f)`, whose
// `p` holds what panics as it's dropped at the collect's end: std's
// in-place guard drops what's left of the `Vec`'s `IntoIter` alone, at the
// root its `SourceIter::as_inner` reaches; the adapters are dropped after,
// unguarded, so what it collected, the collect's result by then, is leaked
// (rust-lang/rust#47949, #852).
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
        let g = D(81);
        let v: Vec<D> = vec![D(1), D(2), D(3)]
            .into_iter()
            .filter(move |d| {
                let _g = &g;
                d.0 != 2
            })
            .map(|d| D(d.0 + 10))
            .collect();
        println!("kept {}", v.len());
    });
    println!("{}", r.is_err());
}
