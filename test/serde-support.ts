// The oracle of the serde tests: a case's Rust built natively with
// serde_json, the same source and dependency versions as its JS, and its
// JS built and loaded (split from serde.test.ts by concept, #1334).
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { buildSerde, compiler, contentDirectory, nativeBinary, run, writeWhole } from "./support";

/** `dir/cases.rs` with serde_json natively, the oracle: printing its
 * `report()`. Built beside a copy named by what it says, so the same cases
 * are one kept binary (`nativeBinary`). */
export function native(dir: string): string {
  const cases = readFileSync(join(dir, "cases.rs"), "utf8");
  const beside = contentDirectory(cases);
  writeWhole(join(beside, "cases.rs"), cases);
  const built = nativeBinary(`include!("cases.rs");
fn main() { println!("{}", serde_json::to_string(&report()).unwrap()); }
`, beside, ["--edition=2024", "-Awarnings", ...buildSerde("rlib")]);
  if ("error" in built) throw new Error(built.error);
  return built.binary;
}

/** The case's JS, built and loaded. */
export async function compiled(dir: string) {
  run([compiler, join(dir, "cases.rs"), "-o", join(dir, "cases.js"), "--", ...buildSerde()]);
  return await import(join(dir, "cases.js"));
}
