// A flatten of a fused source, a `Vec`'s items or a `take` of them, keeps
// it and its function until it's dropped, as std's `Fuse` of a
// `FusedIterator` does: with std's iterator structs, the compiled `Fuse`
// was told the `Map` it holds isn't fused, of its source a caller says is,
// and dropped both at its first `None` (#2243); here, the source was
// dropped at its end whatever it was (#2242).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let g = D(86);
    let mut f = vec![D(1), D(2)].into_iter().flat_map(move |d| {
        let _g = &g;
        vec![D(d.0 * 10)]
    });
    while let Some(x) = f.next() {
        println!("got {}", x.0);
    }
    println!("ended, held");
    drop(f);
    let g = D(87);
    let mut f = vec![D(3), D(4), D(5)].into_iter().take(1).flat_map(move |d| {
        let _g = &g;
        vec![D(d.0 * 10)]
    });
    while let Some(x) = f.next() {
        println!("got {}", x.0);
    }
    println!("taken ended, held");
    drop(f);
    let mut f = vec![D(6), D(7), D(8)]
        .into_iter()
        .take_while(|d| d.0 < 7)
        .flat_map(|d| vec![D(d.0 * 10)]);
    let n = f.by_ref().count();
    println!("counted {n}, held");
    drop(f);
}
