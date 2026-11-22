// Mutations of test/support.ts: a native program's key (`nativeKey`, ADR
// 0567 amended), and the processes a test starts as a shell would
// (`compilerBuild`, `bunTest`, #2066).
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "native-key-dependencies-unread",
    breaks: "a native program built against another build of the crates `-L dependency=` names is the kept one",
    file: "test/support.ts",
    find: '    else if (flag.startsWith("dependency=/") && existsSync(path) && statSync(path).isDirectory()) hash.update(`${portable(path)} ${dependencies(path)}`);\n',
    replace: "",
    tests: ["test/native-outcomes.test.ts", "-t", "dependencies' directory"],
  },
  {
    name: "unchanged-build-written-in-place",
    breaks: "what's built from changed inputs is written where another test run reads it, so a JSX test found `target/libreact.rmeta` gone as a corpus part's run wrote it (check 44024336584)",
    file: "test/support.ts",
    find: "    work(into);\n",
    replace: "    work(dir);\n",
    tests: ["test/harness.test.ts", "-t", "what's built from unchanged inputs isn't built again"],
  },
  {
    name: "compiler-build-bootstrap-kept",
    breaks: "the tests build rust-js with `.env.test`'s `RUSTC_BOOTSTRAP`, so a shell's next build, with `.cargo/config.toml`'s, rebuilds every dependency",
    file: "test/support.ts",
    find: 'export const compilerBuild: { cmd: string[]; env: Record<string, string | undefined> } = { cmd: ["cargo", "build", "--quiet"], env: { RUSTC_BOOTSTRAP: undefined } };\n',
    replace: 'export const compilerBuild: { cmd: string[]; env: Record<string, string | undefined> } = { cmd: ["cargo", "build", "--quiet"], env: {} };\n',
    tests: ["test/harness.test.ts", "-t", "the tests build rust-js as a shell does"],
  },
  {
    name: "bun-test-child-blesses",
    breaks: "a test's own `bun test` child is given its parent's `BLESS`, and a check's \"Bless what differs\" step blesses a fake compiler's JS where harness.test.ts's corpus child was to fail",
    file: "test/support.ts",
    find: 'export const runModes = ["BLESS", "RUST_JS_SNAPSHOTS", "RUST_JS_CASES", ...new Set(parts.map((part) => part.setting))];\n',
    replace: 'export const runModes = ["RUST_JS_SNAPSHOTS", "RUST_JS_CASES", ...new Set(parts.map((part) => part.setting))];\n',
    tests: ["test/part-settings.test.ts", "-t", "a test's own `bun test` child runs without its parent's run modes"],
  },
  {
    name: "bun-test-child-keeps-run-modes",
    breaks: "a test's own `bun test` child is given every run mode of its parent's, `METAMORPHIC=all` and `BLESS` among them",
    file: "test/support.ts",
    find: "  return runSync([process.execPath, \"test\", ...args], cwd, timeout, { ...Object.fromEntries(runModes.map((name) => [name, undefined])), ...env });\n",
    replace: "  return runSync([process.execPath, \"test\", ...args], cwd, timeout, env);\n",
    tests: ["test/part-settings.test.ts", "-t", "a test's own `bun test` child runs without its parent's run modes"],
  },
  {
    name: "known-emptied-issue-kept",
    breaks: "the file of an issue a known list's take-off left with no rows is kept, a heading listing nothing (#2260)",
    file: "test/support.ts",
    find: "    if (file.endsWith(\".txt\") && file !== \"README.txt\" && !kept.has(file)) rmSync(join(dir, file));\n",
    replace: "",
    tests: ["test/support.test.ts", "-t", "a known list kept as a directory"],
  },
  {
    name: "bless-removes-the-whole-folder",
    breaks: "a bless removes its snapshot's folder whole, so blessing the playground, whose snapshot is the JS its app commits beside its Rust, deletes the Rust (`wasm/web/rust`)",
    file: "test/support.ts",
    find: "    for (const path of existsSync(snapshot) ? generated(snapshot, declarations) : []) rmSync(join(snapshot, path), { force: true });\n",
    replace: "    rmSync(snapshot, { recursive: true, force: true });\n",
    tests: ["test/support.test.ts", "-t", "a bless replaces a snapshot's generated files"],
  },
  {
    name: "bless-keeps-stale-generated",
    breaks: "a bless keeps a generated file its compile no longer writes, so the snapshot keeps a module the program lost",
    file: "test/support.ts",
    find: "    for (const path of existsSync(snapshot) ? generated(snapshot, declarations) : []) rmSync(join(snapshot, path), { force: true });\n",
    replace: "",
    tests: ["test/support.test.ts", "-t", "a bless replaces a snapshot's generated files"],
  },
];
