// A collect into a `Vec` in place, whose source's rest panics as it's
// dropped at the collect's end: what it collected is dropped as that
// unwinds, after the rest, as std's in-place guard drops it (#843).
use std::panic::catch_unwind;

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 5 {
            panic!("drop");
        }
    }
}

fn ds() -> Vec<D> {
    vec![D(1), D(2), D(3), D(4), D(5)]
}

fn main() {
    let r = catch_unwind(|| {
        let v: Vec<D> = ds().into_iter().take(3).collect();
        println!("took {}", v.len());
    });
    println!("{}", r.is_err());
    let r = catch_unwind(|| {
        let v: Vec<D> = ds().into_iter().take_while(|d| d.0 < 4).collect();
        println!("took while {}", v.len());
    });
    println!("{}", r.is_err());
    let r = catch_unwind(|| {
        let v: Vec<D> = ds().into_iter().map_while(|d| (d.0 < 3).then_some(d)).collect();
        println!("mapped while {}", v.len());
    });
    println!("{}", r.is_err());
    let r = catch_unwind(|| {
        let v: Vec<D> = ds()
            .into_iter()
            .scan(0, |n, d| {
                *n += 1;
                (*n < 3).then_some(d)
            })
            .collect();
        println!("scanned {}", v.len());
    });
    println!("{}", r.is_err());
}
