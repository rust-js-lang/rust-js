// The process a program runs in, Node's (ADR 0391): std's functions of it
// are recognition's one list, `NODE_ONLY`, each a helper of the runtime's
// `process.js`, which in a browser panics, saying which.

import { beforeAll, expect, test } from "bun:test";
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compileJs, node, runJs } from "./programs";
import { buildCompiler, compiler, fixture, root, run } from "./support";

beforeAll(buildCompiler, 600_000);

const read = (path: string) => readFileSync(join(root, path), "utf8");

// The one list, the helper each is, and the runtime's helpers, held equal:
// a function of Node's is added to all three, or the test says which lacks it.
test("each of std's functions that work in Node only is a helper of process.js, gated by $process", () => {
  const catalog = [...read("src/lower/recognition.rs").matchAll(/\("([\w:]+)", ProcessOp::(\w+)\)/g)].map(([, path, op]) => ({ path, op }));
  const helpers = new Map([...read("src/lower/std_types/process.rs").matchAll(/ProcessOp::(\w+) => "(\$\w+)"/g)].map(([, op, helper]) => [op, helper]));
  const runtime = read("src/runtime/process.js");
  const functions = [...runtime.matchAll(/^function (\$\w+)\(/gm)].map(([, name]) => name);
  expect(catalog.length).toBeGreaterThan(1);
  for (const { path, op } of catalog) {
    const helper = helpers.get(op);
    expect(helper, op).toBeDefined();
    const body = runtime.slice(runtime.indexOf(`function ${helper}(`)).split("\n}\n")[0];
    expect(body, helper).toContain(`$process("\`${path}\`")`);
  }
  expect(functions.filter((name) => name !== "$process").sort()).toEqual([...helpers.values()].sort());
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
  const { $args, $exit } = await import(module);
  const saved = globalThis.process;
  Object.defineProperty(globalThis, "process", { value: undefined, configurable: true, writable: true });
  try {
    expect(() => $args()).toThrow("`std::env::args` needs Node's process, which a browser hasn't (ADR 0391)");
    expect(() => $exit(1)).toThrow("`std::process::exit` needs Node's process");
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
