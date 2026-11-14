// An exported `Result` is declared the value its JS is, `{ TAG: "Ok"; _0:
// T } | { TAG: "Err"; _0: E }`, as an enum's is (ADR 0033), of its type
// arguments, a `js::JsError` what it's declared everywhere, `any`, whatever
// JS threw (#1835): Codex's probe, Intl's `getCanonicalLocales` that keeps
// what it threw.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

/** TypeScript's errors of `file` in `dir`, strict, its declarations checked too. */
function typeChecked(dir: string, file: string): string[] {
  const config = join(dir, `tsconfig-${file}.json`);
  writeFileSync(config, JSON.stringify({
    compilerOptions: { module: "esnext", moduleResolution: "bundler", target: "es2022", strict: true, noEmit: true, skipLibCheck: false, exactOptionalPropertyTypes: true, types: [] },
    files: [file],
  }));
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", config], { cwd: dir });
  return checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
}

test("an exported Result is declared its tagged union, of its type arguments", async () => {
  const dir = fixture("declarations-results");
  writeFileSync(join(dir, "lib.rs"), `pub fn result(value: bool) -> Result<Vec<String>, &'static js::JsError> {
    if value { Ok(vec![String::from("ok")]) } else { Err(js::error::new(Some("no"), None)) }
}

pub fn generic<T>(value: T, ok: bool) -> Result<T, u32> {
    if ok { Ok(value) } else { Err(1) }
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('export function result(value: boolean): {\n    TAG: "Ok";\n    _0: string[];\n} | {\n    TAG: "Err";\n    _0: any;\n};');
  expect(declarations).toContain('export function generic<T>(value: T, ok: boolean): {\n    TAG: "Ok";\n    _0: T;\n} | {\n    TAG: "Err";\n    _0: number;\n};');
  const { result } = await import(join(dir, "lib.js"));
  expect(result(true)).toEqual({ TAG: "Ok", _0: ["ok"] });
  expect(result(false).TAG).toBe("Err");
  writeFileSync(join(dir, "use.ts"), `import { generic, result } from "./lib.js";
const made = result(true);
if (made.TAG === "Ok") {
  const locales: string[] = made._0;
  console.log(locales);
} else {
  const thrown: Error = made._0;
  console.log(thrown);
}
const kept = generic("en", true);
export const tag: "Ok" | "Err" = kept.TAG;
if (kept.TAG === "Ok") {
  const text: string = kept._0;
  console.log(text);
}
`);
  expect(typeChecked(dir, "use.ts")).toEqual([]);
  // A `Result` isn't its `Ok` value.
  writeFileSync(join(dir, "wrong.ts"), `import { generic } from "./lib.js";
export const text: string = generic("en", true);
`);
  expect(typeChecked(dir, "wrong.ts")).toEqual([expect.stringContaining("error TS2322")]);
});
