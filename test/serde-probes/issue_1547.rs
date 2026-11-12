// #1547, its repro: A custom `serialize_u128` bypasses `to_value`'s range error and constructs an impossible serde_json Number
use serde::{Serialize, Serializer};
struct Wide;
impl Serialize for Wide {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u128(u128::MAX)
    }
}
pub fn main() {
    println!("{:?}", serde_json::to_string(&Wide));
    println!("{:?}", serde_json::to_value(Wide));
}
