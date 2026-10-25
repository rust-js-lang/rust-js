// Crates whose maps are std's (ADRs 0423, 0424): rustc builds them, and a
// program's `hashbrown::HashMap` and `HashSet`, and indexmap's `IndexMap`
// and `IndexSet`, are what std's are in JS, a `Map` and a `Set`. Native
// Rust is the oracle, of the same source, the crate corpus's probe of each.
import { beforeAll, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { printed } from "./crates";
import { buildCompiler, buildSerde, compiler, fixture, nativeBinary, root, run, writeWhole } from "./support";

beforeAll(() => {
  buildCompiler();
  buildSerde("rlib");
}, 600_000);

for (const crate of ["hashbrown", "indexmap"]) {
  test(`${crate}'s maps run as std's, as native ${crate}'s do`, async () => {
    const dir = fixture(`bound-${crate}`);
    const report = readFileSync(join(root, `scripts/crate-corpus/${crate}.rs`), "utf8");
    const built = nativeBinary(`mod report {\n${report}\n}\nfn main() { print!("{}", report::report()); }\n`, dir, [
      "--edition=2024",
      "-Awarnings",
      ...buildSerde("rlib"),
    ]);
    if ("error" in built) throw new Error(built.error);
    const native = run([built.binary]);
    writeWhole(join(dir, "app.rs"), `${report}\npub fn main() {\n    print!("{}", report());\n}\n`);
    run([compiler, join(dir, "app.rs"), "-o", join(dir, "app.js"), "--", "--edition=2024", "--crate-type=lib", ...buildSerde()]);
    const js = readFileSync(join(dir, "app.js"), "utf8");
    expect(js).toContain("new Map()");
    expect(js).toContain("new Set()");
    // indexmap's room, of its table of places and then its entries, each
    // its hash, its key and its value, which the room native Rust's
    // 64-bit `usize` gives can't tell apart (ADR 0774, amended).
    if (crate === "indexmap") expect(js).toContain("$tableRoomFor(4294967295, 4, 4);\n      $roomFor(4294967295, 12);");
    expect(printed(join(dir, "app.js"))).toBe(native);
  }, 300_000);
}

// What std's maps haven't, the crate's own, is refused where it's called:
// rustc built the crate, so there's no JS of it.
test("a hashbrown method std's maps haven't is refused", () => {
  const dir = fixture("hashbrown-own");
  writeWhole(join(dir, "lib.rs"), "pub fn f() -> u32 {\n    let mut m: hashbrown::HashMap<u32, u32> = hashbrown::HashMap::new();\n    m.insert(1, 2);\n    *m.get_key_value_mut(&1).unwrap().1\n}\n");
  const p = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--edition=2024", "--crate-type=lib", ...buildSerde()]);
  expect(p.exitCode).not.toBe(0);
  expect(p.stderr.toString()).toContain("rust-js does not support calling `hashbrown::HashMap::<K, V, S, A>::get_key_value_mut` yet");
}, 300_000);
