// #1554, its repro: A crate with an `impl Drop` and a hand-written `Deserialize` is refused: `a value of an associated type, where a type may have a destructor, <D as Deserializer>::Error`
use serde::{Deserialize, Deserializer};

#[derive(Debug)]
struct Loud(i32);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
impl<'de> Deserialize<'de> for Loud {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Loud, D::Error> {
        Ok(Loud(i32::deserialize(d)?))
    }
}

fn main() {
    println!("{:?}", serde_json::from_str::<Loud>("1").map_err(|e| e.to_string()));
}
