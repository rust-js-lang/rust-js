// A precision of a `bool`'s `Display` or `Debug`, or of `()`'s `Debug`,
// cuts what std's `f.pad` writes, then pads it (#941).
fn main() {
    println!("[{:.2}]", true);
    println!("[{:<6.1}|{:>6.3}]", false, true);
    println!("[{:^7.0}]", true);
    println!("[{:.3?}]", false);
    println!("[{:.*}]", 1, true);
    println!("[{:.1?}]", ());
    println!("[{:>5.1?}]", ());
    let s = format!("{:-^9.4}", false);
    println!("{s}");
}
