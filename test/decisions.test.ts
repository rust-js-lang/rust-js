// Decision coverage's catalogue (src/decisions.rs) is the one list of the
// decisions counted and their answers: each is recorded by the owner it
// names, and each record names a decision and answers of it, so no count
// is of a name the report doesn't know, and no catalogued answer is one no
// code gives (docs/finding-wrong-answers.md).
import { expect, test } from "bun:test";
import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { underLoad } from "../scripts/timing";
import { compiler, fixture, root, run } from "./support";

const catalogue = () => {
  const text = readFileSync(join(root, "src/decisions.rs"), "utf8");
  const entries = [...text.matchAll(/name: "([^"]+)",\s*owner: "([^" ]+) [^"]*",\s*answers: &\[([^\]]*)\]/g)];
  return entries.map(([, name, file, answers]) => ({
    name,
    file,
    answers: [...answers.matchAll(/"([^"]+)"/g)].map(([, a]) => a),
  }));
};

const sources = (dir: string): string[] =>
  readdirSync(join(root, dir)).flatMap((entry) => {
    const path = join(dir, entry);
    return statSync(join(root, path)).isDirectory() ? sources(path) : path.endsWith(".rs") ? [path] : [];
  });

test("each catalogued decision is recorded by its owner", () => {
  const decisions = catalogue();
  expect(decisions.length).toBeGreaterThan(10);
  const unrecorded = decisions
    .filter(({ name, file }) => !readFileSync(join(root, file), "utf8").includes(`decisions::record("${name}"`))
    .map(({ name, file }) => `${name}: ${file} records none`);
  expect(unrecorded).toEqual([]);
});

test("each record names a catalogued decision and its answers", () => {
  const decisions = new Map(catalogue().map((d) => [d.name, d.answers]));
  const wrong: string[] = [];
  for (const file of sources("src")) {
    const text = readFileSync(join(root, file), "utf8");
    // A record's decision, and each answer it gives in its call or the
    // answer `match` that feeds it, as literals.
    for (const [call, name] of text.matchAll(/decisions::record\(\s*"([^"]+)",([^;]*)\);/g)) {
      const answers = decisions.get(name);
      if (!answers) {
        wrong.push(`${file}: ${name} isn't catalogued`);
        continue;
      }
      for (const [, literal] of call.slice(call.indexOf(",")).matchAll(/"([^"]+)"/g)) {
        if (!answers.includes(literal)) wrong.push(`${file}: ${name}'s answer ${literal} isn't catalogued`);
      }
    }
  }
  expect(wrong).toEqual([]);
});

// A compile asked to count, `RUST_JS_DECISIONS`, appends each decision's
// every answer, one it gave by how many times, one it didn't by `0`,
// tagged by its build and shape; one not asked writes nothing.
test("a compile asked counts every answer of every decision, a 0 too", () => {
  const dir = fixture("decisions-counted");
  const source = join(dir, "main.rs");
  writeFileSync(source, "use std::cell::RefCell;\npub fn main() {\n    let c = RefCell::new(1);\n    *c.borrow_mut() += 1;\n    println!(\"{}\", c.borrow());\n}\n");
  const counts = join(dir, "counts.tsv");
  run([compiler, source, "-o", join(dir, "main.js"), "--", "--crate-type=lib"], 120_000, { RUST_JS_DECISIONS: counts });
  const rows = readFileSync(counts, "utf8").trim().split("\n").map((line) => line.split("\t"));
  const answers = catalogue().reduce((n, d) => n + d.answers.length, 0);
  expect(rows.length).toBe(answers);
  const borrowed = rows.filter(([decision]) => decision === "refcell-borrow").reduce((n, [, , count]) => n + Number(count), 0);
  expect(borrowed).toBeGreaterThan(0);
  expect(rows.find(([decision, answer]) => decision === "generic-form" && answer === "held-box")?.slice(2, 5)).toEqual(["0", "plain", "program"]);
}, underLoad(120_000));

// A workflow gives an empty setting where it asks no counts, a run not the
// nightly's (nightly.yml, check.yml): that asks none, not a file named ``.
test("an empty setting asks no counts", () => {
  const dir = fixture("decisions-empty");
  const source = join(dir, "main.rs");
  writeFileSync(source, "pub fn main() {\n    println!(\"{}\", 1);\n}\n");
  const compiled = Bun.spawnSync([compiler, source, "-o", join(dir, "main.js"), "--", "--crate-type=lib"], {
    cwd: root,
    env: { ...process.env, RUST_JS_DECISIONS: "" },
  });
  expect([compiled.exitCode, compiled.stderr.toString()]).toEqual([0, ""]);
}, underLoad(120_000));
