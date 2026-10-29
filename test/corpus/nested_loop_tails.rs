// An outer loop's tail after an inner loop, as written, with no label the
// Rust lacks: React DOM's estimateBandwidth, `while (i < n) { let j = i +
// 1; while (j < n) { if (..) { break; } .. } i = j; count++; if (count >
// 10) { break; } }`, whose inner loop's way out MIR holds the outer turn's
// tail in, `continue bb1` its end (rd141c); React's reconcileChildrenArray,
// `while let .. && i < n { let Some(f) = .. else { ..; break; }; .. }` and
// then the function's tail; and its findCurrentFiberUsingSlowPath, a
// `while let` whose `break`s an `if !found { .. }` follows, in an arm of
// an outer loop's `if`, where MIR's block holds what's written after the
// inner loop.

fn average(times: &[f64], end: f64) -> f64 {
    let mut count = 0;
    let mut total = 0.0;
    let mut i = 0;
    while i < times.len() {
        let mut j = i + 1;
        while j < times.len() {
            if times[j] > end {
                break;
            }
            total += times[j];
            j += 1;
        }
        i = j;
        count += 1;
        if count > 10 {
            break;
        }
    }
    total / f64::from(count)
}

fn slot(old: u32, new: u32) -> Option<u32> {
    (old % 4 != 0).then_some(old + new)
}

fn reconcile(first: Option<u32>, children: &[u32], log: &mut Vec<u32>) -> u32 {
    let mut old = first;
    let mut i = 0;
    let mut next;
    while let Some(o) = &old
        && i < children.len()
    {
        if *o > 50 {
            next = old;
            old = None;
        } else {
            next = Some(o + 1);
        }
        let Some(made) = slot(old.unwrap_or(0), children[i]) else {
            // An empty slot: the slow path.
            if old.is_none() {
                old = next;
            }
            break;
        };
        log.push(made);
        old = next;
        i += 1;
    }

    if i == children.len() {
        // The end of the children.
        return 100 + i as u32;
    }

    let Some(old) = old else {
        // Only insertions left.
        return 200 + i as u32;
    };
    old
}

fn find(xs: &[u32], ys: &[u32], mut a: u32, mut b: u32) -> u32 {
    for _ in 0..4 {
        if a != b {
            a += 1;
        } else {
            let mut found = false;
            let mut k = 0;
            while let Some(&c) = xs.get(k) {
                if c == a {
                    found = true;
                    a = c + 1;
                    break;
                }
                k += 1;
            }
            if !found {
                k = 0;
                while let Some(&c) = ys.get(k) {
                    if c == a {
                        found = true;
                        a = c + 3;
                        break;
                    }
                    k += 1;
                }
                if !found {
                    return 0;
                }
            }
        }
        if a == 7 {
            return 7;
        }
    }
    a + b
}

fn main() {
    println!("{}", average(&[1.0, 2.0, 3.0, 9.0, 4.0, 5.0], 8.0));
    println!("{}", average(&[1.0; 40], 8.0));
    println!("{}", average(&[0.5], 8.0));
    let mut log = Vec::new();
    println!("{} {:?}", reconcile(Some(1), &[1, 2, 3], &mut log), log);
    println!("{} {:?}", reconcile(Some(2), &[5, 6, 7, 8], &mut log), log);
    println!("{} {:?}", reconcile(Some(60), &[5, 6], &mut log), log);
    println!("{} {:?}", reconcile(None, &[5], &mut log), log);
    println!("{}", find(&[1, 2, 3], &[9], 0, 0));
    println!("{}", find(&[90, 40], &[1, 2, 6, 30, 4], 1, 1));
    println!("{}", find(&[], &[], 120, 3));
    println!("{}", find(&[5, 4], &[], 4, 4));
}
