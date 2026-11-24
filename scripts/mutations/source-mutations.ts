// The source-execution boundary is itself held by real-process regressions.
import type {Mutation} from "../mutations";
export const mutations: Mutation[] = [
  {name: "source-unloaded-accepted", breaks: "unrelated passing tests falsely judge an unloaded owner", file: "scripts/source-mutations.ts", find: 'if (!compiled && !sourceWasLoaded(marker, mutation.name))', replace: 'if (false)', tests: ["test/source-mutations.test.ts", "-t", "passing tests that never load"]},
  {name: "source-bad-control-accepted", breaks: "a source mutation is caught even when its unchanged control already fails", file: "scripts/source-mutations.ts", find: 'if (!changed && (!tested.passed || tested.ran === 0))', replace: 'if (false)', tests: ["test/source-mutations.test.ts", "-t", "already failing source control"]},
  {name: "source-setup-failure-kept", breaks: "failed mutation setup leaves its independently copied workspace behind", file: "scripts/source-mutations.ts", find: 'peak = Math.max(peak, bytesOf(workspace));\n      rmSync(workspace, { recursive: true, force: true });', replace: 'peak = Math.max(peak, bytesOf(workspace));', tests: ["test/source-mutations.test.ts", "-t", "failed mutation setup"]},
  {name: "source-flake-a-catch", breaks: "a source mutant's test that fails once and passes run again with it counts as a catch", file: "scripts/source-mutations.ts", find: "(tests) => test(workspace, env, tests)", replace: "async () => tested", tests: ["test/source-mutations.test.ts", "-t", "fails once and passes run again"]},
  {name: "source-bindings-unbuilt-judged", breaks: "a mutant of the bindings crates' Rust whose tests don't build them is judged, a survivor of tests that never compile it (#2105)", file: "scripts/source-mutations.ts", find: "  if (compiled && unbuilt.length > 0)", replace: "  if (false)", tests: ["test/source-mutations.test.ts", "-t", "a mutant of the bindings crates' Rust"]},
  {name: "source-bindings-marked", breaks: "a mutant of the bindings crates' Rust is given a JS mark of its load, which no Rust compiles (#2105)", file: "scripts/source-mutations.ts", find: "      writeFileSync(owner, compiled ? source : markSourceLoad(source, marker, mutation.name));", replace: "      writeFileSync(owner, markSourceLoad(source, marker, mutation.name));", tests: ["test/source-mutations.test.ts", "-t", "a mutant of the bindings crates' Rust"]},
  {
    name: "wasm-build-half-linked",
    breaks: "a wasm mutation's workspace links rust-js.wasm but not the sysroot it type-checks against, so its control's compile fails and every such mutation is inconclusive (#2346)",
    file: "scripts/source-mutations.ts",
    find: "  for (const path of wasmBuild) {\n",
    replace: "  for (const path of wasmBuild.slice(0, 1)) {\n",
    tests: ["test/source-mutations.test.ts", "-t", "a wasm mutation's workspace links what the wasm build made"],
  },
  {
    name: "source-clone-shares-the-template",
    breaks: "a mutant's workspace is the template itself, linked, not a clone, so its write reaches the template and every mutant after it (#2503)",
    file: "scripts/source-mutations.ts",
    find: "    return clonefile(Buffer.from(`${from}\\0`), Buffer.from(`${to}\\0`)) === 0;",
    replace: "    symlinkSync(from, to);\n    return true;",
    tests: ["test/source-mutations.test.ts", "-t", "a workspace made of a workspace of the same files is its own"],
  },
];
