// The process a program runs in, Node's (ADR 0391): std's functions of it,
// and of its files (#1156), are recognition's one list, `NODE_ONLY`, each a
// helper of the runtime's `process.js` or `fs.js`, which in a browser
// panics, saying which.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compileJs, node, runJs } from "./programs";
import { buildCompiler, compiler, fixture, root, run } from "./support";

beforeAll(buildCompiler, 600_000);

const read = (path: string) => readFileSync(join(root, path), "utf8");

// The one list, the helper each is, and the runtime's helpers, held equal:
// a function of Node's is added to all three, or the test says which lacks it.
test("each of std's functions that work in Node only is a helper of process.js or fs.js, gated by $process", () => {
  // Each by its `StdPath`, whose path is the table's (`std_paths.rs`).
  const paths = new Map([...read("src/lower/recognition/std_paths.rs").matchAll(/^    (\w+) => "(std::[^"]+)",$/gm)].map(([, item, path]) => [item, path]));
  const catalog = [...read("src/lower/recognition/std_items.rs").matchAll(/\(StdPath::(\w+), NodeOnly::(Process|Fs)\((?:ProcessOp|FsOp)::(\w+)\)\)/g)].map(
    ([, item, of, op]) => ({ path: paths.get(item)!, of, op }),
  );
  expect(catalog.every(({ path }) => path !== undefined)).toBe(true);
  const helpers = new Map<string, string>([
    ...[...read("src/lower/std_types/process.rs").matchAll(/ProcessOp::(\w+) => "(\$\w+)"/g)].map(([, op, helper]): [string, string] => [`Process ${op}`, helper]),
    ...[...read("src/lower/std_types/fs.rs").matchAll(/FsOp::(\w+) => "(\$\w+)"/g)].map(([, op, helper]): [string, string] => [`Fs ${op}`, helper]),
  ]);
  const process = read("src/runtime/process.js");
  const fs = read("src/runtime/fs.js");
  const functions = [...process.matchAll(/^function (\$\w+)\(/gm)].map(([, name]) => name);
  expect(catalog.filter(({ of }) => of === "Fs").length).toBeGreaterThan(1);
  for (const { path, of, op } of catalog) {
    // A file's `open`, `OpenOptions::open`, is its options' method, which
    // `$openWith` is the body of.
    const helper = path === "std::fs::OpenOptions::open" ? "$openWith" : helpers.get(`${of} ${op}`);
    expect(helper, op).toBeDefined();
    const runtime = of === "Fs" ? fs : process;
    const body = runtime.slice(runtime.indexOf(`function ${helper}(`)).split("\n}\n")[0];
    expect(body, helper).toContain(of === "Fs" ? `$nodeFs("\`${path}\`")` : `$process("\`${path}\`")`);
  }
  // `$abortExit`, an abort's exit, isn't std's function, and ends nothing
  // without a process (#847); `$nodeFs`, Node's fs, is a gate as `$process` is.
  const gates = ["$process", "$abortExit", "$nodeFs"];
  const processHelpers = catalog.filter(({ of }) => of === "Process").map(({ op }) => helpers.get(`Process ${op}`));
  expect(functions.filter((name) => !gates.includes(name)).sort()).toEqual(processHelpers.sort() as string[]);
  // Only it, and printing, which writes to `console` where there's none, read Node's process.
  const runtimeFiles = readdirSync(join(root, "src/runtime"))
    .filter((file) => file.endsWith(".js") && read(`src/runtime/${file}`).includes("globalThis.process"));
  expect(runtimeFiles.sort()).toEqual(["print.js", "process.js"]);
});

// In a browser, which has no process, each panics, naming what needed it.
test("in a browser, a function of Node's process panics, saying which", async () => {
  // The runtime the compiler writes, as a module the test imports.
  const module = join(fixture("process-browser"), "runtime.mjs");
  writeFileSync(module, run([compiler, "--runtime-module"]));
  const { $args, $exit, $fileOpen, $fsRead, $openOptions } = await import(module);
  const saved = globalThis.process;
  Object.defineProperty(globalThis, "process", { value: undefined, configurable: true, writable: true });
  try {
    expect(() => $args()).toThrow("`std::env::args` needs Node's process, which a browser hasn't (ADR 0391)");
    expect(() => $exit(1)).toThrow("`std::process::exit` needs Node's process");
    // std's files, Node's fs (#1156): a browser's panic, which no `Err` is.
    expect(() => $fileOpen("notes.txt")).toThrow("`std::fs::File::open` needs Node's process, which a browser hasn't (ADR 0391)");
    expect(() => $fsRead("notes.txt")).toThrow("`std::fs::read` needs Node's process");
    expect(() => $openOptions().read(true).open("notes.txt")).toThrow("`std::fs::OpenOptions::open` needs Node's process");
  } finally {
    Object.defineProperty(globalThis, "process", { value: saved, configurable: true, writable: true });
  }
});

// `env::temp_dir()` on macOS with no `TMPDIR` is what `getconf
// DARWIN_USER_TEMP_DIR` prints, `confstr`'s, or `/tmp` where it fails, as
// std's `darwin_temp_dir` falls back (#1639): asked of a process that says
// it's macOS, with a `getconf` of the test's own on its `PATH`, so the
// branch runs on Linux too, where CI's mutations run.
test("env::temp_dir on macOS with no TMPDIR asks getconf, else /tmp", async () => {
  const dir = fixture("process-darwin-temp");
  const module = join(dir, "runtime.mjs");
  writeFileSync(module, run([compiler, "--runtime-module"]));
  const { $tempDir } = await import(module);
  const real = globalThis.process;
  const asked = (getconf: string) => {
    const bin = join(dir, `bin-${getconf.length}`);
    mkdirSync(bin, { recursive: true });
    writeFileSync(join(bin, "getconf"), `#!/bin/sh\n${getconf}\n`, { mode: 0o755 });
    const env = Object.fromEntries(Object.entries(real.env).filter(([name]) => name !== "TMPDIR"));
    // Its own `env`, no `TMPDIR`, whose `PATH`, which `spawnSync` reads,
    // finds the test's `getconf` first.
    const darwin = Object.create(real, {
      platform: { value: "darwin" },
      env: { value: { ...env, PATH: `${bin}:${real.env.PATH}` } },
    });
    Object.defineProperty(globalThis, "process", { value: darwin, configurable: true, writable: true });
    try {
      return $tempDir();
    } finally {
      Object.defineProperty(globalThis, "process", { value: real, configurable: true, writable: true });
    }
  };
  expect(asked('echo "/var/folders/stub/T/"')).toBe("/var/folders/stub/T/");
  expect(asked("exit 1")).toBe("/tmp");
});

// In Node, `process::exit(code)` ends the program there, with its code.
test("process::exit ends the program with its code", () => {
  const dir = fixture("process-exit");
  const file = join(dir, "exit.rs");
  writeFileSync(file, 'fn main() {\n    println!("before");\n    std::process::exit(3);\n}\n');
  const compiled = compileJs(file, dir);
  if (!("js" in compiled)) throw new Error(compiled.error);
  expect(readFileSync(compiled.js, "utf8")).toContain("$exit(3);");
  const ran = runJs([node ?? "node"], compiled.js, dir, "node");
  expect(ran.stdout).toBe("before\n");
  // The process exited 3, having said how it ended: by `exit`, of 3.
  expect(ran.outcome).toBe('exited 3 after it ended {"exit":3}');
});

// `env::temp_dir()` and `env::home_dir()` are std's unix bodies (#1639):
// `TMPDIR`, else macOS's `confstr(_CS_DARWIN_USER_TEMP_DIR)`, which
// `getconf` prints, else `/tmp`; `HOME` where it isn't empty, else the
// password database's entry. Each environment that picks a branch, run
// natively and as JS, prints the same.
test("env::temp_dir and env::home_dir read the environment, else the OS, as std's unix bodies do", () => {
  const dir = fixture("process-dirs");
  const file = join(dir, "dirs.rs");
  writeFileSync(
    file,
    'fn main() {\n    println!("{}", std::env::temp_dir().display());\n    println!("{:?}", std::env::home_dir());\n}\n',
  );
  const compiled = compileJs(file, dir);
  if (!("js" in compiled)) throw new Error(compiled.error);
  const js = readFileSync(compiled.js, "utf8");
  expect(js).toContain("$tempDir()");
  expect(js).toContain("$homeDir()");
  const binary = join(dir, "dirs");
  run(["rustc", "--edition", "2024", "-o", binary, file]);
  const { TMPDIR: tmp, HOME: home, ...rest } = process.env;
  const environments: Record<string, Record<string, string | undefined>> = {
    "as it is": process.env,
    "no TMPDIR": { ...rest, HOME: home },
    "an empty TMPDIR": { ...process.env, TMPDIR: "" },
    "no HOME": { ...rest, TMPDIR: tmp },
    "an empty HOME": { ...process.env, HOME: "" },
  };
  for (const [name, env] of Object.entries(environments)) {
    const native = Bun.spawnSync([binary], { env, cwd: dir });
    const outcome = join(dir, "dirs.json");
    const ran = Bun.spawnSync([node ?? "node", join(root, "test/corpus-run.ts"), compiled.js, outcome, compiled.js, outcome], { env, cwd: root });
    expect(native.exitCode).toBe(0);
    expect(native.stdout.toString().split("\n")).toHaveLength(3);
    expect(`${name}: ${ran.stdout.toString()}`).toBe(`${name}: ${native.stdout.toString()}`);
  }
});
