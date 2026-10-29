// What an owning iterator gives, of an array or a `Vec` given it by value,
// nothing else holds, so it's changed in place uncopied (#141); an array
// still read after its loop is copied as it's given, so it reads as it was.
// A copy each, `copied()`'s, is its own too; `repeat`'s, of one value, and
// one through a reference it gives are copied.

fn bump(p: &mut (i32, i32)) {
    p.0 += 1;
}

fn main() {
    for mut p in [(0, 5), (7, 0)] {
        bump(&mut p);
        println!("{p:?}");
    }
    let kept = [(1, 1), (2, 2)];
    for mut p in kept {
        bump(&mut p);
        println!("{p:?}");
    }
    println!("{kept:?}");
    let held = vec![[3, 3], [4, 4]];
    let copy = held.clone();
    for mut row in held {
        row[0] = 9;
        println!("{row:?}");
    }
    println!("{copy:?}");
    let a = (5, 5);
    let pair = vec![a, a];
    for mut p in pair {
        bump(&mut p);
        println!("{p:?}");
    }
    println!("{a:?}");
    let mut grid = [[0u8; 2]; 2];
    for (i, mut row) in grid.into_iter().enumerate() {
        row[1] = i as u8 + 1;
        println!("{row:?}");
    }
    grid[0][0] = 7;
    println!("{grid:?}");
    // Of a borrowing iterator's, or through a reference it gives: copied.
    let seen = [(6, 6), (8, 8)];
    for mut p in seen.iter().copied() {
        bump(&mut p);
        println!("{p:?}");
    }
    let refs = vec![&seen[0], &seen[1]];
    for &(mut p) in refs {
        bump(&mut p);
        println!("{p:?}");
    }
    println!("{seen:?}");
    for mut p in std::iter::repeat(seen[0]).take(2) {
        bump(&mut p);
        println!("{p:?}");
    }
}
