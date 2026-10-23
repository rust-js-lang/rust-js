// round_ties_even(): a tie to the even integer, of f64s and f32s, a
// negative tie's sign kept, as Rust's is; round() a tie away from zero.
fn main() {
    let xs = [0.5, 1.5, 2.5, 3.5, -0.5, -1.5, -2.5, -3.5, 2.4, 2.6, -2.6, 1e16, f64::NAN, f64::INFINITY, -0.0];
    for x in xs {
        println!("{x:?} {:?} {:?}", x.round_ties_even(), x.round());
    }
    for x in [0.5f32, 2.5, -2.5, 4.5, 8388609.5, -0.4] {
        println!("{x:?} {:?} {:?}", x.round_ties_even(), x.round());
    }
}
