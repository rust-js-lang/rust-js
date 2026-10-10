// A const parameter is a value its caller gives (ADR 0107), and `[x; N]`
// an array of `N` of them, from MIR as from THIR.
fn filled<const N: usize>(x: u8) -> [u8; N] {
    [x; N]
}

fn total<const N: usize>(values: [u8; N]) -> usize {
    let mut sum = 0;
    for i in 0..N {
        sum += values[i] as usize;
    }
    sum * N
}

fn main() {
    let a: [u8; 3] = filled(2);
    println!("{:?} {}", a, total(a));
    println!("{}", total(filled::<5>(1)));
}
