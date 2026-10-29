// A `while let` whose `if` the program wrote does something before its
// `break`: that `if` as written, `if (n === 0) { log.push(..); break; }`,
// and the loop's test its `Some`, `while ((n = counts[i]) != null)`, where
// MIR's one way out of both made `while (true) { .. if (n != null) { if (n
// === 0) { .. } else { ..; continue; } } break; }`. The same tests in the
// same order: a missing count and a zero both end the loop, the zero after
// its line is logged, as in Rust.
fn until_zero(counts: &[u32], log: &mut Vec<String>) -> u32 {
    let mut i = 0;
    let mut total = 0;
    while let Some(&n) = counts.get(i) {
        if n == 0 {
            log.push(format!("zero at {i}"));
            break;
        }
        total += n;
        i += 1;
    }
    total
}

// What it does before its `break` declares a name what goes on does too:
// each in its own block, as written.
fn until_zero_at(counts: &[u32], log: &mut Vec<String>) -> usize {
    let mut i = 0;
    while let Some(&n) = counts.get(i) {
        if n == 0 {
            let at = i * 2;
            log.push(format!("zero, twice {at}"));
            break;
        }
        let at = i + 1;
        log.push(format!("{n} at {at}"));
        i += 1;
    }
    i
}

// Of an `||`, which MIR tests as `if (n <= 9 && n !== 3) { i++; continue;
// } log.push(..)`: the `if` as written too.
fn until_big(counts: &[u32], log: &mut Vec<String>) -> usize {
    let mut i = 0;
    while let Some(&n) = counts.get(i) {
        if n > 9 || n == 3 {
            log.push(format!("big {n}"));
            break;
        }
        i += 1;
    }
    i
}

// An `else` the program wrote, then its `break`: as written.
fn until_zero_else(counts: &[u32], log: &mut Vec<String>) -> usize {
    let mut i = 0;
    while let Some(&n) = counts.get(i) {
        if n == 0 {
            log.push(format!("zero at {i}"));
        } else {
            i += 1;
            continue;
        }
        break;
    }
    i
}

// In another `if`, after what the loop does first.
fn until_even(counts: &[u32], log: &mut Vec<String>) -> usize {
    let mut i = 0;
    while let Some(&n) = counts.get(i) {
        i += 1;
        if n > 9 {
            if n % 2 == 0 {
                log.push(format!("even {n}"));
                break;
            }
            log.push(format!("odd {n}"));
        }
    }
    i
}

fn main() {
    for counts in [&[1, 2, 0, 4][..], &[3, 4], &[], &[0]] {
        let mut log = Vec::new();
        let total = until_zero(counts, &mut log);
        println!("{total} {log:?}");
        let at = until_zero_at(counts, &mut log);
        println!("{at} {log:?}");
        let at = until_zero_else(counts, &mut log);
        println!("{at} {log:?}");
    }
    for counts in [&[1, 11, 12, 13][..], &[10], &[11, 13], &[1, 3, 4], &[]] {
        let mut log = Vec::new();
        let at = until_even(counts, &mut log);
        println!("{at} {log:?}");
        let at = until_big(counts, &mut log);
        println!("{at} {log:?}");
    }
}
