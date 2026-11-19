// A `cycle()` of a map whose closure counts its calls: each turn steps a
// clone of the map as it was made, its closure's count with it, so each
// turn counts from where the original's was, as std's `Cycle` clones its
// `orig` (ADR 0525).
fn main() {
    let mut calls = 0;
    let counted = [1, 2].iter().map(move |x| {
        calls += 1;
        x * 10 + calls
    });
    println!("{:?}", counted.cycle().take(5).collect::<Vec<_>>());
}
