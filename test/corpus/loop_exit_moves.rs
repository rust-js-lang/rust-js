// What's written after a loop runs after it, whatever MIR moved into its
// way out: a value the loop declared dropped first, a name shadowed after
// it, and loops of its own.

struct Guard(&'static str);

impl Drop for Guard {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

// A value the loop declares, dropped where its `break` leaves.
fn guarded(limit: u32) -> u32 {
    let mut n = 0;
    loop {
        let _g = Guard("round");
        n += 1;
        if n >= limit {
            break;
        }
        println!("round {}", n);
    }
    let after = n * 10;
    println!("after {}", after);
    after
}

// A name declared before the loop, shadowed after it.
fn shadowed(words: &[&str]) -> usize {
    let count = words.len();
    let mut i = 0;
    loop {
        if i == words.len() || words[i] == "stop" {
            break;
        }
        i += 1;
    }
    let count = count - i;
    println!("{} left", count);
    count
}

// A loop after the loop, with a `break` of its own.
fn two_loops(values: &[i32]) -> i32 {
    let mut it = values.iter();
    let mut first = 0;
    loop {
        match it.next() {
            Some(&v) if v > 0 => {
                first = v;
                break;
            }
            Some(_) => continue,
            None => break,
        }
    }
    let mut sum = 0;
    for &v in it {
        if v < 0 {
            break;
        }
        sum += v;
    }
    first * 100 + sum
}

// A labeled loop left from inside an inner one, then more.
fn grid_find(grid: &[[i32; 3]], want: i32) -> String {
    let mut at = None;
    'outer: loop {
        for (r, row) in grid.iter().enumerate() {
            for (c, &v) in row.iter().enumerate() {
                if v == want {
                    at = Some((r, c));
                    break 'outer;
                }
            }
        }
        break;
    }
    let text = match at {
        Some((r, c)) => format!("{} at {},{}", want, r, c),
        None => format!("{} missing", want),
    };
    text
}

// A name declared before the loop, set once in it, read after it.
fn set_in_loop(values: &[u32]) -> u32 {
    let found;
    let mut i = 0;
    loop {
        if values[i] % 7 == 0 {
            found = values[i];
            break;
        }
        i += 1;
    }
    println!("found {} at {}", found, i);
    found
}

// A name shadowed after the loop by one that doesn't read it.
fn reshadowed(limit: u32) -> String {
    let label = limit * 2;
    println!("label {}", label);
    let mut n = 0;
    loop {
        n += 3;
        if n > limit {
            break;
        }
    }
    let label = format!("n={}", n);
    label
}

// A `return` written in the loop stays where it's written.
fn first_big(values: &[i32]) -> i32 {
    let mut i = 0;
    loop {
        if values[i] > 10 {
            return values[i] * 2;
        }
        i += 1;
    }
}

// So does a `break` that gives the loop its value.
fn first_even(values: &[i32]) -> i32 {
    let mut i = 0;
    loop {
        if values[i] % 2 == 0 {
            break values[i] + 1;
        }
        i += 1;
    }
}

// A name the loop's test shadows, its early `break`s each its own.
fn shadow_in_loop(values: &[i32]) -> i32 {
    let mut it = values.iter();
    let mut total = 0;
    loop {
        let v = it.next();
        if let Some(&w) = v {
            let v = w * 2;
            if v > 3 {
                total += v;
                continue;
            }
        }
        break;
    }
    total
}

fn main() {
    println!("{}", guarded(3));
    println!("{}", shadowed(&["a", "b", "stop", "c"]));
    println!("{}", shadowed(&["a"]));
    println!("{}", two_loops(&[-1, 0, 4, 1, 2, -3, 9]));
    println!("{}", two_loops(&[-1]));
    let grid = [[1, 2, 3], [4, 5, 6]];
    println!("{}", grid_find(&grid, 5));
    println!("{}", grid_find(&grid, 7));
    println!("{}", set_in_loop(&[3, 5, 14, 21]));
    println!("{}", reshadowed(10));
    println!("{} {}", first_big(&[1, 12, 30]), first_even(&[3, 5, 8]));
    println!("{} {}", shadow_in_loop(&[2, 5, 1, 9]), shadow_in_loop(&[]));
}
