// `Option`'s and `Result`'s other methods, as std's are (ADR 0326): their
// combinations, transposes, inspections, and an `Option`'s place filled.

#[derive(Debug, Default, Clone, PartialEq)]
struct Point {
    x: i32,
}

fn main() {
    let some: Option<i32> = Some(2);
    let none: Option<i32> = None;
    println!("{:?} {:?} {:?}", some.and(Some("x")), none.and(Some("x")), some.xor(none));
    println!("{:?} {:?} {:?}", some.xor(Some(3)), some.zip(Some('a')), some.zip(none));
    println!("{:?} {:?}", Some((1, "a")).unzip(), None::<(i32, &str)>.unzip());
    let ok: Option<Result<i32, String>> = Some(Ok(4));
    let err: Option<Result<i32, String>> = Some(Err("bad".to_string()));
    println!("{:?} {:?} {:?}", ok.transpose(), err.transpose(), None::<Result<i32, String>>.transpose());
    let mut seen = Vec::new();
    let kept = some.inspect(|x| seen.push(*x)).map(|x| x + 1);
    none.inspect(|x| seen.push(*x));
    println!("{kept:?} {seen:?} {} {}", some.map_or_default(|x| x * 10), none.map_or_default(|x| x * 10));
    println!("{:?} {:?}", some.as_slice(), none.as_slice());
    let name: Option<String> = Some("ann".to_string());
    println!("{:?}", name.as_deref().map(str::len));

    let mut count: Option<i32> = None;
    *count.get_or_insert(5) += 1;
    *count.get_or_insert(9) += 1;
    println!("{count:?}");
    let mut total: Option<i32> = None;
    *total.get_or_insert_with(|| 10) *= 2;
    let mut point: Option<Point> = None;
    point.get_or_insert_default().x += 3;
    let old = count.insert(1);
    *old += 100;
    println!("{count:?} {total:?} {point:?}");
    let mut maybe = Some(4);
    println!("{:?} {:?}", maybe.take_if(|x| *x > 5), maybe);
    println!("{:?} {:?}", maybe.take_if(|x| *x == 4), maybe);

    let good: Result<i32, String> = Ok(1);
    let bad: Result<i32, String> = Err("no".to_string());
    println!("{:?} {:?}", good.clone().and(Ok::<&str, String>("y")), bad.clone().and(Ok::<&str, String>("y")));
    println!("{:?} {:?}", good.clone().or(Ok::<i32, i32>(7)), bad.clone().or(Ok::<i32, i32>(7)));
    println!("{:?}", bad.clone().or_else(|e| if e == "no" { Ok::<i32, i32>(0) } else { Err(1) }));
    let nested: Result<Result<i32, String>, String> = Ok(Err("inner".to_string()));
    println!("{:?} {:?}", nested.flatten(), Ok::<Result<i32, String>, String>(Ok(3)).flatten());
    let mut log = Vec::new();
    let _ = good.clone().inspect(|x| log.push(format!("ok {x}"))).inspect_err(|e| log.push(format!("err {e}")));
    let _ = bad.clone().inspect(|x| log.push(format!("ok {x}"))).inspect_err(|e| log.push(format!("err {e}")));
    println!("{log:?}");
    println!("{:?} {:?}", good.iter().collect::<Vec<_>>(), bad.iter().count());
    println!("{} {}", good.clone().map_or_default(|x| x + 1), bad.clone().map_or_default(|x| x + 1));
    let listed: Result<Option<i32>, String> = Ok(Some(5));
    println!("{:?} {:?}", listed.transpose(), Ok::<Option<i32>, String>(None).transpose());
    let point_ref: Result<&Point, String> = Ok(&Point { x: 1 });
    let mut owned = point_ref.cloned().unwrap();
    owned.x = 9;
    let text: Result<String, i32> = Ok("hey".to_string());
    println!("{owned:?} {:?} {:?}", text.as_deref(), Ok::<&i32, ()>(&4).copied());
    println!("{}", unsafe { good.clone().unwrap_unchecked() });
    println!("{}", unsafe { bad.clone().unwrap_err_unchecked() });
}
