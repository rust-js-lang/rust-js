// What the compiler writes of a block the program labeled, `'desc: { ..
// }`: its label where JS keeps one, by the name the program gave it (ADR
// 0742). Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = () => ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

// ADR 0742: a JS block MIR's breaks leave, that a `break` the program
// wrote of a block it labeled goes to the end of, is the program's label,
// `desc: { .. break desc; }`, not one MIR names by its block, `bb41`.
// React's `addValueToProperties` (shared/ReactPerformanceTrackProperties.js)
// leaves its `'desc` block from deep inside.
test("a block the program labeled keeps its label", async () => {
  const dir = fixture("labeled-block-names");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Unknown};

pub fn describe(value: Option<&Unknown>, indent: usize, out: &mut Vec<String>) -> String {
    let desc = if let Some(value) = value
        && let Kind::Object(_) | Kind::Array(_) = js::classify(Some(value))
    {
        'desc: {
            let keys = js::object::keys(value);
            if keys.is_empty() {
                break 'desc "empty".to_string();
            }
            let name = keys.join(",");
            if name.len() > 10 {
                break 'desc name;
            }
            if name == "a,b" {
                if indent > 4 {
                    break 'desc "deep".to_string();
                } else if indent > 3 {
                    out.push(name);
                    return String::new();
                }
            }
            out.push(name);
            return String::new();
        }
    } else {
        "other".to_string()
    };
    desc
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toMatch(/\n\s+desc: \{\n/);
  expect(js).toMatch(/desc = "deep";\n\s+break desc;\n/);
  expect(js).not.toMatch(/bb\d+/);
  const lib = await import(join(dir, "lib.js"));
  const out: string[] = [];
  const got = [{}, { abcdef: 1, ghijkl: 2 }, { a: 1, b: 2 }, { c: 1 }, null].map((v) => lib.describe(v, 5, out));
  expect(got).toEqual(["empty", "abcdef,ghijkl", "deep", "", "other"]);
  expect(out).toEqual(["c"]);
});
