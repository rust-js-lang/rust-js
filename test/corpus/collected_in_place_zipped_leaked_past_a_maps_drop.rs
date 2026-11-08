// A collect into a `Vec` in place of a `zip` with a range then a `map(f)`
// whose `f` holds what panics as it's dropped at the collect's end: a
// zip's `as_inner` reaches its first side, the `Vec`'s `IntoIter`, whose
// rest alone std's in-place guard drops; what it collected is leaked as
// the adapters' drop unwinds (#852).
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
        let v: Vec<D> = vec![D(1), D(2)]
            .into_iter()
            .zip(0..5u32)
            .map(move |(d, i)| {
                let _h = &h;
                D(d.0 + i)
            })
            .collect();
        println!("mapped {}", v.len());
    });
    println!("{}", r.is_err());
}
