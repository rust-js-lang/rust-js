//! A const-generic function the library exports, called by its consumer
//! with lengths of its own, and by itself.
pub fn sum_n<const N: usize>(a: [i32; N]) -> i32 {
    let mut s = 0i32;
    for x in a {
        s = s.wrapping_add(x);
    }
    s
}

pub fn report() -> String {
    format!("{} {}", sum_n([1, 2, 3]), sum_n([i32::MAX, 1]))
}
