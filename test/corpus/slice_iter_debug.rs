// `{:?}` of a slice's iterator is what's left of it, as its `as_slice()`
// gives it, `Iter([2, 3])`, a derived `Debug`'s field too, as
// percent-encoding's `PercentDecode` holds one (ADR 0453).
#[derive(Clone, Debug)]
struct Decode<'a> {
    bytes: std::slice::Iter<'a, u8>,
}

fn rest(items: &[u32]) -> String {
    let mut it = items.iter();
    it.next();
    format!("{it:?}")
}

fn tail(items: &[u8]) -> std::slice::Iter<'_, u8> {
    println!("tail");
    let mut it = items.iter();
    it.next();
    it
}

fn main() {
    let data = [1u8, 2, 3];
    let mut it = data.iter();
    it.next();
    println!("{:?} {:?}", it.as_slice(), it);
    let decode = Decode { bytes: data.iter() };
    println!("{decode:?}");
    println!("{}", rest(&[7, 8, 9]));
    println!("{:?}", data[1..].iter());
    println!("{:?}", tail(&data));
}
