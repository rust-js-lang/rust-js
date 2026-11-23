// A crate rustc builds whose API rust-js writes the JS of (ADR 0529):
// memchr's searches, over the byte arrays slices are, and getrandom's bytes
// and its `Error` (#2326). Native Rust is the oracle, of the same source;
// and what's outside the API rust-js writes is refused where it's called.
import { beforeAll, expect, test } from "bun:test";
import { cpSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { printed } from "./crates";
import { buildSerde } from "./serde-crate";
import { buildCompiler, compiler, fixture, nativeBinary, root, run, writeWhole } from "./support";
import { channel } from "../scripts/toolchain.ts";

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

const cli = join(root, "tooling", "cli.js");
const build = (dir: string) => runSync([process.execPath, cli, "build"], dir, 600_000, { RUST_JS_COMPILER: compiler });
const cargo = (dir: string, ...args: string[]) => runSync(["cargo", `+${channel}`, ...args], dir, 600_000);

/** `test/shipped/getrandom`, the app of each qualified getrandom and
 * rand_core 0.5.1, in a fixture `name`, its crates fetched as its locks
 * pin them, crates.io's. Only `rust-js build` checks a crate as the
 * catalog qualifies it, and says so beside its metadata, as serde's
 * build (`serde/build.sh`), a plain Cargo's, doesn't: so it's built so. */
function getrandomApp(name: string): string {
  const at = fixture(name);
  cpSync(join(root, "test", "shipped", "getrandom"), at, { recursive: true });
  for (const dir of [at, join(at, "native")]) expect(cargo(dir, "fetch", "--locked", "-q").code).toBe(0);
  return at;
}

// Each qualified getrandom, against native getrandom of the same version:
// fills of each length across the edge of one `getRandomValues`, a part of
// a slice, uninitialized bytes, its numbers; each of its `Error`'s
// constants and codes, the OS's among them, shown plain, pretty, padded,
// a piece at a time into a writer, and as a `Box<dyn Error>`; and rand_core
// 0.5's `OsRng` and `From<getrandom::Error>`, which itertools' quickcheck
// reaches.
test("getrandom's API runs as native getrandom's, at each version it's qualified at", () => {
  const at = getrandomApp("shipped-getrandom");
  const built = build(at);
  expect(built.stderr).toBe("");
  expect(built.code).toBe(0);
  const js = readFileSync(join(at, "src", "lib.js"), "utf8");
  expect(js).toContain('$getrandom(dest, "0.1")');
  expect(js).toContain('$getrandomU64()');
  expect(js).not.toContain("rust-js/crates/getrandom");
  const native = cargo(join(at, "native"), "run", "-q", "--locked");
  expect(native.code).toBe(0);
  expect(printed(join(at, "src", "lib.js"))).toBe(native.stdout);
}, 1_200_000);

// A getrandom whose check chooses a backend itself: 0.3's `wasm_js` with
// the `getrandom_backend` cfg its user gives, and 0.4's `wasm_js`, whose
// custom backend would leave the `Error::WEB_CRYPTO` its `Display` names
// undefined. rustc checks the backend chosen, against rust-js's own
// wasm-bindgen (tooling/compat/covered.txt); its JS is rust-js's still.
test("a getrandom whose check chooses its own backend is checked so, its JS rust-js's", () => {
  const at = fixture("shipped-getrandom-backend");
  cpSync(join(root, "test", "shipped", "getrandom"), at, { recursive: true });
  const manifest = readFileSync(join(at, "Cargo.toml"), "utf8")
    .replace('version = "=0.3.4", features = ["std"]', 'version = "=0.3.4", features = ["std", "wasm_js"]')
    .replace('version = "=0.4.3" }', 'version = "=0.4.3", features = ["wasm_js"] }');
  writeFileSync(join(at, "Cargo.toml"), manifest);
  mkdirSync(join(at, ".cargo"), { recursive: true });
  writeWhole(join(at, ".cargo", "config.toml"), "[target.wasm32-unknown-unknown]\nrustflags = [\"--cfg\", 'getrandom_backend=\"wasm_js\"']\n");
  // Its locks take the features' crates, wasm-bindgen's.
  for (const dir of [at, join(at, "native")]) expect(cargo(dir, "fetch", "-q").code).toBe(0);
  const built = build(at);
  expect(built.stderr).toBe("");
  expect(built.code).toBe(0);
  expect(readFileSync(join(at, ".cargo", "config.toml"), "utf8")).toContain("wasm-bindgen = { path =");
  expect(existsSync(join(at, "rust-js", "crates"))).toBe(true);
  expect(readFileSync(join(at, "src", "lib.js"), "utf8")).toContain('$getrandom(dest, "0.3")');
  const native = cargo(join(at, "native"), "run", "-q");
  expect(native.code).toBe(0);
  expect(printed(join(at, "src", "lib.js"))).toBe(native.stdout);
}, 1_200_000);

// What rust-js doesn't write of getrandom, 0.4's `SysRng` or an `Error`
// made an `io::Error`, is refused where it's called; and so is any of a
// getrandom no qualified check checked, a plain Cargo build's, as serde's
// build is, whose version rust-js can't know.
test("getrandom rust-js doesn't write, or doesn't know the version of, is refused", () => {
  const at = getrandomApp("shipped-getrandom-refused");
  writeFileSync(join(at, "src", "lib.rs"), "pub fn f(e: getrandom02::Error) -> std::io::Error {\n    e.into()\n}\n");
  const into = build(at);
  expect(into.code).not.toBe(0);
  expect(into.stderr).toContain("rust-js does not support calling `std::convert::Into::into`");
  writeFileSync(join(at, "src", "lib.rs"), "pub fn f() -> u32 {\n    rand_core::RngCore::next_u32(&mut getrandom04::SysRng)\n}\n");
  const sys = build(at);
  expect(sys.code).not.toBe(0);
  expect(sys.stderr).toContain("getrandom");
  // A plain Cargo build's, getrandom 0.1's, whose backend for rust-js's
  // target is its failing one.
  const plain = fixture("shipped-getrandom-plain");
  mkdirSync(join(plain, "src"), { recursive: true });
  writeWhole(join(plain, "Cargo.toml"), '[package]\nname = "plain"\nversion = "0.0.0"\nedition = "2024"\n\n[dependencies]\ngetrandom = "=0.1.16"\n\n[workspace]\n');
  writeWhole(join(plain, "src", "lib.rs"), "");
  const checked = cargo(plain, "build", "-q", "--target", "wasm32-unknown-unknown", "--message-format=json");
  expect(checked.code).toBe(0);
  const artifacts: string[] = checked.stdout
    .split("\n")
    .filter((line) => line.startsWith("{"))
    .map((line) => JSON.parse(line))
    .filter((message) => message.reason === "compiler-artifact")
    .flatMap((message) => message.filenames.filter((file: string) => file.endsWith(".rlib")));
  const rlib = artifacts.find((file) => file.includes("libgetrandom-"));
  expect(rlib).toBeDefined();
  writeWhole(join(plain, "app.rs"), "pub fn f(dest: &mut [u8]) -> bool {\n    getrandom::getrandom(dest).is_ok()\n}\n");
  const direct = Bun.spawnSync([compiler, join(plain, "app.rs"), "-o", join(plain, "app.js"), "--", "--edition=2024", "--crate-type=lib", `--extern=getrandom=${rlib}`, ...artifacts.flatMap((file) => ["-L", `dependency=${join(file, "..")}`])]);
  expect(direct.exitCode).not.toBe(0);
  expect(direct.stderr.toString()).toContain("rust-js does not support calling `getrandom::getrandom`");
}, 1_200_000);

// serde_json, read as rustc builds it, its `IndexMut`'s panics raised where
// the version Cargo built raises them: Cargo's check of it through rust-js
// finds them in that version's source and notes them beside its metadata,
// which the app's compile reads (#2325).
test("serde_json's sites are noted as Cargo's check of it through rust-js is made, and given where its panics are raised", () => {
  const at = fixture("shipped-serde-json-sites");
  mkdirSync(join(at, "src"), { recursive: true });
  writeWhole(join(at, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[dependencies]\nserde_json = "=1.0.151"\n\n[workspace]\n');
  writeWhole(join(at, "src", "lib.rs"), 'pub fn set(v: &mut serde_json::Value) {\n    v["a"] = serde_json::Value::Null;\n}\n');
  expect(cargo(at, "fetch", "-q").code).toBe(0);
  const built = build(at);
  expect(built.stderr).toBe("");
  expect(built.code).toBe(0);
  expect(readFileSync(join(at, "src", "lib.js"), "utf8")).toContain(
    '"/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/serde_json-1.0.151/src/value/index.rs:102:18"',
  );
}, 600_000);
