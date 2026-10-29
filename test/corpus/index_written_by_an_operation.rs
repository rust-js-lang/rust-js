// `v[i] = n + 1` writes an operation of values made before, which runs
// nothing between the item's place and its write: its index is read where
// it's written, `table[suffixIndex] = prefixIndex + 1`, not held (#353).
// Its checks, a division's by zero first, then the index's, panic in Rust's
// order.

fn pattern_table(word: &str) -> Vec<u32> {
    let p = word.as_bytes();
    let mut pattern_table = vec![0u32; p.len()];
    let mut prefix_index = 0;
    let mut suffix_index = 1;
    while suffix_index < p.len() {
        if p[prefix_index] == p[suffix_index] {
            pattern_table[suffix_index] = prefix_index as u32 + 1;
            suffix_index += 1;
            prefix_index += 1;
        } else if prefix_index == 0 {
            pattern_table[suffix_index] = 0;
            suffix_index += 1;
        } else {
            prefix_index = pattern_table[prefix_index - 1] as usize;
        }
    }
    pattern_table
}

fn shares(items: &mut [u32], at: usize, total: u32, parts: u32) {
    items[at] = total / parts;
}

fn main() {
    println!("{:?}", pattern_table("abcdabca"));
    println!("{:?}", pattern_table("aabaaab"));

    let mut signs = vec![0i8; 3];
    let mut i = 0;
    for n in [4i8, -2, 7] {
        signs[i] = -n;
        i += 1;
    }
    println!("{signs:?}");

    let mut bytes = vec![0u8; 2];
    let mut j = 0;
    while j < 2 {
        bytes[j] = (j as u16 * 300) as u8;
        j += 1;
    }
    println!("{bytes:?}");
    let mut k = 0;
    for wide in [300u16, 7] {
        bytes[k] = wide as u8;
        k += 1;
    }
    println!("{bytes:?}");

    let mut items = vec![0u32; 2];
    shares(&mut items, 1, 9, 2);
    println!("{items:?}");
    let by_zero = std::panic::catch_unwind(move || shares(&mut vec![0u32; 2], 5, 9, 0));
    println!("{}", by_zero.is_err());
    let past = std::panic::catch_unwind(move || shares(&mut vec![0u32; 2], 5, 9, 3));
    println!("{}", past.is_err());
}
