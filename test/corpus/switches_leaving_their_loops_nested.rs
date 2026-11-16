// A `match` of constants inside a loop, whose arm leaves the loop, inside
// another such loop's arm: JS's `break` in a `switch` leaves the `switch`,
// so each loop's exit is made its own, by tidy's `continue`s and a `break`
// after the `switch`, by an `if` taken first, or, while a `switch` of
// guarded arms is one, a label of tidy's, `loop`, the inner's `loop$1`,
// apart from every label around it and in it (#1969's rule; "names are
// ids", ADR 1150). Each `break` leaves its own loop, as the Rust's does.
fn scan(rows: &[Vec<u8>]) -> u32 {
    let mut total = 0;
    for row in rows {
        match row.len() {
            0 => break,
            1 => continue,
            2 => total += 100,
            _ => {
                for &x in row {
                    match x {
                        0 => break,
                        9 => return total + 1000,
                        5 => continue,
                        _ => total += u32::from(x),
                    }
                }
            }
        }
    }
    total
}

// Where the arms that stay run what's after the `match`, each loop's exit
// is its label's: the outer's and the inner's apart.
fn weighed(rows: &[Vec<u8>]) -> u32 {
    let mut total = 0;
    for row in rows {
        match row.len() {
            0 => break,
            1 => total += 1,
            2 => total += 2,
            3 => total += 3,
            _ => {
                for &x in row {
                    match x {
                        0 => break,
                        1 => total += 10,
                        2 => total += 20,
                        3 => total += 30,
                        _ => total += u32::from(x),
                    }
                    total += 1;
                }
            }
        }
        total *= 2;
    }
    total
}

// Of guarded arms, a loop of them inside another's.
fn guarded(rows: &[Vec<u8>], cap: u8) -> u32 {
    let mut total = 0;
    for row in rows {
        match row.len() {
            0 => break,
            n if n > usize::from(cap) => total += 1000,
            1 => total += 1,
            2 => total += 2,
            _ => {
                for &x in row {
                    match x {
                        0 => break,
                        x if x > cap => total += 100,
                        1 => total += 10,
                        2 => total += 20,
                        _ => total += u32::from(x),
                    }
                    total += 1;
                }
            }
        }
        total *= 2;
    }
    total
}

fn main() {
    println!("{}", scan(&[vec![1, 2, 3], vec![7], vec![4, 0, 4], vec![], vec![1, 1, 1]]));
    println!("{}", scan(&[vec![1, 5, 2], vec![3, 4], vec![2, 9, 2]]));
    println!("{}", scan(&[vec![6, 6, 6], vec![1, 2], vec![0, 0, 0]]));
    println!("{}", weighed(&[vec![1, 2, 0, 7], vec![3], vec![2, 2], vec![], vec![5]]));
    println!("{}", weighed(&[vec![1, 3, 5, 0, 2], vec![9, 9, 9, 9, 9], vec![1, 0], vec![4, 4, 4, 4]]));
    println!("{}", guarded(&[vec![1, 7, 2, 0, 9], vec![3, 3, 3], vec![1], vec![], vec![2]], 5));
    println!("{}", guarded(&[vec![9, 9, 9, 9, 9, 9, 9], vec![1, 2, 3, 4], vec![6]], 3));
}
