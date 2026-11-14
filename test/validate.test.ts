// `js::validate` (#1681 change 3, M1, ADR 1143): every tree the compiler
// prints is checked first, in every build; each body between tidy's stages
// too where `RUST_JS_VALIDATE` is set, as `.env.test` does of every process
// a test starts. A read nothing declares nor made a global is rust-js's own
// mistake: the compile is refused, naming the module, the read and what
// broke it. Nothing a program writes makes one, so the test makes one:
// `--without-known Global` forgets each global's mark.
import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, target } from "./support";

beforeAll(buildReact, 600_000);

test("a read nothing declares nor made a global is refused, saying what broke it", () => {
  expect(readFileSync(join(import.meta.dir, "..", ".env.test"), "utf8")).toContain("\nRUST_JS_VALIDATE=1\n");
  const dir = fixture("validate");
  writeFileSync(join(dir, "main.rs"), 'fn main() {\n    println!("hi");\n}\n');
  const compile = (...flags: string[]) => Bun.spawnSync([compiler, join(dir, "main.rs"), "-o", join(dir, "main.js"), ...flags]);
  expect(compile().exitCode).toBe(0);
  const refused = compile("--without-known", "Global");
  expect(refused.exitCode).not.toBe(0);
  expect(refused.stderr.toString()).toContain(
    `rust-js made ${join(dir, "main.js")} wrong as it was lowered: \`console\` is read, and nothing declares it nor made it a global: a bug of rust-js's`,
  );
});

// What preparation and tidy make of a global's read keeps it one: tidy's
// destructuring of two binding statics' reads, `window.$RC` and
// `window.$RX`, React's Fizz instruction set's, is of `window`, the global,
// not a variable nothing declares (#1681, found regenerating the port).
test("a destructuring of a global's properties reads the global", () => {
  const dir = fixture("validate-destructured");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;

unsafe extern "Rust" {
    #[link_name = "window.$RC"]
    safe static RC: &'static Unknown;
    #[link_name = "window.$RX"]
    safe static RX: &'static Unknown;
}

pub fn both() -> (&'static Unknown, &'static Unknown) {
    let (complete, client_render) = (RC, RX);
    (complete, client_render)
}
`);
  const compiled = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  expect(compiled.exitCode, compiled.stderr.toString()).toBe(0);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("  const complete = window.$RC;\n  const { $RX: clientRender } = window;\n");
});
