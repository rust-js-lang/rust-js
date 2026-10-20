// A crate rustc builds whose API rust-js writes the JS of (ADR 0529):
// memchr's searches, over the byte arrays slices are. Native Rust is the
// oracle, of the same source, the crate corpus's probe; and what's outside
// the API rust-js writes is refused where it's called.
import { beforeAll, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { printed } from "./crates";
import { buildCompiler, buildSerde, compiler, fixture, nativeBinary, root, run, writeWhole } from "./support";

beforeAll(() => {
  buildCompiler();
  buildSerde("rlib");
}, 600_000);

test("memchr's searches run as native memchr's do", () => {
  const dir = fixture("shipped-memchr");
  const report = readFileSync(join(root, "scripts/crate-corpus/memchr.rs"), "utf8");
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
  expect(js).toContain("$memchr(text, [111])");
  expect(js).toContain("$memchrIter(");
  expect(printed(join(dir, "app.js"))).toBe(native);
}, 300_000);

// What rust-js doesn't write of memchr, `memmem`'s, is refused where it's
// called: rustc built the crate, so there's no JS of it.
test("a memchr function rust-js doesn't write is refused", () => {
  const dir = fixture("shipped-memchr-memmem");
  writeWhole(join(dir, "lib.rs"), "pub fn f(h: &[u8]) -> Option<usize> {\n    memchr::memmem::find(h, b\"ab\")\n}\n");
  const p = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--edition=2024", "--crate-type=lib", ...buildSerde()]);
  expect(p.exitCode).not.toBe(0);
  expect(p.stderr.toString()).toContain("rust-js does not support calling `memchr::memmem::find`");
}, 300_000);
