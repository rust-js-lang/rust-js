// More items than JS's bound of a call's arguments, about 120,000 in
// Node: `extend_from_within` and a Debug builder's `entries` push each,
// never spread into one call, which throws a `RangeError` (#334).
use std::fmt;

struct Big(u32);

impl fmt::Debug for Big {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut list = f.debug_list();
        list.entry(&"head");
        list.entries(0..self.0);
        list.entry(&"tail");
        list.finish()
    }
}

struct Pairs(u32);

impl fmt::Debug for Pairs {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map()
            .entry(&"a", &1)
            .entries((0..self.0).map(|i| (i, i)))
            .finish()
    }
}

fn main() {
    let mut v = vec![0u8; 200_000];
    v[7] = 3;
    v.extend_from_within(..);
    println!("{} {}", v.len(), v[200_007]);
    let s = format!("{:?}", Big(200_000));
    println!("{} {}", s.len(), &s[s.len() - 10..]);
    println!("{}", format!("{:?}", Pairs(200_000)).len());
    println!("{}", format!("{:#?}", Big(150_000)).len());
}
