// The tests a test file has, by name: what a mutation's `tests` may name,
// `-t` a pattern of them (scripts/mutations.ts). A mutation naming a test
// that's gone runs nothing as the compiler is, and CI's shard said so only
// when it ran: #2542 took away the test `never-holding-fact-asked` names, in
// test/crates.test.ts, whose other tests are named where they run, by
// templates, which a read of the file's literals can't judge, and `main`
// went red (2026-11-25).

import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";

import { cratePairParts, partOf } from "../scripts/shard";
import { bunTest } from "./support";

const root = join(import.meta.dir, "..");

/** A test file's tests, read without running it: whether it's there,
 * `names`, each named by a literal, and whether those are all its tests,
 * `complete`: not where one is named by a template or a value, or it's no
 * test file. The corpus's tests are its cases; a snapshots file's, each
 * row of the table it gives `snapshotTests`; a part of the crate pairs'
 * table, the pairs `partOf` puts in it. */
export function testNamesOf(file: string): { exists: boolean; names: string[]; complete: boolean } {
  // Read once a process: a catalogue's 16,000 mutations name some 400
  // files, the corpus's the 3,000 cases of its folder.
  let found = read.get(file);
  if (found === undefined) read.set(file, (found = namesRead(file)));
  return found;
}

const read = new Map<string, { exists: boolean; names: string[]; complete: boolean }>();

function namesRead(file: string): { exists: boolean; names: string[]; complete: boolean } {
  const path = join(root, file);
  const exists = existsSync(path);
  const source = exists ? readFileSync(path, "utf8") : "";
  const literal = /^\s*(?:test|it)(?:\.\w+)?\(\s*"((?:[^"\\]|\\.)*)"/gm;
  const cases =
    file === "test/corpus.test.ts"
      ? readdirSync(join(root, "test/corpus")).filter((name) => name.endsWith(".rs"))
      : source.includes("snapshotTests(")
        ? [...source.matchAll(/^\s*\["(\w+)", "/gm)].map((m) => `${m[1]}: the generated JS is its snapshot`)
        : /^test\/crates-pairs-(\d+)\.test\.ts$/.test(file)
          ? readdirSync(join(root, "test/crates/pairs"))
              .filter((name) => partOf(name, cratePairParts) === Number(/(\d+)\.test\.ts$/.exec(file)![1]))
              .map((name) => `two crates: ${name}`)
          : [];
  const complete = cases.length > 0 || (file.endsWith(".test.ts") && !/^\s*(?:test|it)(?:\.\w+)?\(\s*[^"\s]/m.test(source));
  return { exists, names: [...cases, ...[...source.matchAll(literal)].map((m) => JSON.parse(`"${m[1]}"`) as string)], complete };
}

/** Each of `files`' tests as Bun registers them, its own listing: each run
 * with a pattern no test has, its JUnit report naming every test it
 * skipped (`bunTest`, a test's one way to run `bun test`): 0.6 s
 * test/leases.test.ts', 9 s test/compiler.test.ts', which loads what its
 * tests share as it starts. Only what a change touches is listed, a few. */
export function listedTests(files: string[], dir: string): Map<string, string[]> {
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const listed = new Map<string, string[]>();
  const unescape = (text: string) => text.replace(/&quot;/g, '"').replace(/&apos;/g, "'").replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&amp;/g, "&");
  for (const file of files) {
    const report = join(dir, `${file.replace(/\W/g, "_")}.xml`);
    bunTest([`./${file}`, "-t", "^ no test is named this $", "--reporter=junit", `--reporter-outfile=${report}`]);
    const xml = existsSync(report) ? readFileSync(report, "utf8") : "";
    listed.set(file, [...xml.matchAll(/<testcase name="([^"]*)"/g)].map((m) => unescape(m[1]!)));
  }
  return listed;
}

/** Whether `pattern`, a `-t`'s, names a test of `file`, or with none, the
 * file is a test file there: of `listed`, Bun's own listing of it, where
 * given; else a literal name of a file whose tests are all literals; any of
 * one whose aren't, which the runner's controls judge as it runs it. */
export function namesATest(file: string, pattern: string | undefined, listed?: ReadonlyMap<string, string[]>): boolean {
  const { exists, names, complete } = testNamesOf(file);
  if (!exists) return false;
  if (pattern === undefined) return true;
  const matching = new RegExp(pattern);
  const known = listed?.get(file);
  if (known !== undefined) return known.some((name) => matching.test(name));
  return !complete || names.some((name) => matching.test(name));
}
