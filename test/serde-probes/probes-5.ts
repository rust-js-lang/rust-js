import { writeFileSync } from "node:fs";
const head = (await Bun.file("probes-3.ts").text()).split("const head = String.raw`")[1].split("`;")[0];
const R = String.raw;
const lines = [
  R`#[derive(Serialize, Deserialize, Debug, Default)] #[serde(default)] struct T(i32, String); println!("{}", r::<T>(r#"[4,"s"]"#)); println!("{}", w(&serde_json::from_str::<T>("[4]").unwrap()));`,
  R`#[derive(Serialize, Deserialize, Debug, Default)] #[serde(default)] struct T(i32, i32); let t: T = serde_json::from_str("[]").unwrap(); println!("{}", t.0 + t.1);`,
  R`#[derive(Serialize, Deserialize, Debug)] #[serde(transparent)] struct Tr(Body); #[derive(Serialize, Deserialize, Debug)] #[serde(tag = "t")] enum E { A(Tr) } println!("{} {}", w(&E::A(Tr(Body { n: 1 }))), r::<E>(r#"{"t":"A","n":3}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] #[serde(tag = "t")] enum E { B(Box<Body>) } println!("{} {}", w(&E::B(Box::new(Body { n: 2 }))), r::<E>(r#"{"t":"B","n":3}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] #[serde(tag = "t")] enum E { A(Vec<i32>) } println!("{}", r::<E>(r#"{"t":"A"}"#));`,
  R`#[derive(Serialize, Deserialize, Debug)] #[serde(tag = "t")] enum E { A(Option<Body>) } println!("{} {}", w(&E::A(Some(Body { n: 1 }))), w(&E::A(None)));`,
  R`#[derive(Serialize, Deserialize, Debug)] #[serde(tag = "t")] enum In { X { a: i32 } } #[derive(Serialize, Deserialize, Debug)] #[serde(tag = "o")] enum E { A(In) } println!("{} {}", w(&E::A(In::X { a: 1 })), r::<E>(r#"{"o":"A","t":"X","a":2}"#));`,
  R`let m: HashMap<String, i32> = serde_json::from_str(r#"{"a":1,"a":2}"#).unwrap(); println!("{m:?}");`,
  R`#[derive(Deserialize, Debug)] struct S { a: Vec<i32> } let deep = format!("{{\"a\":[],\"x\":{}{}}}", "[".repeat(200), "]".repeat(200)); println!("{}", r::<S>(&deep));`,
];
lines.forEach((line, i) => writeFileSync(`p/chk_${String(i).padStart(2, "0")}.rs`, `${head}\nfn main() {\n${line}\n}\n`));
