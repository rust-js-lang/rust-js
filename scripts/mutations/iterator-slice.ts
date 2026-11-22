// What a change runs in a push's check, by the paths it touches
// (scripts/iteratorSlice.ts): the WASM build's and rustc's tests' paths
// (dev-loop change 3).
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "push-rows-base-unread",
    breaks: "CI's rows of a push are a branch's merge base's, which on `main` is the commit checked out, so they name nothing and a row a narrower local pick left out waits for the nightly (#2340)",
    file: "scripts/iteratorSlice.ts",
    find: "  return env.RUST_JS_ROWS_BASE || Bun.spawnSync(",
    replace: "  return Bun.spawnSync(",
    tests: ["test/iterator-rows.test.ts", "-t", "the rows the change names are of the range a given base starts"],
  },
  {
    name: "wasm-input-directory-a-file",
    breaks: "a directory wasm/prebuilt.sh lists, `src`, matches only a file of its name, so a change to the compiler skips the WASM build",
    file: "scripts/iteratorSlice.ts",
    find: "return existsSync(full) && statSync(full).isDirectory() ? `${path}/**` : path;",
    replace: "return path;",
    tests: ["test/ci.test.ts", "-t", "the WASM build, rustc's tests and the mutations run"],
  },
  {
    name: "job-imports-not-followed",
    breaks: "what a job's tests import beyond their own imports isn't the job's, so a change to the harness skips it",
    file: "scripts/iteratorSlice.ts",
    find: "      walk([base, `${base}.ts`, `${base}.js`]",
    replace: "      if (!files.includes(file)) continue;\n      walk([base, `${base}.ts`, `${base}.js`]",
    tests: ["test/ci.test.ts", "-t", "the WASM build, rustc's tests and the mutations run"],
  },
  {
    name: "rustc-suite-imports-dropped",
    breaks: "rustc's tests skip a change to what their script imports",
    file: "scripts/iteratorSlice.ts",
    find: '  ...importsOf(["scripts/rustc-suite.ts"]),',
    replace: '  "scripts/rustc-suite.ts",',
    tests: ["test/ci.test.ts", "-t", "the WASM build, rustc's tests and the mutations run"],
  },
  {
    name: "slice-report-unsaid",
    breaks: "a slice's JUnit report is written with nothing beside it saying what ran it, so a failed check's `attribute` can't run its failing tests again (#2003)",
    file: "scripts/iteratorSlice.ts",
    find: "  writeFileSync(`${report}.json`, JSON.stringify(ran satisfies SliceReport));\n",
    replace: "",
    tests: ["test/ci.test.ts", "-t", "a slice's report says what ran it"],
  },
];
