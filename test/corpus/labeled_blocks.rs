// A labeled block: `break 'name value` leaves it early with its value, from
// any depth, a loop's inside it included, as early returns of a function
// that isn't one.

fn grade(score: i32) -> &'static str {
    'graded: {
        if score < 0 {
            break 'graded "invalid";
        }
        if score >= 90 {
            break 'graded "A";
        }
        "B or below"
    }
}

fn first_even(rows: &[Vec<i32>]) -> Option<(usize, i32)> {
    let found = 'search: {
        for (i, row) in rows.iter().enumerate() {
            for &x in row {
                if x % 2 == 0 {
                    break 'search Some((i, x));
                }
            }
        }
        None
    };
    found
}

fn main() {
    println!("{} {} {}", grade(-1), grade(95), grade(50));
    println!("{:?} {:?}", first_even(&[vec![1, 3], vec![5, 8, 10]]), first_even(&[vec![1]]));

    // As a statement, leaving it without a value, and one inside another.
    let mut log = Vec::new();
    'outer: {
        log.push("start");
        'inner: {
            if log.len() == 1 {
                break 'inner;
            }
            log.push("not reached");
        }
        log.push("after inner");
        if !log.is_empty() {
            break 'outer;
        }
        log.push("not reached either");
    }
    println!("{:?}", log);

    // A loop inside one, its own `break` and `continue` still the loop's.
    let total = 'sum: {
        let mut sum = 0;
        for n in 1.. {
            if n % 2 == 0 {
                continue;
            }
            if n > 9 {
                break;
            }
            sum += n;
            if sum > 100 {
                break 'sum -1;
            }
        }
        sum
    };
    println!("{}", total);
}
