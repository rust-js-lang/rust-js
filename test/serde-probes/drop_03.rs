use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(from = "i32")]
struct Loud(i32);
impl From<i32> for Loud {
    fn from(n: i32) -> Loud {
        Loud(n)
    }
}
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
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
    println!("{:?}", serde_json::from_str::<S>(r#"{"a":7,"a":8,"b":9}"#).map_err(|e| e.to_string()));
    println!("end");
}
