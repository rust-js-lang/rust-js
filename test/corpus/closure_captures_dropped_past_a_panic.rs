// A closure's captures dropped in turn, the rest still dropped after one
// whose drop panics, as drop glue goes on past a panicking field: where
// it's made, moved to another variable, and as an iterator's fold drops
// the closures it holds (#846).
use std::panic::catch_unwind;

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 90 {
            panic!("drop");
        }
    }
}

fn main() {
    let r = catch_unwind(|| {
        let g = D(90);
        let h = D(80);
        let c = move || {
            let _g = &g;
            let _h = &h;
        };
        c();
    });
    println!("{}", r.is_err());
    let r = catch_unwind(|| {
        let g = D(90);
        let h = D(80);
        let c = move || {
            let _g = &g;
            let _h = &h;
        };
        let k = c;
        k();
    });
    println!("{}", r.is_err());
    let r = catch_unwind(|| {
        let h = D(80);
        let g = D(90);
        let total = vec![1u32, 2]
            .into_iter()
            .map(move |x| {
                let _h = &h;
                x + 1
            })
            .fold(0, move |a, x| {
                let _g = &g;
                a + x
            });
        println!("total {total}");
    });
    println!("{}", r.is_err());
}
