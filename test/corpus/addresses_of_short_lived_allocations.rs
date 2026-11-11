// Addresses as Rust's allocator gives them (#1426): live allocations never
// overlap, an over-aligned type's is aligned, an address made a pointer again
// finds its place, and a loop that asks the address of each of many
// short-lived allocations runs, its ranges reused once they're dropped.
#[repr(align(64))]
struct Line([u8; 64]);

fn short_lived(n: u32) -> usize {
    let mut total = 0usize;
    for i in 0..n {
        let v: Vec<u8> = format!("GET /{} HTTP/1.1\r\n", i % 97).into_bytes();
        let start = v.as_ptr() as usize;
        let past = v[4..].as_ptr() as usize;
        total = total.wrapping_add(past - start);
    }
    total
}

fn main() {
    // Live ones, of many sizes: no two overlap.
    let live: Vec<Vec<u32>> = (0..300).map(|i| vec![i; (i % 17 + 1) as usize]).collect();
    let mut spans: Vec<(usize, usize)> = live.iter().map(|v| (v.as_ptr() as usize, v.len() * 4)).collect();
    spans.sort();
    let apart = spans.windows(2).all(|w| w[0].0 + w[0].1 <= w[1].0);
    println!("{} {apart}", spans.len());

    let line = Box::new(Line([7; 64]));
    let at = &*line as *const Line as usize;
    println!("{} {}", at % 64, line.0[63]);

    let bytes = vec![10u8, 20, 30];
    let addr = bytes.as_ptr() as usize + 2;
    let back = addr as *const u8;
    println!("{}", unsafe { *back });

    println!("{}", short_lived(50_000));
}
