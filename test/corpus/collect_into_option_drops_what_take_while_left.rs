// A collect into an `Option` or a `Result` of a `take_while()` of a `Vec`'s
// items, which stops before their end, drops what's left where the collect's
// temporary is, as std's `GenericShunt` does, its source closed as it stops
// (#1004, #1979); and one a `None` stops, the rest dropped there too.
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let got: Option<Vec<u32>> = vec![D(1), D(2), D(3), D(4)]
        .into_iter()
        .take_while(|d| d.0 < 3)
        .map(|d| Some(d.0))
        .collect();
    println!("{got:?}");
    let ok: Result<Vec<u32>, String> = vec![D(5), D(6), D(7)]
        .into_iter()
        .take_while(|d| d.0 < 6)
        .map(|d| Ok(d.0))
        .collect();
    println!("{ok:?}");
    let none: Option<Vec<u32>> = vec![D(8), D(9), D(10)]
        .into_iter()
        .map(|d| if d.0 == 9 { None } else { Some(d.0) })
        .collect();
    println!("{none:?}");
    println!("end");
}
