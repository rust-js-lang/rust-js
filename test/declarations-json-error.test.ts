// What a crate's declarations say of a `serde_json::Error` (#2373, ADR
// 1149): `@rust-js/std`'s opaque `JsonError`, not `any`, which TypeScript
// takes only of what rust-js gave, its `message`, `line` and `column` read
// as serde_json's `line()` and `column()` are. Those keys are declared of
// one list, `JSON_ERROR_KEYS`, and the runtime's `$jsonErrorOf` writes them,
// which JS can't share: the JS a crate runs holds the two to each other.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildSerde } from "./serde-crate";
import { buildCompiler, compiler, fixture, root, run } from "./support";
import { es2022, repoTypes, type Setting, typeChecked } from "./type-checked";

beforeAll(() => {
  buildCompiler();
  buildSerde("rlib");
}, 600_000);

const checking: Setting = { ...es2022, ...repoTypes };

/** The keys `@rust-js/std`'s committed `JsonError` declares, in order. */
function declaredKeys(): string[] {
  const types = readFileSync(join(root, "std/index.d.ts"), "utf8");
  const body = types.slice(types.indexOf("export interface JsonError {"));
  return [...body.slice(0, body.indexOf("\n}")).matchAll(/^ {2}readonly (\w+): /gm)].map(([, key]) => key!);
}

test("a serde_json::Error is std's opaque JsonError, its keys what the runtime gives", async () => {
  const dir = fixture("declarations-json-error");
  writeFileSync(join(dir, "lib.rs"), `use serde::Deserialize;

#[derive(Deserialize)]
pub struct Point {
    pub x: f64,
}

pub fn parse(text: &str) -> Result<Point, serde_json::Error> {
    serde_json::from_str(text)
}

pub fn shown(error: &serde_json::Error) -> String {
    error.to_string()
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...buildSerde()]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('import type { JsonError } from "@rust-js/std";');
  expect(declarations).toContain("_0: JsonError;");
  expect(declarations).toContain("export function shown(error: JsonError): string;");
  expect(declarations).not.toContain("any");
  expect(declaredKeys()).toEqual(["message", "line", "column"]);
  writeFileSync(join(dir, "use.ts"), `import { parse, shown } from "./lib.js";
const parsed = parse("{");
export const where: string = parsed.TAG === "Err" ? \`\${parsed._0.message} \${parsed._0.line}:\${parsed._0.column} \${shown(parsed._0)}\` : "";
`);
  expect(typeChecked(dir, "use.ts", checking)).toEqual([]);
  writeFileSync(join(dir, "wrong.ts"), `import { shown } from "./lib.js";
shown({ message: "made", line: 1, column: 1 });
shown("made");
`);
  expect(typeChecked(dir, "wrong.ts", checking)).toEqual([
    expect.stringContaining("error TS2741: Property '[jsonError]' is missing in type '{ message: string; line: number; column: number; }' but required in type 'JsonError'"),
    expect.stringContaining("error TS2345: Argument of type 'string' is not assignable to parameter of type 'JsonError'"),
  ]);
  // The runtime's `Err` payload has the declared keys, no more, and its
  // `Display` is built of them as the declaration says.
  const lib = await import(join(dir, "lib.js"));
  const failed = lib.parse("{");
  expect(failed.TAG).toBe("Err");
  expect(Object.keys(failed._0)).toEqual(declaredKeys());
  expect(lib.shown(failed._0)).toBe(`${failed._0.message} at line ${failed._0.line} column ${failed._0.column}`);
});
