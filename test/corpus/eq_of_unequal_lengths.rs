// `eq()` of two `TrustedLen` iterators of unequal lengths is `false` with
// no item stepped or compared, as std's `SpecIterEq` asks their lengths
// first: what each holds dropped as it's dropped, the other's first (ADR
// 0992). Of equal lengths, each pair compared and dropped in turn.
#[derive(PartialEq)]
struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn main() {
    let same = vec![D(1), D(2), D(3)].into_iter().eq(vec![D(9), D(2)]);
    println!("same {same}");
    let same = vec![D(4), D(5)].into_iter().eq(vec![D(4), D(6), D(7)]);
    println!("same {same}");
    let same = vec![D(8)].into_iter().eq(vec![D(8), D(10)]);
    println!("same {same}");
    let ne = vec![D(11), D(12)].into_iter().ne(vec![D(11)]);
    println!("ne {ne}");
    let same = vec![D(13), D(14)].into_iter().eq(vec![D(13), D(15)]);
    println!("same {same}");
    println!("{} {}", (1..4u32).eq(1..3u32), [1u32, 2].iter().eq([1u32, 2, 3].iter()));
}
