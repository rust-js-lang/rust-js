//@ run-abort
// A collect into a `Vec` of a `filter(p)` then a `map(f)`, whose `p` and
// `f` each hold what panics as it's dropped at the collect's end: the
// `map`'s drop drops its source, the `filter`, whose predicate's panic
// unwinds through the drop of `f`, whose panic aborts there. Of std's
// struct, its `close`, an arrow made outside a function's body, guards
// its cleanup where it's made (#938).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 >= 80 {
            panic!("drop {}", self.0);
        }
    }
}

fn main() {
    let r = std::panic::catch_unwind(|| {
        let h = D(80);
        let g = D(81);
        let v: Vec<D> = vec![D(1), D(2), D(3)]
            .into_iter()
            .filter(move |d| {
                let _g = &g;
                d.0 != 2
            })
            .map(move |d| {
                let _h = &h;
                D(d.0 + 10)
            })
            .collect();
        println!("mapped {}", v.len());
    });
    println!("{}", r.is_err());
}
