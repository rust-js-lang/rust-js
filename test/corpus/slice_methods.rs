// A slice's fills, copies, order checks, chunks and splits, as std's are
// (ADR 0324).

#[derive(Clone, Debug)]
struct Cell {
    n: u8,
}

fn main() {
    let mut nums = [0u8; 5];
    nums.fill(7);
    let mut next = 0;
    nums.fill_with(|| {
        next += 1;
        next
    });
    println!("{nums:?}");
    let mut cells = vec![Cell { n: 0 }; 2];
    cells.fill(Cell { n: 3 });
    cells[0].n = 9;
    println!("{cells:?}");

    let mut target = [0; 3];
    target.copy_from_slice(&[1, 2, 3]);
    let mut copies = vec![Cell { n: 0 }, Cell { n: 0 }];
    let source = [Cell { n: 1 }, Cell { n: 2 }];
    copies.clone_from_slice(&source);
    copies[0].n = 5;
    println!("{target:?} {copies:?} {source:?}");
    let mut left = [1, 2];
    let mut right = [3, 4];
    left.swap_with_slice(&mut right);
    let mut shifted = [1, 2, 3, 4, 5];
    shifted.copy_within(1..3, 3);
    println!("{left:?} {right:?} {shifted:?}");

    let sorted = [1, 2, 2, 5];
    println!("{} {}", [1.0, f64::NAN].is_sorted(), [0.5, 1.5].is_sorted());
    println!("{} {} {}", sorted.is_sorted(), [3, 1].is_sorted(), sorted.is_sorted_by(|a, b| a <= b));
    println!("{} {}", ["a", "bb", "c"].is_sorted_by_key(|s| s.len()), sorted.partition_point(|&x| x < 3));

    let items = [1, 2, 3, 4, 5, 6, 7];
    println!("{:?}", items.chunks_exact(3).collect::<Vec<_>>());
    println!("{:?}", items.chunks_exact(3).remainder());
    println!("{:?} {:?}", items.rchunks(3).collect::<Vec<_>>(), items.rchunks_exact(2).collect::<Vec<_>>());
    println!("{:?} {:?} {:?}", items.first_chunk::<2>(), items.last_chunk::<3>(), items.first_chunk::<9>());
    println!("{} {}", items.starts_with(&[1, 2]), items.ends_with(&[6]));
    println!("{:?} {:?}", items.strip_prefix(&[1]), items.strip_suffix(&[9]));
    println!("{:?}", [1, 2].repeat(3));
    println!("{:?}", items.split(|x| x % 3 == 0).collect::<Vec<_>>());
    println!("{:?}", [1, 3].split(|x| x % 3 == 0).collect::<Vec<_>>());
    println!("{:?}", items.splitn(2, |x| x % 2 == 0).collect::<Vec<_>>());
    println!("{:?}", items.rsplit(|&x| x == 4).collect::<Vec<_>>());
    println!("{:?}", items.split_inclusive(|x| x % 3 == 0).collect::<Vec<_>>());

    let mut words = vec!["banana", "fig", "apple"];
    let mut calls = 0;
    words.sort_by_cached_key(|w| {
        calls += 1;
        w.len()
    });
    println!("{words:?} {calls}");
    let mut bytes = b"Hello!".to_vec();
    println!("{} {:?}", bytes.is_ascii(), bytes.to_ascii_uppercase());
    bytes.make_ascii_lowercase();
    println!("{:?} {}", bytes, b" ab ".trim_ascii().len());
}
