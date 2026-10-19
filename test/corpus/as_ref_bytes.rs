// Text as `AsRef<[u8]>`, base64's `decode<T: AsRef<[u8]>>(input: T)`: its
// UTF-8 bytes, as `as_bytes()` gives them (ADR 0162).
fn total<T: AsRef<[u8]>>(input: T) -> (usize, u32) {
    let bytes = input.as_ref();
    (bytes.len(), bytes.iter().map(|&b| u32::from(b)).sum())
}

fn main() {
    println!("{:?} {:?} {:?}", total("héllo"), total(String::from("abc")), total(vec![1u8, 2, 3]));
    println!("{:?}", total(&b"raw"[..]));
}
