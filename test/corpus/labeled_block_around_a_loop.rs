// A block the program labeled around a loop, `'done: { for &x in row {
// if x == 7 { break 'done; } } .. }`, in a loop of its own: MIR's
// structure put the block in the inner loop, `const iter = row.values();
// while (true) { const x = iter.next().value; done: { if (x != null) { ..
// continue; } .. } break; }`, as what follows the block leaves that loop.
// It's around the loop, as written, `done: { for (..) { if (x === 7) {
// break done; } } .. }`, and what follows it after it (#777).
pub fn labels(rows: &[Vec<u32>], out: &mut Vec<&'static str>) {
    for row in rows {
        let label;
        'done: {
            for &x in row {
                if x == 7 {
                    break 'done;
                }
            }
            if row.len() > 2 {
                label = "long";
            } else if row.is_empty() {
                break 'done;
            } else {
                label = "short";
            }
            out.push(label);
        }
        out.push("-");
    }
}

pub fn sums(rows: &[Vec<u32>], sums: &mut Vec<u32>) {
    for row in rows {
        let mut sum = 0;
        'row: {
            for &x in row {
                if x == 0 {
                    break 'row;
                }
            }
            sum = row.iter().sum();
        }
        sums.push(sum);
        sums.push(0);
    }
}

pub fn found(rows: &[Vec<u32>], seen: &mut Vec<u32>) -> Option<u32> {
    for row in rows {
        'scan: {
            for (i, &x) in row.iter().enumerate() {
                if x > 5 {
                    seen.push(i as u32);
                    break 'scan;
                }
                if x == 0 {
                    return Some(i as u32);
                }
            }
            seen.push(99);
        }
        seen.push(row.len() as u32);
    }
    None
}

fn main() {
    let rows = vec![vec![1, 2, 3], vec![7, 1], vec![], vec![4, 5], vec![1, 0, 7]];
    let mut out = Vec::new();
    labels(&rows, &mut out);
    println!("{out:?}");
    let mut s = Vec::new();
    sums(&rows, &mut s);
    println!("{s:?}");
    let mut seen = Vec::new();
    println!("{:?} {seen:?}", found(&rows, &mut seen));
    let mut seen = Vec::new();
    println!("{:?} {seen:?}", found(&[vec![9, 1], vec![2]], &mut seen));
}
