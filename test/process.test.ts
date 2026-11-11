// The process a program runs in, Node's (ADR 0391): std's functions of it,
// and of its files (#1156), are recognition's one list, `NODE_ONLY`, each a
// helper of the runtime's `process.js` or `fs.js`, which in a browser
// panics, saying which.

import { beforeAll, expect, test } from "bun:test";
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compileJs, node, runJs } from "./programs";
import { buildCompiler, compiler, fixture, root, run } from "./support";

beforeAll(buildCompiler, 600_000);

const read = (path: string) => readFileSync(join(root, path), "utf8");

// The one list, the helper each is, and the runtime's helpers, held equal:
// a function of Node's is added to all three, or the test says which lacks it.
test("each of std's functions that work in Node only is a helper of process.js or fs.js, gated by $process", () => {
  const catalog = [...read("src/lower/recognition/std_items.rs").matchAll(/\("([\w:]+)", NodeOnly::(Process|Fs)\((?:ProcessOp|FsOp)::(\w+)\)\)/g)].map(
    ([, path, of, op]) => ({ path, of, op }),
  );
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
