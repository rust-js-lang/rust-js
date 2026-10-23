// `sum()` and `product()` into an `Option` or a `Result` of numbers: the
// first `None` or `Err`, where stepping stops, or the numbers' own.
fn main() {
    let t = |x: i32| {
        println!("t{x}");
        x
    };
    let a: Option<i32> = [1, 2, 3].iter().map(|&x| if x < 3 { Some(t(x)) } else { None }).sum();
    let b: Option<i32> = [1, 2].iter().map(|&x| Some(t(x))).sum();
    let c: Option<u8> = [200u8, 100].iter().map(|&x| Some(x)).product::<Option<u8>>().or(Some(0));
    let d: Result<i64, String> = ["1", "2", "x", "4"].iter().map(|s| s.parse::<i64>().map_err(|e| e.to_string())).sum();
    let e: Result<f64, ()> = [1.5, 2.0].into_iter().map(Ok).product();
    let f: Option<u64> = std::iter::empty::<Option<u64>>().sum();
    let g: Option<i32> = (1..=4i32).map(|x| x.checked_mul(1000)).product();
    println!("{a:?} {b:?} {c:?} {d:?} {e:?} {f:?} {g:?}");
}
