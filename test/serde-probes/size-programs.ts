import { writeFileSync } from "node:fs";
const head = `#![allow(unused, dead_code)]\nuse serde::{Deserialize, Serialize};\n`;
// hello: no serde at all
writeFileSync("hello.rs", `pub fn greet(name: &str) -> String { format!("hello {name}") }\nfn main() { println!("{}", greet("x")); }\n`);
const strukt = (i: number) => `#[derive(Serialize, Deserialize, Clone, Debug)]\n#[serde(rename_all = "camelCase")]\npub struct T${i} { pub item_id: u32, pub display_name: String, pub tags: Vec<String>, pub price: f64, pub is_active: bool, pub note: Option<String>, pub count_a: i32, pub count_b: i32 }\n`;
const many = (n: number, ser: boolean, de: boolean) => {
  let s = head;
  for (let i = 0; i < n; i++) s += strukt(i).replace("Serialize, Deserialize", [ser && "Serialize", de && "Deserialize"].filter(Boolean).join(", "));
  for (let i = 0; i < n; i++) {
    if (ser) s += `pub fn write${i}(v: &T${i}) -> String { serde_json::to_string(v).unwrap() }\n`;
    if (de) s += `pub fn read${i}(t: &str) -> Result<T${i}, serde_json::Error> { serde_json::from_str(t) }\n`;
  }
  return s + "fn main() {}\n";
};
writeFileSync("one_both.rs", many(1, true, true));
writeFileSync("one_ser.rs", many(1, true, false));
writeFileSync("one_de.rs", many(1, false, true));
writeFileSync("ten_both.rs", many(10, true, true));
writeFileSync("fifty_both.rs", many(50, true, true));
writeFileSync("fifty_ser.rs", many(50, true, false));
writeFileSync("fifty_de.rs", many(50, false, true));
writeFileSync("value.rs", head + `use serde_json::Value;\npub fn read(t: &str) -> Result<Value, serde_json::Error> { serde_json::from_str(t) }\npub fn write(v: &Value) -> String { serde_json::to_string(v).unwrap() }\nfn main() {}\n`);
