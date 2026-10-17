// Early returns from inside nested matches and loops, each where it's made.

#[derive(Debug)]
enum Error {
    Empty,
    Negative(i32),
    TooBig { at: usize, value: i32 },
}

fn parse(s: &str) -> Result<i32, Error> {
    if s.is_empty() {
        return Err(Error::Empty);
    }
    let n: i32 = s.parse().map_err(|_| Error::Empty)?;
    if n < 0 {
        return Err(Error::Negative(n));
    }
    Ok(n)
}

// A `?` inside a loop inside a loop, and the inner loop's own `break`.
fn sum_rows(rows: &[&[&str]]) -> Result<i32, Error> {
    let mut total = 0;
    for (i, row) in rows.iter().enumerate() {
        let mut seen = 0;
        for cell in row.iter() {
            if *cell == "stop" {
                break;
            }
            let n = parse(cell)?;
            if n > 100 {
                return Err(Error::TooBig { at: i, value: n });
            }
            seen += n;
        }
        total += seen;
    }
    Ok(total)
}

// A match whose arms return, some through `?`, one falling to the end.
fn describe(token: Option<&str>, limit: i32) -> Result<String, Error> {
    let text = match token {
        None => return Err(Error::Empty),
        Some("zero") => "0".to_string(),
        Some(t) if t.starts_with('#') => {
            let n = parse(&t[1..])?;
            if n > limit {
                return Err(Error::TooBig { at: 0, value: n });
            }
            format!("#{}", n)
        }
        Some(t) => {
            let n = parse(t)?;
            let n = n * 2;
            n.to_string()
        }
    };
    Ok(format!("<{}>", text))
}

// A loop that only ends by returning, its `continue`s written.
fn first_even(values: &[&str]) -> Result<i32, Error> {
    let mut i = 0;
    loop {
        let Some(v) = values.get(i) else {
            return Err(Error::Empty);
        };
        i += 1;
        let n = parse(v)?;
        if n % 2 == 1 {
            continue;
        }
        return Ok(n);
    }
}

// A labeled loop broken from inside an inner one, a value made after it.
fn find_pair(grid: &[Vec<i32>], want: i32) -> Result<(usize, usize), Error> {
    let mut found = None;
    'rows: for (r, row) in grid.iter().enumerate() {
        for (c, &v) in row.iter().enumerate() {
            if v < 0 {
                return Err(Error::Negative(v));
            }
            if v == want {
                found = Some((r, c));
                break 'rows;
            }
            if v > want * 10 {
                break;
            }
        }
    }
    found.ok_or(Error::Empty)
}

// A `?` in a loop that holds a loop of its own, left by its own `break`,
// as it's given to what a call's arguments are pushed to.
fn call_args(tokens: &[&str]) -> Result<Vec<i32>, Error> {
    let mut it = tokens.iter().copied().peekable();
    match it.next() {
        Some("(") => {
            let mut args = Vec::new();
            if it.peek() != Some(&")") {
                args.push(parse(it.next().unwrap_or(""))?);
                while let Some(&",") = it.peek() {
                    it.next();
                    for _ in 0..2 {
                        if it.peek() != Some(&"_") {
                            break;
                        }
                        it.next();
                    }
                    args.push(parse(it.next().unwrap_or(""))?);
                }
            }
            if it.next() != Some(")") {
                return Err(Error::TooBig { at: args.len(), value: 0 });
            }
            Ok(args)
        }
        Some(t) => Err(Error::Negative(t.len() as i32)),
        None => Err(Error::Empty),
    }
}

fn main() {
    let rows: [&[&str]; 3] = [&["1", "2", "stop", "x"], &["3"], &["4", "5"]];
    println!("{:?}", sum_rows(&rows));
    println!("{:?}", sum_rows(&[&["1", ""]]));
    println!("{:?}", sum_rows(&[&["1"], &["200"]]));
    println!("{:?}", sum_rows(&[&["-4"]]));
    for t in [None, Some("zero"), Some("#5"), Some("#50"), Some("#"), Some("21"), Some("-1")] {
        println!("{:?} -> {:?}", t, describe(t, 10));
    }
    println!("{:?}", first_even(&["3", "5", "8", "9"]));
    println!("{:?}", first_even(&["3", "x"]));
    println!("{:?}", first_even(&["3"]));
    let grid = vec![vec![1, 2, 3], vec![40, 5, 6], vec![7, 8, 9]];
    println!("{:?}", find_pair(&grid, 6));
    println!("{:?}", find_pair(&grid, 3));
    println!("{:?}", find_pair(&grid, 100));
    println!("{:?}", find_pair(&[vec![1, -2]], 5));
    println!("{:?}", call_args(&["(", "1", ",", "_", "2", ",", "_", "_", "3", ")"]));
    println!("{:?}", call_args(&["(", "1", ",", "_", "_", "_", ")"]));
    println!("{:?}", call_args(&["(", ")"]));
    println!("{:?}", call_args(&["(", "1", "2"]));
    println!("{:?}", call_args(&["(", "x"]));
    println!("{:?}", call_args(&["add"]));
    println!("{:?}", call_args(&[]));
}
