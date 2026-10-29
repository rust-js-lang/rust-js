// A `break 'rows` the program wrote in an inner loop, of a `'rows: while
// let`: `rows: while ((row = rows[r]) != null) { .. while (..) { if (n ===
// 0) { ..; break rows; } .. } r++; }`, its label the program's and the
// loop its test, not `while (true) { const row = rows[r]; bb22: { if (row
// != null) { .. break bb22; .. continue; } } break; }`, a block MIR made
// of the turn whose end is the loop's way out (uz3).

fn first_zero(rows: &[Vec<u32>], log: &mut Vec<String>) -> u32 {
    let mut r = 0;
    let mut total = 0;
    'rows: while let Some(row) = rows.get(r) {
        let mut i = 0;
        while let Some(&n) = row.get(i) {
            if n == 0 {
                log.push(format!("zero at {r}, {i}"));
                break 'rows;
            }
            total += n;
            i += 1;
        }
        r += 1;
    }
    total
}

fn main() {
    let mut log = Vec::new();
    println!("{}", first_zero(&[vec![1, 2], vec![3, 0, 4], vec![5]], &mut log));
    println!("{}", first_zero(&[vec![1], vec![], vec![2]], &mut log));
    println!("{log:?}");
}
