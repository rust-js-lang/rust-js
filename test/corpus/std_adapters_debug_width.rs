// std's adapters' `Debug` given a width pads each field, as native Rust's
// derived `Debug` hands its `Formatter` on (#1164).
fn main() {
    let v = [1, 2, 3];
    println!("[{:5?}]", v.iter().skip(1));
    println!("[{:<4?}]", v.iter().take(2));
    println!("[{:3?}]", v.iter().enumerate());
    println!("[{:?}]", v.iter().skip(1));
}
