// `{:e}` and `{:E}` as Rust writes them: the shortest digits that read back
// as the value, an `f32`'s its own, a 64-bit integer's every one, a negative
// zero's sign kept, and `inf` and `NaN` as they are, upper or not.

fn main() {
    println!("{:E} {:E} {:e} {:E}", f64::INFINITY, f64::NAN, f64::NEG_INFINITY, -0.0f64);
    println!("{:e} {:e} {:e} {:e} {:E}", 0i32, 1000i32, -1234i64, u64::MAX, 120u8);
    println!("{:e} {:e} {:e} {:e}", 0.1f32, 1.5e-7f32, f32::MAX, 16777216.0f32);
    println!("{:e} {:e} {:e} {:e}", 1e21f64, 5e-324f64, 0.3f64, -0.0f32);
    println!("[{:>12e}] [{:<10e}] [{:+e}] [{:010e}]", 1234.5f64, 7i32, 2.5f64, -1.5f64);
}
