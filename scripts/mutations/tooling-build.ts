// Mutations of tooling/build.js, the host's builder and `rust-js test`.
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "run-output-piped",
    breaks: "a command the builder runs writes to pipes, whose `close` a child it leaves running holds off, so `rust-js --version-json` hangs the build to its timeout (#2441); read of files it never wrote, what it says is lost",
    file: "tooling/build.js",
    find: '      child = spawn(command, args, { cwd, env: { ...process.env, RUST_JS_JS_RUNTIME: process.execPath, ...env }, stdio: ["ignore", ...fds] });\n',
    replace: '      child = spawn(command, args, { cwd, env: { ...process.env, RUST_JS_JS_RUNTIME: process.execPath, ...env }, stdio: ["ignore", "pipe", "pipe"] });\n',
    tests: ["test/in-source.test.ts", "-t", "the builder reads what the compiler said once it has exited"],
  },
  {
    name: "refused-tests-not-run-again",
    breaks: "`rust-js test` leaves the tests a refusal at run time came before refused, never running them in a process of their own (#2625)",
    file: "tooling/build.js",
    find: "      names = refused;\n",
    replace: "      break;\n",
    tests: ["test/rust-js-test-runners.test.ts", "-t", "a test after one refused at run time is run again"],
  },
];
