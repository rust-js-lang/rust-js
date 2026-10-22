// A `Result` steps as its `Ok` value or none, by `into_iter` too, a
// `&mut` one's a place; and orders `Ok` before `Err`, then by the value.
fn main() {
    let r: Result<i32, String> = Ok(3);
    println!("{:?}", r.into_iter().collect::<Vec<_>>());
    let mut m: Result<i32, ()> = Ok(1);
    for x in &mut m {
        *x += 1;
    }
    let rs = vec![Ok(1), Err("x"), Ok(3)];
    let total: i32 = rs.iter().flat_map(|r| r.into_iter()).sum();
    println!("{m:?} {total}");

    let mut v: Vec<Result<i32, String>> = vec![Err("b".into()), Ok(2), Err("a".into()), Ok(1)];
    v.sort();
    println!("{:?}", v);
    let a: Result<i32, i32> = Ok(5);
    let b: Result<i32, i32> = Err(0);
    println!("{} {} {:?} {:?}", a < b, a == Ok(5), a.cmp(&b), b.max(a));
    let f: Vec<Result<f64, ()>> = vec![Ok(f64::NAN), Ok(1.0)];
    println!("{:?} {:?}", f[0].partial_cmp(&f[1]), f[1].partial_cmp(&Err(())));
    let nested = [Ok(Some(2)), Ok(None), Err((1, "z"))];
    println!("{:?}", nested.iter().max());
}
