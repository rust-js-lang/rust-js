use serde::ser::{Error, SerializeSeq};
use serde::{Serialize, Serializer};

struct Guard(&'static str);
impl Drop for Guard {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
struct Bad;
impl Serialize for Bad {
    fn serialize<S: Serializer>(&self, _s: S) -> Result<S::Ok, S::Error> {
        let _g = Guard("bad");
        Err(S::Error::custom("inner"))
    }
}
struct Outer;
impl Serialize for Outer {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let _g = Guard("outer");
        let mut seq = s.serialize_seq(Some(1))?;
        seq.serialize_element(&Bad)?;
        println!("not reached");
        seq.end()
    }
}

fn main() {
    println!("{:?}", serde_json::to_string(&Outer).map_err(|e| e.to_string()));
    println!("{:?}", serde_json::to_string(&vec![Outer]).map_err(|e| e.to_string()));
    println!("end");
}
