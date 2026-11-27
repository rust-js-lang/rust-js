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
];
