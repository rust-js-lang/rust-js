fn main() {
    let f = i32::MIN;
    println!("{} {} {}", f.wrapping_neg(), f.wrapping_abs(), 5i32.wrapping_neg());
}
