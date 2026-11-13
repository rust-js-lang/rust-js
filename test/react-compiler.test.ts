// The React Compiler net (ADR 1142): every component and hook rust-js
// emits is one React Compiler compiles, at the version Vite's React plugin
// uses, so an app turning it on keeps every memo. Per push, Babel alone:
// the guide's output fences, examples/' committed JS and the cases of
// test/react-compiler/, compiled here, each a program or a consumer's
// finding. A function it skips or refuses fails, but one
// test/react-compiler-known.txt lists, of an open `react-compiler` issue
// or failing as the hand-written JS it names does; an entry nothing fails
// of fails too. The lowering tests' programs are swept nightly
// (scripts/react-compiler-sweep.ts).
import { beforeAll, expect, test } from "bun:test";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { runSync } from "./child";
import { type Failure, failures, known, names, presetsPlugin } from "./react-compiler-check";
import { buildReact, compiler, fixture, root, run, target } from "./support";
import { underLoad } from "../scripts/timing";

beforeAll(buildReact, 600_000);

const walk = (dir: string): string[] =>
  readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (["node_modules", "target", "dist", ".next"].includes(name) || name.startsWith(".")) return [];
    return statSync(path).isDirectory() ? walk(path) : [path];
  });

/** Each module of the guide's output fences, `js` and `jsx`: a fence of
 * several, each headed `// src/x.js`, taken apart. */
function guideModules(): { source: string; code: string }[] {
  const out: { source: string; code: string }[] = [];
  for (const page of walk(join(root, "docs/guide")).filter((p) => p.endsWith(".md"))) {
    const text = readFileSync(page, "utf8");
    let k = 0;
    for (const m of text.matchAll(/^```(jsx|js)\n([\s\S]*?)^```$/gm)) {
      k++;
      const parts = m[2].split(/^(?=\/\/ \S+\.m?jsx?\n)/m);
      parts.forEach((code, i) => out.push({ source: `${relative(root, page)}#${k}${parts.length > 1 ? `.${i + 1}` : ""}`, code }));
    }
  }
  return out;
}

/** examples/' committed JS of React's. */
function exampleModules(): { source: string; code: string }[] {
  return walk(join(root, "examples"))
    .filter((p) => /\.(jsx?|mjs)$/.test(p))
    .map((p) => ({ source: relative(root, p), code: readFileSync(p, "utf8") }))
    .filter(({ code, source }) => source.endsWith(".jsx") || code.includes('from "react'));
}

/** Each of test/react-compiler/'s cases compiled, against react, webapi and js. */
function caseModules(): { source: string; code: string }[] {
  const dir = join(root, "test/react-compiler");
  return readdirSync(dir)
    .filter((name) => name.endsWith(".rs"))
    .map((name) => {
      const out = fixture(`react-compiler-${name.replace(/\.rs$/, "")}`);
      run([compiler, join(dir, name), "-o", join(out, "lib.js"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
      return { source: `test/react-compiler/${name}`, code: readFileSync(join(out, "lib.jsx"), "utf8") };
    });
}

test("the React Compiler the net runs is the one Vite's React plugin's preset resolves", () => {
  expect(presetsPlugin()).toBe(require.resolve("babel-plugin-react-compiler"));
  const version = JSON.parse(readFileSync(join(presetsPlugin(), "../../package.json"), "utf8")).version;
  expect(version).toBe(JSON.parse(readFileSync(join(root, "package.json"), "utf8")).devDependencies["babel-plugin-react-compiler"]);
});

test("React Compiler compiles every component and hook rust-js emits, but the known", () => {
  const entries = known();
  const unlisted: string[] = [];
  const met = new Set<number>();
  let compiled = 0;
  for (const { source, code } of [...guideModules(), ...exampleModules(), ...caseModules()]) {
    compiled++;
    for (const failure of failures(code, source)) {
      // A guide fence that isn't a module, two imports side by side to
      // compare: what's compiled the docs test holds (test/docs.test.ts).
      if (failure.kind === "Throw" && source.startsWith("docs/guide/") && failure.reason.startsWith("SyntaxError")) continue;
      const entry = entries.find((e) => names(e, source, failure));
      if (entry) met.add(entry.line);
      else unlisted.push(`${source}\t${failure.fn}\t${failure.reason}\t(${failure.kind})`);
    }
  }
  // A failure no entry lists: fix rust-js, or file a `react-compiler` issue
  // and list it, or, failing as hand-written JS does, name that JS.
  expect(unlisted).toEqual([]);
  // An entry nothing fails of is stale: take it off.
  expect(entries.filter((e) => !e.source.startsWith("sweep:") && !met.has(e.line)).map((e) => `line ${e.line}: ${e.source} ${e.fn}`)).toEqual([]);
  expect(compiled).toBeGreaterThan(50);
}, 600_000);

// An entry "as hand-written" names JS a person would write that fails the
// same: it does, so the failure isn't rust-js's.
test("each known entry's hand-written witness fails as it says", () => {
  for (const entry of known().filter((e) => e.witness)) {
    const path = join(root, "test/react-compiler", entry.witness!);
    const failed: Failure[] = failures(readFileSync(path, "utf8"), path);
    expect([entry.line, failed.some((f) => f.reason === entry.reason)]).toEqual([entry.line, true]);
  }
});

// An entry's issue is open and labeled `react-compiler`, where `gh` can
// say: one closed is fixed, and its entry stale.
test("each known entry's issue is an open react-compiler one", () => {
  const issues = [...new Set(known().flatMap((e) => (e.issue ? [e.issue] : [])))];
  for (const issue of issues) {
    const p = runSync(["gh", "issue", "view", String(issue), "--json", "state,labels"], root, 30_000);
    if (p.code !== 0) {
      console.warn(`gh can't say of #${issue} here: ${p.stderr.split("\n")[0]}`);
      continue;
    }
    const { state, labels } = JSON.parse(p.stdout) as { state: string; labels: { name: string }[] };
    expect([issue, state, labels.some((l) => l.name === "react-compiler")]).toEqual([issue, "OPEN", true]);
  }
}, underLoad(120_000));
