// A comment before the `continue` an arm is, `1 => { // Skipped.
// continue; }`, is before that `continue`, in its `case` or its `if`
// (#574): MIR's test of the arm's pattern goes to the loop's start itself,
// no block of the arm's own placed where it's written. One before an arm
// that does more first, and a `continue` of a loop inside the arm, keep
// theirs.
pub fn switched(reason: i32, log: &mut Vec<i32>) {
    for k in 0..reason {
        match k {
            0 => log.push(0),
            // Above one.
            1 => {
                // Skipped.
                continue;
            }
            _ => log.push(k),
        }
        log.push(100);
    }
}

pub fn tested(reason: i32, log: &mut Vec<i32>) {
    for k in 0..reason {
        match k {
            3 => {
                // Three is skipped.
                continue;
            }
            _ => log.push(k),
        }
        log.push(200);
    }
}

pub fn after(reason: i32, log: &mut Vec<i32>) {
    for k in 0..reason {
        match k {
            0 => log.push(0),
            1 => {
                log.push(1);
                // Skipped after.
                continue;
            }
            _ => log.push(k),
        }
        log.push(300);
    }
}

pub fn inner(rows: &[&[i32]], log: &mut Vec<i32>) {
    for row in rows {
        match row.len() {
            0 => log.push(-1),
            1 => {
                for &x in row.iter() {
                    if x < 0 {
                        // A negative one.
                        continue;
                    }
                    log.push(x);
                }
                // The row's done.
                continue;
            }
            _ => log.push(row[0]),
        }
        log.push(400);
    }
}

fn main() {
    let mut log = Vec::new();
    switched(4, &mut log);
    tested(5, &mut log);
    after(3, &mut log);
    inner(&[&[], &[5], &[-2], &[7, 8]], &mut log);
    println!("{log:?}");
}
