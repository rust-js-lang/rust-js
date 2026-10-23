// {:?} of text, a str's, a String's or a char's, is written whole: std's
// Debug of it takes no precision, nor a width.
fn main() {
    println!("[{:.1?}] [{:5.2?}] [{:.0?}] [{:>8.3?}]", "hello", "ab", "x", "é\nz");
    println!("[{:.1?}] [{:5?}] [{:.2?}]", 'c', 'd', String::from("abc"));
    let p = 2;
    println!("[{:.p$?}] [{:.*?}]", "dynamic", 1, "star");
}
