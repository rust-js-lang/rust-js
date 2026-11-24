// What JS may call is checked, and nothing else (#2403 lever (3), Tu's
// decision): a function the crate's root gives JS, its own or by a `pub
// use`, a module's namespace it re-exports, a class it re-exports, and what
// a module a hand-written JS file of the program imports gives, each a
// forwarder's target too, as `reexports::JsReached` finds it. A function
// `pub mod` paths alone reach, which only Rust names, isn't checked and
// isn't split: its JS is main's. A namespace's method JS may call splits
// like a function, its body beside it in the object.
import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { compiler, fixture, root, run } from "./support";
import { exportsDeclared } from "./type-checked";

/** The crate of `files`, `lib.rs` its root, beside `js`, hand-written JS
 * files, compiled: each module's JS by its file's name. */
function compiled(name: string, files: Record<string, string>, js: Record<string, string> = {}) {
  const dir = fixture(name);
  for (const [file, text] of Object.entries({ ...files, ...js })) writeFileSync(join(dir, file), text);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type=lib", "--edition=2024"]);
  return { dir, read: (file: string) => readFileSync(join(dir, file), "utf8") };
}

const inner = `pub fn hidden(n: u32) -> u32 {
    n + 1
}

pub fn given(n: u32) -> u32 {
    n * 2
}
`;

let reach: ReturnType<typeof compiled>;

beforeAll(() => {
  reach = compiled("boundary-reach", {
    "lib.rs": `pub mod inner;
pub mod shown;
pub mod imported;

pub use inner::given;
pub use shown as display;

pub fn root(n: u32) -> u32 {
    inner::hidden(n) + inner::given(n)
}

/// Passes on to \`inner::passed\`, which its module re-exports in its place.
pub fn passed(n: u32) -> u32 {
    inner::passed(n)
}
`,
    "inner.rs": `${inner}
pub fn passed(n: u32) -> u32 {
    n + 3
}
`,
    "shown.rs": "pub fn shown(n: u32) -> u32 {\n    n\n}\n",
    "imported.rs": "pub fn imported(n: u32) -> u32 {\n    n\n}\n\npub fn untouched(n: u32) -> u32 {\n    n\n}\n",
  }, {
    "app.js": 'import { imported } from "./imported.js";\nexport const value = imported(1);\n',
  });
}, 600_000);

// A file rust-js wrote is no hand-written JS: compiled again, beside the
// JS its first build wrote, whose root imports `inner.js`, the same.
test("compiled again, a file rust-js wrote is no hand-written JS", () => {
  const first = reach.read("inner.js");
  run([compiler, join(reach.dir, "lib.rs"), "-o", join(reach.dir, "lib.js"), "--", "--crate-type=lib", "--edition=2024"]);
  expect(reach.read("inner.js")).toBe(first);
});

test("a function only pub mod paths reach is unchecked and unsplit", () => {
  const js = reach.read("inner.js");
  expect(js).toContain("export function hidden(n) {\n  return (n + 1) >>> 0;\n}");
  expect(js).not.toContain("hidden$");
});

test("the root's own function, and one its pub use gives, are checked", () => {
  expect(reach.read("lib.js")).toContain('$checkU32(n, "root(n)");');
  expect(reach.read("inner.js")).toContain('$checkU32(n, "given(n)");');
});

test("a module whose namespace the root re-exports is checked", () => {
  expect(reach.read("shown.js")).toContain('$checkU32(n, "shown(n)");');
});

test("a module a hand-written JS file imports is checked, all it gives", () => {
  const js = reach.read("imported.js");
  expect(js).toContain('$checkU32(n, "imported(n)");');
  expect(js).toContain('$checkU32(n, "untouched(n)");');
});

test("a forwarder's target is checked, as its module re-exports it in its place", () => {
  expect(reach.read("lib.js")).toContain('export { passed } from "./inner.js";');
  expect(reach.read("inner.js")).toContain('$checkU32(n, "passed(n)");');
});

test("a namespace's method JS may call splits, its body beside it in the object", async () => {
  const { dir, read } = compiled("boundary-reach-namespace", {
    "lib.rs": `pub struct Perms(u8);

impl Perms {
    pub const fn from_bits_retain(bits: u8) -> Self {
        Perms(bits)
    }

    pub fn bits(&self) -> u8 {
        self.0
    }
}

pub fn both() -> u8 {
    Perms::from_bits_retain(3).bits()
}
`,
  });
  const js = read("lib.js");
  expect(js).toContain('  fromBitsRetain(bits) {\n    $checkU8(bits, "Perms.fromBitsRetain(bits)");\n    return Perms.fromBitsRetain$(bits);\n  },\n  fromBitsRetain$(bits) {');
  expect(js).toContain("Perms.bits$(Perms.fromBitsRetain$(3))");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.both()).toBe(3);
  expect(() => lib.Perms.fromBitsRetain(300)).toThrow("Perms.fromBitsRetain(bits): expected a u8");
});

// A library's function its own JS doesn't check, as no JS of the library
// may call it (`Item::checked`), given JS by a consumer: the consumer's own
// checked forwarder in its re-export's place, by name or by a glob; and of a
// namespace of its module, a module of its own in its place (#2403 (Y), #2536).
const unchecked = "pub mod m {\n    pub fn f(n: u32) -> u32 {\n        n + 1\n    }\n}\n";

/** `dep`, `unchecked`, a library, and `app` using it, compiled in a
 * fixture of `name`: the app's JS, or what the compiler said refusing it. */
function consumer(name: string, app: string, dep = unchecked): { js?: string; refused?: string; dir: string } {
  const dir = fixture(name);
  writeFileSync(join(dir, "dep.rs"), dep);
  run([compiler, "--declarations", join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`]);
  writeFileSync(join(dir, "app.rs"), app);
  const made = runSync([compiler, "--declarations", join(dir, "app.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir], root, 120_000);
  if (made.code !== 0) return { refused: made.stderr, dir };
  return { js: readFileSync(join(dir, "app", "lib.js"), "utf8"), dir };
}

test("a library's unchecked function a consumer re-exports by name is its own checked forwarder", async () => {
  const { js, dir } = consumer("boundary-reach-by-name", "pub use dep::m::f as g;\n");
  expect(readFileSync(join(dir, "dep", "m.js"), "utf8")).not.toContain("$check");
  expect(js).toContain('export function g(n) {\n  $checkU32(n, "g(n)");\n  return f(n);\n}');
  const app = await import(join(dir, "app", "lib.js"));
  expect(app.g(1)).toBe(2);
  expect(() => app.g(-1)).toThrow("g(n): expected a u32");
  // Its `.d.ts` re-exports it still, the forwarder the function's own
  // signature: react-dom's `useFormState` had gone from its list.
  expect(await exportsDeclared(dir, "app/lib.js")).toEqual([]);
});

// By its own name, which the module imports the library's by: named apart,
// exported as it, `export { f$1 as f }`.
test("a library's unchecked function a consumer re-exports by a glob is its own checked forwarder", async () => {
  const { js, dir } = consumer("boundary-reach-glob", "pub use dep::m::*;\n");
  expect(js).toContain('function f$1(n) {\n  $checkU32(n, "f(n)");\n  return f(n);\n}\n\nexport { f$1 as f };');
  const app = await import(join(dir, "app", "lib.js"));
  expect(() => app.f(-1)).toThrow("f(n): expected a u32");
  expect(await exportsDeclared(dir, "app/lib.js")).toEqual([]);
});

test("a library's module of unchecked functions a consumer re-exports as a namespace is a module of its own, which checks them", async () => {
  const { js, dir } = consumer("boundary-reach-namespace", "pub use dep::m as ns;\n\npub fn one() -> u32 {\n    ns::f(0)\n}\n");
  expect(js).toContain('export * as ns from "./ns.js";');
  const ns = readFileSync(join(dir, "app", "ns.js"), "utf8");
  expect(ns).toContain('import * as Ns from "../dep/m.js";');
  expect(ns).toContain('export * from "../dep/m.js";');
  expect(ns).toContain('export function f(n) {\n  $checkU32(n, "f(n)");\n  return Ns.f(n);\n}');
  const app = await import(join(dir, "app", "lib.js"));
  expect([app.one(), app.ns.f(1)]).toEqual([1, 2]);
  expect(() => app.ns.f(-1)).toThrow("f(n): expected a u32");
  expect(await exportsDeclared(dir, "app/lib.js")).toEqual([]);
  expect(await exportsDeclared(dir, "app/ns.js")).toEqual([]);
});

// One of a list, where the module's JS writes its forwarder: its `.d.ts`
// re-exports it still, in its place among the list's names, as written
// (react-dom's `useFormState` before `useFormStatus`, #2403 (Y)).
test("a library's unchecked function of a list a consumer re-exports is declared in its place", async () => {
  const dep = "pub mod m {\n    pub fn f(n: u32) -> u32 {\n        n + 1\n    }\n\n    pub const K: u32 = 7;\n}\n";
  const { js, dir } = consumer("boundary-reach-list", "pub use dep::m::{f, K};\n", dep);
  expect(js).toContain('export { K } from "../dep/m.js";');
  expect(js).toContain('$checkU32(n, "f(n)");');
  expect(readFileSync(join(dir, "app", "lib.d.ts"), "utf8")).toContain('export { f, K } from "../dep/m.js";');
  expect(await exportsDeclared(dir, "app/lib.js")).toEqual([]);
});
