use serde::{Deserialize, Deserializer};

struct Guard(&'static str);
impl Drop for Guard {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
#[derive(Debug)]
struct Pair(i32, i32);
impl<'de> Deserialize<'de> for Pair {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Pair, D::Error> {
        let _g = Guard("pair");
        let (a, b) = <(i32, i32)>::deserialize(d)?;
        println!("read both");
        Ok(Pair(a, b))
    }
}

fn main() {
    println!("{:?}", serde_json::from_str::<Pair>("[1,\"x\"]").map_err(|e| e.to_string()));
    println!("{:?}", serde_json::from_str::<Pair>("[1,2]").map_err(|e| e.to_string()));
    println!("end");
}
