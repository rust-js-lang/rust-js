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
#[derive(Deserialize, Debug)]
struct S {
    a: Loud,
    b: Loud,
}

fn main() {
    println!("{:?}", serde_json::from_str::<S>(r#"{"a":1,"b":"x"}"#).map_err(|e| e.to_string()));
    println!("{:?}", serde_json::from_str::<Vec<Loud>>("[2,3,true]").map_err(|e| e.to_string()));
    println!("{:?}", serde_json::from_str::<(Loud, Loud)>("[4,5,6]").map_err(|e| e.to_string()));
    println!("end");
}
