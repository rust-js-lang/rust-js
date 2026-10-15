// A tagged enum's fieldless variant is an object, `{ kind: "On" }` (ADR
// 0284), not its name as an untagged one's is: it's compared, looked for
// and kept as a map's key by what it is, not by JS's identity, and ordered
// by its variant. A float's `contains` and a hand-written `eq` are
// compared as Rust compares them too.
use std::collections::HashMap;

#[cfg_attr(rust_js, rust_js::tag = "kind")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Light {
    On,
    Off,
}

#[derive(Clone, Copy, Debug)]
pub enum Odd {
    A,
    B,
}

impl PartialEq for Odd {
    fn eq(&self, _other: &Odd) -> bool {
        println!("asked");
        true
    }
}

fn main() {
    let xs = [f64::NAN, 1.0];
    println!("{}", xs.contains(&f64::NAN));
    let v: Vec<f64> = vec![f64::NAN];
    println!("{}", v.iter().any(|x| *x == f64::NAN));
    println!("{}", Odd::A == Odd::B);
    let lights = [Light::On, Light::Off];
    println!("{}", lights.contains(&Light::Off));
    println!("{}", Light::On == Light::On);
    let mut seen = HashMap::new();
    seen.insert(Light::On, 1);
    seen.insert(Light::On, 2);
    println!("{} {:?}", seen.len(), seen.get(&Light::On));
    println!("{}", Light::Off as i32);
    // Cast where it runs: by its tag, not its object (ADR 0393).
    let code = |light: Light| light as i32;
    println!("{} {}", code(Light::On), code(Light::Off));
    println!("{:?}", std::mem::discriminant(&Light::Off));
    println!("{} {:?}", Light::On < Light::Off, Light::Off.cmp(&Light::On));
}
