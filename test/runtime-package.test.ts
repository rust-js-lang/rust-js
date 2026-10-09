// The runtime as a package (ADR 0103): `@rust-js/runtime`, released with
// the compiler, as ReScript's `@rescript/runtime` is. A module imports the
// helpers its code names from it, instead of carrying its own copy of each.

import { beforeAll, expect, test } from "bun:test";
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { node } from "./programs";
import { buildCompiler, compiler, fixture, root, run } from "./support";

beforeAll(buildCompiler, 600_000);

// The package's module is the compiler's helpers, each exported: made by
// the compiler, so it can't drift from what the compiler's imports name.
test("the committed @rust-js/runtime is the compiler's helpers, each exported once", () => {
  const printed = run([compiler, "--runtime-module"]);
  expect(readFileSync(join(root, "runtime", "index.js"), "utf8")).toBe(printed);
  const names = [...printed.matchAll(/^export (?:async )?(?:function\*?|class|const|let) (\$[\w$]+)/gm)].map((m) => m[1]);
  expect(names.length).toBeGreaterThan(100);
  expect(new Set(names).size).toBe(names.length);
  const pkg = JSON.parse(readFileSync(join(root, "runtime", "package.json"), "utf8"));
  const version = readFileSync(join(root, "Cargo.toml"), "utf8").match(/^version = "([^"]+)"/m)![1];
  expect([pkg.name, pkg.version]).toEqual(["@rust-js/runtime", version]);
});

// Each helper is a file of its own, `src/runtime/<name>.js`, that the
// compiler reads in: JS that parses as JS, and none left behind unread.
test("each helper's file parses as JavaScript, and the compiler reads each", () => {
  const dir = join(root, "src", "runtime");
  const files = readdirSync(dir).filter((file) => file.endsWith(".js"));
  const rust = readFileSync(join(root, "src", "runtime.rs"), "utf8");
  expect(files.length).toBeGreaterThan(150);
  for (const file of files) {
    expect(rust, file).toContain(`include_str!("runtime/${file}")`);
    // Compiled, and never run.
    expect(() => new Function(readFileSync(join(dir, file), "utf8")), file).not.toThrow();
  }
});

// A module names a few helpers, and imports just those; one a helper uses,
// the package has for it.
test("a module imports the helpers it names from @rust-js/runtime, and runs", () => {
  const dir = fixture("runtime-package");
  writeFileSync(join(dir, "lib.rs"), `pub fn main() {
    let names = vec!["ada", "grace"];
    let i = names.len() - 1;
    println!("{:?} {}", names[i], 7 / (i as i32));
    let parsed: Result<u32, _> = "x".parse();
    println!("{:?}", parsed.is_err());
}
`);
  const out = join(dir, "lib.js");
  run([compiler, join(dir, "lib.rs"), "-o", out]);
  const js = readFileSync(out, "utf8");
  expect(js).toMatch(/^import \{ [^}]+ \} from "@rust-js\/runtime";$/m);
  expect(js).not.toMatch(/^function \$/m);
  const imported = js.match(/^import \{ ([^}]+) \} from "@rust-js\/runtime";$/m)![1].split(", ");
  expect(imported).toEqual([...imported].sort());
  for (const name of imported) expect(js.split("\n").slice(3).join("\n")).toContain(name);
  const printed = run([node ?? "node", "--input-type=module", "--eval", `(await import(${JSON.stringify(out)})).main();`]);
  expect(printed).toBe('"grace" 7\ntrue\n');
});

// A `OnceLock`'s or `LazyLock`'s init that uses its own cell deadlocks in
// Rust, which JS can't do: rust-js says so, never a std-looking panic
// (ADRs 0317, 0318).
test("a lock's init that uses its own lock is rust-js's error, not a deadlock", () => {
  const dir = fixture("runtime-lock-reentrant");
  writeFileSync(join(dir, "lib.rs"), `use std::sync::{LazyLock, OnceLock};

static LAZY: LazyLock<u32> = LazyLock::new(|| *LAZY + 1);
static ONCE: OnceLock<u32> = OnceLock::new();

pub fn lazy() -> u32 {
    *LAZY
}

pub fn once() -> u32 {
    *ONCE.get_or_init(|| {
        let _ = ONCE.set(1);
        2
    })
}
`);
  const out = join(dir, "lib.js");
  run([compiler, join(dir, "lib.rs"), "-o", out]);
  const call = (name: string) =>
    run([node ?? "node", "--input-type=module", "--eval", `try { (await import(${JSON.stringify(out)})).${name}(); } catch (e) { console.log(e.message); }`]);
  expect(call("lazy")).toBe("rust-js does not support a `LazyLock` whose init uses it, which deadlocks in Rust\n");
  expect(call("once")).toBe("rust-js does not support a `OnceLock` whose init sets it, which deadlocks in Rust\n");
});

// Locking a lock its thread holds deadlocks in Rust, which JS can't do:
// rust-js says so (ADR 0328).
test("a lock locked again while held is rust-js's error, not a deadlock", () => {
  const dir = fixture("runtime-lock-held");
  writeFileSync(join(dir, "lib.rs"), `use std::sync::{Mutex, RwLock};

pub fn mutex() -> u32 {
    let m = Mutex::new(1);
    let held = m.lock().unwrap();
    let again = *m.lock().unwrap();
    *held + again
}

pub fn rw() -> u32 {
    let rw = RwLock::new(1);
    let reading = rw.read().unwrap();
    let shared = *rw.read().unwrap();
    *rw.write().unwrap() += 1;
    *reading + shared
}

pub fn rw_read() -> u32 {
    let rw = RwLock::new(1);
    let mut writing = rw.write().unwrap();
    *writing += *rw.read().unwrap();
    *writing
}
`);
  const out = join(dir, "lib.js");
  run([compiler, join(dir, "lib.rs"), "-o", out]);
  const call = (name: string) =>
    run([node ?? "node", "--input-type=module", "--eval", `try { (await import(${JSON.stringify(out)})).${name}(); } catch (e) { console.log(e.message); }`]);
  const error = "rust-js does not support locking a lock its thread holds, which deadlocks in Rust\n";
  expect(call("mutex")).toBe(error);
  expect(call("rw")).toBe(error);
  expect(call("rw_read")).toBe(error);
});
