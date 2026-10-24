// What a crate reads of a library's items, as it reads its own: a `const`
// by its name, imported from the library's module (ADR 0770). Each test
// compiles a library and a crate that uses it, and runs what it wrote.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** `dep`, a library, and `app`, a crate that uses it, compiled in `dir`:
 * each one's JS. */
function compiled(dir: string, dep: string, app: string): { dep: string; app: string } {
  writeFileSync(join(dir, "dep.rs"), dep);
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`, ...js]);
  writeFileSync(join(dir, "app.rs"), app);
  run([compiler, join(dir, "app.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir, ...js]);
  return { dep: readFileSync(join(dir, "dep", "lib.js"), "utf8"), app: readFileSync(join(dir, "app", "lib.js"), "utf8") };
}

// React's `REACT_MEMO_TYPE`, shared's, which ReactMemo imports by its name
// from `shared/ReactSymbols`: read in its crate and in another alike.
test("a library's const is imported by its name, as its own crate reads it", async () => {
  const dir = fixture("cross-crate-consts");
  const { dep, app } = compiled(dir, `use js::SymbolFor;

pub const REACT_MEMO_TYPE: SymbolFor = SymbolFor("react.memo");
pub const LIMIT: u32 = 5;
pub const NAME: &str = "memo";

pub fn local() -> (SymbolFor<'static>, u32) {
    (REACT_MEMO_TYPE, LIMIT)
}
`, `use js::SymbolFor;

pub fn memo() -> (SymbolFor<'static>, u32, &'static str) {
    (dep::REACT_MEMO_TYPE, dep::LIMIT, dep::NAME)
}
`);
  expect(dep).toContain("  return [REACT_MEMO_TYPE, LIMIT];\n");
  expect(app).toContain('import { LIMIT, NAME, REACT_MEMO_TYPE } from "../dep/lib.js";\n');
  expect(app).toContain("  return [REACT_MEMO_TYPE, LIMIT, NAME];\n");
  const { memo } = await import(join(dir, "app", "lib.js"));
  expect(memo()).toEqual([Symbol.for("react.memo"), 5, "memo"]);
});
