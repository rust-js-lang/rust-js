// Passing jobs must not hide the child owner's observed-overrun diagnostic.
import type {Mutation} from "../mutations";
export const mutations: Mutation[] = [
  {"name":"check-startup-build-blocks-parent","breaks":"a synchronous build delays observing the independent fast prune","file":"scripts/checkFast.ts","find":"const build = Bun.spawn(commands.build, { cwd, stdout: \"inherit\", stderr: \"inherit\" });\n  const code = await build.exited;","replace":"const build = Bun.spawnSync(commands.build, { cwd, stdout: \"inherit\", stderr: \"inherit\" });\n  const code = build.exitCode;","tests":["test/check-startup.test.ts"]},
  {"name":"check-startup-prune-timed-as-build","breaks":"the pruning duration is taken from the compiler rather than its own exit","file":"scripts/checkFast.ts","find":"return { build: built, prune: await pruned };","replace":"return { build: built, prune: { ...await pruned, seconds: built.seconds } };","tests":["test/check-startup.test.ts"]},
  {"name":"check-startup-build-not-awaited","breaks":"jobs can start before a failed or unfinished compiler build","file":"scripts/checkFast.ts","find":"const code = await build.exited;","replace":"const code = 0;","tests":["test/check-startup.test.ts"]},
  {"name":"check-startup-prune-failure-hidden","breaks":"a pruning failure is treated as success","file":"scripts/checkFast.ts","find":".then(code => ({ code, seconds: (Date.now() - pruning) / 1000 }));","replace":".then(code => ({ code: 0, seconds: (Date.now() - pruning) / 1000 }));","tests":["test/check-startup.test.ts"]},
  {
    name: "check-passing-child-warning-hidden",
    breaks: "a successful job hides an observed child overrun and its job name",
    file: "scripts/checkFast.ts",
    find: "for (const warning of childWarnings(result.log)) console.warn(name + \": \" + warning);",
    replace: "void childWarnings(result.log);",
    tests: ["test/timing.test.ts","-t","child observation warning"],
  },
  {
    name: "check-fast-corpus-libraries-only-marked",
    breaks: "check:fast's corpus builds as libraries only the cases marked `library-differs` (#1920)",
    file: "scripts/checkFast.ts",
    find: "env: { RUST_JS_CASES: `${i + 1}/${corpusParts}${picked ? `:${picked.caseList}` : \"\"}` },",
    replace: "env: { RUST_JS_CASES: `${i + 1}/${corpusParts}${picked ? `:${picked.caseList}` : \"\"}`, RUST_JS_LIBRARIES: \"differing\" },",
    tests: ["test/ci.test.ts", "-t", "each push's corpus builds every case as a library"],
  },
];
