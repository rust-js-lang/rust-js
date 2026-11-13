// What runs nothing is what lowering knew it to be, never a name (ADR
// 1138): a JS global's read lowering named, and a value's text, called
// where it may run code, as a format's arguments are evaluated first.
import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { build } from "vite";
import { runSync } from "./child";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// A JS global's read, `Math.PI` of `to_degrees()`, which no call changes, as
// lowering named it (`Known::BuiltIn`, #1674): a value `{:?}` reads twice
// after it is held first, past it, never by the global's name, which a
// crate's item may share.
test("a value `{:?}` reads twice is held past a JS global's read", async () => {
  const dir = fixture("mir-applied-past-a-global");
  writeFileSync(join(dir, "lib.rs"), `fn parse(s: &str) -> Result<u8, String> { s.parse::<u8>().map_err(|_| s.to_string()) }
pub fn main() {
    let x = 1.5f64;
    println!("{} {:?}", x.to_degrees(), parse("x"));
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('const value = parse("x");');
  expect(js).not.toContain("((result) =>");
});

// A JS value's text, `js::shown`, is its `toString`, which a format runs
// where it formats it, after every argument is evaluated, as Rust's
// `format_args!` takes them: a later argument's call before it, held, in
// the build and the production build, minified, whose minifier takes a
// `toString` to run nothing (#1676).
test("js::shown's toString runs after every argument is evaluated", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("shown-ordered");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;

thread_local! {
    static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

pub fn mark(what: String) {
    LOG.with(|log| log.borrow_mut().push(what));
}

fn loud(n: i32) -> i32 {
    mark(format!("arg {n}"));
    n
}

pub fn around(value: Option<&js::Unknown>) -> String {
    format!("{} {} {}", loud(1), js::shown(value), loud(2))
}

pub fn before(value: Option<&js::Unknown>) -> String {
    format!("{} {}", js::shown(value), loud(3))
}

pub fn log() -> String {
    LOG.with(|log| log.borrow().join(", "))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  writeFileSync(
    join(dir, "main.js"),
    `import { around, before, log, mark } from "./lib.js";
const value = { toString() { mark("toString"); return "o"; } };
console.log(around(value), before(value), log());
`,
  );
  const want = "1 o 2 o 3 arg 1, arg 2, toString, arg 3, toString\n";
  expect(runSync(["node", join(dir, "main.js")], dir, 60_000).stdout).toBe(want);
  await build({
    configFile: false,
    logLevel: "silent",
    root: dir,
    build: { outDir: join(dir, "production"), emptyOutDir: true, minify: true, target: "esnext", lib: { entry: join(dir, "main.js"), formats: ["es"], fileName: () => "main.min.js" } },
  });
  expect(runSync(["node", join(dir, "production", "main.min.js")], dir, 60_000).stdout).toBe(want);
});
