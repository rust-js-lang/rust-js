// #1833: a scope guard's LocalKey::with runs only its callback once an
// eager key exists. Its cleanup is a finally when that callback cannot
// throw. A lazy initializer or a throwing callback retains abort-safe cleanup.
import {beforeAll, expect, test} from "bun:test";
import {readFileSync, writeFileSync} from "node:fs";
import {join} from "node:path";
import {buildCompiler, compiler, fixture, run, root} from "./support";

beforeAll(buildCompiler, 600_000);

function compile(name: string, drop: string, initializer = "const { Cell::new(0) }", extra = "") {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
${extra}
thread_local! { static VALUE: Cell<u32> = ${initializer}; }
struct Restore(u32);
impl Drop for Restore {
    fn drop(&mut self) { VALUE.with(|value| { ${drop} }); }
}
pub fn read() -> u32 { VALUE.with(Cell::get) }
pub fn set(next: u32) { VALUE.with(|value| value.set(next)); }
pub fn scoped<T>(next: u32, run: impl FnOnce() -> T) -> T {
    let outer = VALUE.with(|value| value.replace(next));
    let _restore = Restore(outer);
    run()
}
pub fn guarded<T>(run: impl FnOnce() -> T) -> T {
    let _restore = Restore(0);
    run()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  return {dir, js:readFileSync(join(dir, "lib.js"), "utf8")};
}

let pure: ReturnType<typeof compile>;
beforeAll(() => {pure = compile("local-with-infallible-drop", "value.set(self.0);");}, 60_000);

test("a LocalKey with restore preserves callback results, nesting and exact thrown identity", async () => {
  const lib = await import(join(pure.dir, "lib.js"));
  const trace: unknown[] = [];
  const token = {};
  const result = lib.scoped(3, () => {
    trace.push(lib.read());
    const inner = lib.scoped(7, () => {trace.push(lib.read()); return token;});
    expect(inner).toBe(token);
    trace.push(lib.read());
    return token;
  });
  expect(result).toBe(token);
  expect(trace).toEqual([3,7,3]);
  expect(lib.read()).toBe(0);
  for (const value of [undefined, null, false, 0, "", token, Promise.resolve(token)]) {
    expect(lib.scoped(9, () => value)).toBe(value);
    expect(lib.read()).toBe(0);
  }
  for (const marker of [undefined, null, 0, token, Error("callback")]) {
    let caught = false;
    let thrown: unknown;
    try {lib.scoped(4, () => {throw marker;});} catch (error) {caught = true; thrown = error;}
    expect(caught).toBe(true);
    expect(thrown).toBe(marker);
    expect(lib.read()).toBe(0);
  }
});

test("a proved nonthrowing LocalKey with restore uses one finally", () => {
  expect((pure.js.match(/\bfinally\b/g) ?? []).length).toBe(2);
  expect(pure.js).not.toContain("$cleanup");
  // Stack-overflow/abort guards remain governed by ADR 0660; this test
  // deliberately does not demand that semantic machinery disappear.
});

test("a LocalKey with callback that can panic keeps abort-safe cleanup", async () => {
  const fallible = compile("local-with-fallible-drop", 'if value.get() == 9 { panic!("restore"); } value.set(self.0);');
  expect(fallible.js).toContain("$cleanup");
  const lib = await import(join(fallible.dir, "lib.js"));
  expect(lib.scoped(3, () => 8)).toBe(8);
  expect(lib.read()).toBe(0);
  expect(() => lib.scoped(9, () => 8)).toThrow("restore");
  expect(lib.read()).toBe(9);
});


test("a lazy LocalKey initializer remains a possible panic in its drop", async () => {
  const lazy = compile("local-with-lazy-drop", "value.set(self.0);", "initialize()", `fn initialize() -> Cell<u32> { panic!("initializer"); }`);
  expect(lazy.js).toContain("$cleanup");
  const lib = await import(join(lazy.dir, "lib.js"));
  expect(() => lib.guarded(() => 8)).toThrow("initializer");
});


test("an unrelated function receiving a key and callback is not LocalKey with", async () => {
  const other = compile("local-with-unrelated-function", "explode(&VALUE, |_| {}); value.set(self.0);", "const { Cell::new(0) }", `use std::thread::LocalKey;
fn explode(_key: &LocalKey<Cell<u32>>, _run: impl FnOnce(&Cell<u32>)) { panic!("unrelated"); }`);
  expect(other.js).toContain("$cleanup");
  const lib = await import(join(other.dir, "lib.js"));
  expect(() => lib.guarded(() => 8)).toThrow("unrelated");
});

test("a generic LocalKey with restore proves its instantiated callback", async () => {
  const dir = fixture("local-with-generic-drop");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
thread_local! { static VALUE: Cell<u32> = const { Cell::new(0) }; }
struct Restore<F: Fn()>(F);
impl<F: Fn()> Drop for Restore<F> {
    fn drop(&mut self) { VALUE.with(|_| (self.0)()); }
}
pub fn read() -> u32 { VALUE.get() }
pub fn scoped<T>(next: u32, run: impl FnOnce() -> T) -> T {
    let outer = VALUE.replace(next);
    let _restore = Restore(|| VALUE.set(outer));
    run()
}
pub fn with_cleanup<T>(run: impl FnOnce() -> T, cleanup: impl Fn()) -> T {
    let _restore = Restore(cleanup);
    run()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  const token = {};
  expect(lib.scoped(5, () => {expect(lib.read()).toBe(5); return token;})).toBe(token);
  expect(lib.read()).toBe(0);
  const marker = {};
  let thrown: unknown;
  try {lib.withCleanup(() => token, () => {throw marker;});} catch (error) {thrown = error;}
  expect(thrown).toBe(marker);
  const abort = join(dir, "abort.mjs");
  writeFileSync(abort, `import {withCleanup} from "./lib.js";
try {
  withCleanup(() => {throw Error("callback");}, () => {throw Error("cleanup");});
  console.log("survived");
} catch {
  console.log("caught");
}
`);
  const ended = Bun.spawnSync(["bun", abort], {cwd:root, timeout:10000, stdout:"pipe", stderr:"pipe"});
  expect([ended.exitCode, ended.stdout.toString(), ended.stderr.toString()]).toEqual([134, "", "thread caused non-unwinding panic. aborting.\n"]);
  expect(lib.withCleanup.toString()).toContain("$cleanup");
  expect(lib.scoped.toString()).toContain("finally");
  expect(lib.scoped.toString()).not.toContain("$cleanup");
});
