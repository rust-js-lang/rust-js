// A collect into a `Vec` in place of a `filter(p)` whose `p` holds what
// drops, `move |d| { let _g = &g; .. }`: where an item `p` doesn't keep
// panics as it's dropped, what the collect has gathered is std's
// `try_fold` accumulator, which the filter's fold closure has made what it
// gives by then, so it's leaked (rust-lang/rust#47949); then what's left
// of the source is dropped, then `p` with what it holds (#2056). Of a
// `filter` alone, and of a `filter` then a `map`.
use std::panic::catch_unwind;

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 2 {
            panic!("drop");
        }
    }
}

fn filtered() {
    let g = D(81);
    let v: Vec<D> = vec![D(1), D(2), D(3)]
        .into_iter()
        .filter(move |d| {
            let _g = &g;
            d.0 != 2
        })
        .collect();
    println!("kept {}", v.len());
}

fn filtered_then_mapped() {
    let g = D(82);
    let v: Vec<D> = vec![D(1), D(2), D(3)]
        .into_iter()
        .filter(move |d| {
            let _g = &g;
            d.0 != 2
        })
        .map(|d| D(d.0 + 10))
        .collect();
    println!("kept {}", v.len());
}

fn kept_all() {
    let g = D(83);
    let v: Vec<D> = vec![D(4), D(5)]
        .into_iter()
        .filter(move |d| {
            let _g = &g;
            d.0 != 2
        })
        .collect();
    println!("kept {}", v.len());
}

fn main() {
    println!("{}", catch_unwind(filtered).is_err());
    println!("{}", catch_unwind(filtered_then_mapped).is_err());
    kept_all();
}
