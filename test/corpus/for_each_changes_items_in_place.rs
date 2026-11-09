// What a `for_each` closure changes of the items it's given `&mut`, as a
// `for` loop's body does: a tuple's part its pattern binds, `|(a, b)|` of
// `iter_mut()`'s items, changes the tuple in place, so `[(1, 'a'); 3]
// .to_vec()`'s items are three, not one item thrice; and a row its
// parameter is, `row.pop()` of a table's `iter_mut()`, may be shorter, so
// `board[0][2]` is checked, out of bounds, as where a function's given
// one, `for_each(pop_row)` (#1076, the metamorphic net's
// `for-each` rewrite of `enumerate_mut_patterns` and
// `table_rows_kept_by_views`).
fn shortened(width: usize) -> u32 {
    let mut board = vec![vec![1u32; width]; 2];
    board.iter_mut().for_each(|row| {
        row.pop();
    });
    board[0][width - 1]
}

fn pop_row(row: &mut Vec<u32>) {
    row.pop();
}

fn popped(width: usize) -> u32 {
    let mut board = vec![vec![1u32; width]; 2];
    board.iter_mut().for_each(pop_row);
    board[1][width - 1]
}

fn main() {
    let item = [(1, 'a'); 3];
    let mut v = item.to_vec();
    v.iter_mut().enumerate().skip(1).for_each(|(i, (a, b))| {
        *a *= i as i32 + 1;
        *b = 'z';
    });
    println!("{v:?}");
    println!("{}", std::panic::catch_unwind(|| shortened(3)).is_err());
    println!("{}", std::panic::catch_unwind(|| popped(2)).is_err());
}
