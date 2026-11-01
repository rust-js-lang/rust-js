//! A const-generic function used only inside the library: its instances are
//! the library's, each of its own length.
fn sum_n<const N: usize>(a: [i32; N]) -> i32 {
    let mut s = 0i32;
    for x in a {
        s = s.wrapping_add(x);
    }
    s
}

pub fn report() -> String {
    format!("{} {}", sum_n([1, 2, 3]), sum_n([10, 20, 30, 40]))
}
