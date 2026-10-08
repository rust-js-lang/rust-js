// Differential test: native Rust vs. the JS that rust-js generates.
//
//   examples/fib.rs ──rustc────► native ──► expected results ─┐
//                  └─rust-js───► fib.js ──► actual results ───┴─► must be equal

import { beforeAll, expect, test } from "bun:test";
import { copyFileSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { runSync } from "./child";
import { decode, expected, same, type Outcome } from "./oracle";
import { callInNode, type Call } from "./programs";
import { root, target, run, buildCompiler, buildReact, buildSerde, buildWebapi, compiler, fixture } from "./support";

// Values are JSON: numbers, and objects and arrays for structs and tuples.
type Case = { fn: string; args: unknown[]; value?: unknown; panic?: string };

// Native Rust runs as the file loads, so each function it called is a test
// of its own. Same semantics rust-js targets: the release profile, where
// arithmetic wraps.
run(["rustc", "--edition=2024", "-Coverflow-checks=off", "--crate-type=lib", "--crate-name=modules",
  "examples/modules/lib.rs", "-o", join(target, "libmodules.rlib")]);
run(["rustc", "--edition=2024", "-Coverflow-checks=off", "--extern", `modules=${join(target, "libmodules.rlib")}`,
  ...buildSerde("rlib"), "test/native.rs", "-o", join(target, "native")]);
// A minute, well past what it takes, so one that doesn't end fails the file.
const cases: Case[] = run([join(target, "native")], 60_000).trim().split("\n").map(decode);
let fib: Record<string, (...args: any[]) => number>;
let structs: Record<string, (...args: any[]) => unknown>;
let closures: Record<string, (...args: any[]) => unknown>;
let collections: Record<string, (...args: any[]) => unknown>;
let options: Record<string, (...args: any[]) => unknown>;
let methods: Record<string, (...args: any[]) => unknown>;
let genericOptions: Record<string, (...args: any[]) => unknown>;
let combinators: Record<string, (...args: any[]) => unknown>;
let text: Record<string, (...args: any[]) => unknown>;
let calc: Record<string, (...args: any[]) => unknown>;
let numbers: Record<string, (...args: any[]) => unknown>;
let inventory: Record<string, (...args: any[]) => unknown>;
let queues: Record<string, (...args: any[]) => unknown>;
let report: Record<string, (...args: any[]) => unknown>;
let lexer: Record<string, (...args: any[]) => unknown>;
let values: Record<string, (...args: any[]) => unknown>;
let versions: Record<string, (...args: any[]) => unknown>;
let wire: Record<string, (...args: any[]) => unknown>;
let inbox: Record<string, (...args: any[]) => unknown>;
let api: Record<string, (...args: any[]) => unknown>;
let dynamic: Record<string, (...args: any[]) => unknown>;
let wide: Record<string, (...args: any[]) => unknown>;
let consts: Record<string, (...args: any[]) => unknown>;
let enums: Record<string, (...args: any[]) => unknown>;
let strings: Record<string, (...args: any[]) => unknown>;
let results: Record<string, (...args: any[]) => unknown>;
let iterators: Record<string, (...args: any[]) => unknown>;
let throws: Record<string, (...args: any[]) => any>;
let builtins: Record<string, (...args: any[]) => any>;
// The multi-file crate: its root, and two of its other modules.
let modules: Record<string, Record<string, (...args: any[]) => number>>;
// Imports from JS modules: the root, and a module two directories down.
let imports: Record<string, Record<string, () => unknown>>;
let asyncs: Record<string, (...args: any[]) => any>;
// What each native case's call did as JS, under Node, at the case's index.
let outcomes: Outcome[];

beforeAll(async () => {
  buildCompiler();
  run([compiler, "examples/fib.rs", "-o", join(target, "fib.js")]);
  fib = await import(join(target, "fib.js"));
  run([compiler, "examples/structs.rs", "-o", join(target, "structs.js")]);
  structs = await import(join(target, "structs.js"));
  run([compiler, "examples/closures.rs", "-o", join(target, "closures.js")]);
  closures = await import(join(target, "closures.js"));
  run([compiler, "examples/collections.rs", "-o", join(target, "collections.js")]);
  collections = await import(join(target, "collections.js"));
  run([compiler, "examples/options.rs", "-o", join(target, "options.js")]);
  options = await import(join(target, "options.js"));
  run([compiler, "examples/methods.rs", "-o", join(target, "methods.js")]);
  methods = await import(join(target, "methods.js"));
  run([compiler, "examples/generic_options.rs", "-o", join(target, "generic_options.js")]);
  genericOptions = await import(join(target, "generic_options.js"));
  run([compiler, "examples/std_traits.rs", "-o", join(target, "std_traits.js")]);
  run([compiler, "examples/combinators.rs", "-o", join(target, "combinators.js")]);
  combinators = await import(join(target, "combinators.js"));
  run([compiler, "examples/text.rs", "-o", join(target, "text.js")]);
  text = await import(join(target, "text.js"));
  run([compiler, "examples/calc.rs", "-o", join(target, "calc.js")]);
  calc = await import(join(target, "calc.js"));
  run([compiler, "examples/numbers.rs", "-o", join(target, "numbers.js")]);
  numbers = await import(join(target, "numbers.js"));
  run([compiler, "examples/inventory.rs", "-o", join(target, "inventory.js")]);
  inventory = await import(join(target, "inventory.js"));
  run([compiler, "examples/queues.rs", "-o", join(target, "queues.js")]);
  queues = await import(join(target, "queues.js"));
  run([compiler, "examples/report.rs", "-o", join(target, "report.js")]);
  report = await import(join(target, "report.js"));
  run([compiler, "examples/lexer.rs", "-o", join(target, "lexer.js")]);
  lexer = await import(join(target, "lexer.js"));
  run([compiler, "examples/values.rs", "-o", join(target, "values.js")]);
  values = await import(join(target, "values.js"));
  run([compiler, "examples/versions.rs", "-o", join(target, "versions.js")]);
  versions = await import(join(target, "versions.js"));
  run([compiler, "examples/wire.rs", "-o", join(target, "wire.js"), "--", ...buildSerde()]);
  wire = await import(join(target, "wire.js"));
  run([compiler, "examples/inbox.rs", "-o", join(target, "inbox.js"), "--", ...buildSerde()]);
  inbox = await import(join(target, "inbox.js"));
  run([compiler, "examples/api.rs", "-o", join(target, "api.js"), "--", ...buildSerde()]);
  api = await import(join(target, "api.js"));
  run([compiler, "examples/dynamic.rs", "-o", join(target, "dynamic.js"), "--", ...buildSerde()]);
  dynamic = await import(join(target, "dynamic.js"));
  run([compiler, "examples/wide.rs", "-o", join(target, "wide.js"), "--", ...buildSerde()]);
  wide = await import(join(target, "wide.js"));
  run([compiler, "examples/consts.rs", "-o", join(target, "consts.js")]);
  consts = await import(join(target, "consts.js"));
  run([compiler, "examples/enums.rs", "-o", join(target, "enums.js")]);
  enums = await import(join(target, "enums.js"));
  run([compiler, "examples/strings.rs", "-o", join(target, "strings.js")]);
  strings = await import(join(target, "strings.js"));
  run([compiler, "examples/results.rs", "-o", join(target, "results.js")]);
  results = await import(join(target, "results.js"));
  run([compiler, "examples/iterators.rs", "-o", join(target, "iterators.js")]);
  iterators = await import(join(target, "iterators.js"));
  run([compiler, "examples/thread_locals.rs", "-o", join(target, "thread_locals.js")]);
  // The webapi crate is used from its metadata (ADR 0024).
  buildWebapi();
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  run([compiler, "examples/counter.rs", "-o", join(target, "counter.js"), ...withWeb]);
  run([compiler, "test/web_forms.rs", "-o", join(target, "web_forms.js"), ...withWeb]);
  run([compiler, "examples/todo.rs", "-o", join(target, "todo.js"), ...withWeb]);
  run([compiler, "examples/countdown.rs", "-o", join(target, "countdown.js"), ...withWeb]);
  run([compiler, "examples/fetch.rs", "-o", join(target, "fetch.js"), ...withWeb]);
  run([compiler, "test/throws.rs", "-o", join(target, "throws.js"), ...withWeb]);
  throws = await import(join(target, "throws.js"));
  run([compiler, "test/builtins.rs", "-o", join(target, "builtins.js"), ...withWeb]);
  builtins = await import(join(target, "builtins.js"));
  // The playground's own Rust (ADRs 0032, 0044), as compile-rust.ts compiles it with
  // rust-js.wasm: with React.
  buildReact();
  // Into an empty folder, so a file an older layout wrote can't pass for its output.
  rmSync(join(target, "playground"), { recursive: true, force: true });
  run([compiler, "wasm/web/rust/lib.rs", "-o", join(target, "playground", "lib.js"),
    ...withWeb, "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  run([compiler, "test/async.rs", "-o", join(target, "async.js"), ...withWeb]);
  asyncs = await import(join(target, "async.js"));
  run([compiler, "examples/modules/lib.rs", "-o", join(target, "modules", "lib.js")]);
  modules = {
    lib: await import(join(target, "modules", "lib.js")),
    stats: await import(join(target, "modules", "stats.js")),
    util: await import(join(target, "modules", "util.js")),
  };
  // Imports (ADR 0028): `./greet.js` is relative to the root's JS, so it goes beside it.
  run([compiler, "test/imports/lib.rs", "-o", join(target, "imports", "lib.js")]);
  copyFileSync(join(root, "test/imports/greet.js"), join(target, "imports", "greet.js"));
  copyFileSync(join(root, "test/imports/inner/wave.js"), join(target, "imports", "inner", "wave.js"));
  imports = {
    lib: await import(join(target, "imports", "lib.js")),
    leaf: await import(join(target, "imports", "inner", "leaf.js")),
  };
  outcomes = callInNode(jsFiles(), cases.map(request), fixture("compiler-calls"));
}, 600_000);

// A native case as a call of the JS, under Node (ADR 0095). `nth` takes an
// enum; in JS a fieldless variant is its name as a string. "modules.summary"
// is the crate root's; "modules.stats.mean" is stats.js's.
function request(c: Case): Call {
  if (c.fn === "nth_asc" || c.fn === "nth_desc") {
    return { module: "fib", fn: "nth", args: [c.fn === "nth_asc" ? "Ascending" : "Descending", ...c.args] };
  }
  const path = c.fn.split(".");
  if (path.length === 1) return { module: "fib", fn: c.fn, args: c.args };
  if (path[0] === "modules") {
    const [file, name] = path.length === 2 ? ["lib", path[1]] : [path[1], path[2]];
    return { module: `modules.${file}`, fn: name, args: c.args };
  }
  return { module: path[0], fn: path[1], args: c.args };
}

// Each example's JS, by the name its cases start with.
function jsFiles(): Record<string, string> {
  const files: Record<string, string> = { harness: join(root, "test/harness-values.ts") };
  for (const { module } of cases.map(request)) {
    if (module.startsWith("modules.")) files[module] = join(target, "modules", `${module.slice("modules.".length)}.js`);
    else files[module] ??= join(target, `${module}.js`);
  }
  return files;
}

test("native Rust ran every case", () => {
  expect(cases.length).toBeGreaterThan(100);
});

// One test per function, reporting every call that differs, not just the first.
for (const [fn, calls] of Map.groupBy(cases.entries(), ([, c]) => c.fn)) {
  test(`${fn} matches native Rust`, () => {
    const differ: { call: string; native: Outcome; js: Outcome }[] = [];
    for (const [index, c] of calls) {
      const js = outcomes[index];
      if (!same(js, expected(c))) {
        differ.push({ call: `${fn}(${c.args.map((a) => Bun.inspect(a)).join(", ")})`, native: expected(c), js });
      }
    }
    expect(differ).toEqual([]);
  });
}

// ADR 0019: one JS file per module, with generated imports and exports.
test("a crate split across files becomes one JS file per module", async () => {
  const out = join(target, "modules");
  const files = [...new Bun.Glob("**/*.js").scanSync(out)].sort();
  // `geometry` only holds other modules, so it gets no file.
  expect(files).toEqual(["geometry/area.js", "geometry/util.js", "lib.js", "stats.js", "util.js"]);

  // Exported: \`pub\` functions, plus private ones another file calls
  // (\`clamp\`, called from child modules). Private and local: not exported.
  expect(Object.keys(modules.lib).sort()).toEqual(["HALVES", "clamp", "doubled_mean", "mixed", "shadowed", "summary"]);
  expect(Object.keys(modules.stats)).toEqual(["mean"]);

  const area = await Bun.file(join(out, "geometry/area.js")).text();
  // Specifiers are relative, and aliases are unique within the file.
  expect(area).toContain('import { clamp } from "../lib.js";');
  expect(area).toContain('import { triple } from "./util.js";');
  expect(area).toContain('import { double } from "../util.js";');
  // Imports are named after locals are known, so locals keep their names.
  expect(area).toContain("const util = (x + 1) >>> 0;");
  expect(area).toContain("return double(util);");
  // lib ↔ stats import each other: a cycle, which Rust and ES modules allow.
  const stats = await Bun.file(join(out, "stats.js")).text();
  expect(stats).toContain('import { HALVES, clamp } from "./lib.js";');
  // A `const` of another module, by its name there.
  expect(stats).toContain("return (x / HALVES) >>> 0;");

  // Each file's source map points into the .rs file its module lives in.
  const sources = async (f: string) => (await Bun.file(join(out, `${f}.map`)).json()).sources;
  expect(await sources("stats.js")).toEqual(["../../examples/modules/stats.rs"]);
  expect(await sources("geometry/area.js")).toEqual(["../../../examples/modules/geometry/area.rs"]);
  // An inline module lives in its parent's file.
  expect(await sources("util.js")).toEqual(["../../examples/modules/lib.rs"]);
});

// ADR 0028: `#[link_name = "module#path"]` imports from a JS module.
test("extern items from JS modules become import statements", async () => {
  // They run: Node's modules, and a hand-written file.
  expect(imports.lib.paths()).toBe("a/b/c.txt");
  expect(imports.lib.file_name()).toBe("y.txt");
  expect(imports.lib.urls()).toEqual(["https://example.com/a", "https://example.com/b"]);
  expect(imports.lib.greetings()).toEqual(["Hello, world!", "Good day, world.", "!?"]);
  expect(imports.leaf.hello()).toBe("Hello, leaf!");
  expect(imports.leaf.bye()).toBe("Bye, leaf!");
  expect(imports.leaf.joined()).toBe("a/leaf");

  const lib = await Bun.file(join(target, "imports", "lib.js")).text();
  // One statement per module and kind, as a person would write them. A
  // default or namespace import is named after its module.
  expect(lib).toContain('import greet, { punctuation } from "./greet.js";\nimport * as greet$1 from "./greet.js";\n');
  expect(lib).toContain('import { join, posix } from "node:path";');
  // Beside the global `URL`, the imported one is renamed.
  expect(lib).toContain('import { URL as URL$1 } from "node:url";');
  expect(lib).toContain('return [new URL$1("https://example.com/a").href, new URL("https://example.com/b").href];');
  // A path goes on from the import, and a local doesn't hide one.
  expect(lib).toContain('return posix.basename("/tmp/x/y.txt");');
  expect(lib).toContain('const join$1 = "c.txt";\n  return join(join("a", "b"), join$1);');
  expect(lib).toContain('greet$1.polite("world")');
  // Two directories down, only what the file uses, from the same file.
  const leaf = await Bun.file(join(target, "imports", "inner", "leaf.js")).text();
  expect(leaf).toMatch(
    /\nimport greet from "\.\.\/greet\.js";\nimport wave from "\.\/wave\.js";\nimport \{ join as join\$1 \} from "node:path";\n\nfunction join/,
  );
  // Its own `join` renames its import of node:path's, not the root's.
  expect(leaf).toContain('return join(join$1("a", "leaf"));');
});

// ADR 0029: `async fn` is an `async function`, `.await` is `await`, and a
// future is a JS promise.
test("async code becomes async functions and await", async () => {
  expect(await asyncs.sum(2, 3)).toBe(10);
  expect(await asyncs.countdown(4)).toBe(4);
  expect(await asyncs.swap([1, 2])).toEqual([2, 1]);
  expect([await asyncs.given({ params: 4 }), await asyncs.given({})]).toEqual([5, 1]);
  expect(await asyncs.blocks(5)).toBe(26);
  expect(await asyncs.held()).toBe(5);
  // The listener heard the first ping, and was gone for the second.
  expect(asyncs.listen_until_aborted()).toBe(1);
  // A spawned task runs up to its first `.await` at once, the rest later.
  const log = asyncs.spawned();
  expect(log.value).toEqual([1, 2]);
  await Bun.sleep(20);
  expect(log.value).toEqual([1, 2, 3]);
  // `window::fetch`, from the webapi crate, and the response's promises.
  // Bun has `fetch`; the webapi crate reaches it through `window`. A POST's
  // answer is its method and body, as the server was sent them.
  const server = Bun.serve({
    port: 0,
    fetch: async (request) => request.method === "POST" ? new Response(`${request.method} ${await request.text()}`) : new Response("hello", { status: 201 }),
  });
  (globalThis as any).window = globalThis;
  try {
    expect(await asyncs.load(server.url.href)).toEqual([201, true, "hello"]);
    // Binary data: `bytes()` and `arrayBuffer()`, five bytes of "hello".
    expect(await asyncs.load_bytes(server.url.href)).toEqual([5, 5, 5]);
    expect(await asyncs.post(server.url.href, "saved")).toBe("POST saved");
  } finally {
    delete (globalThis as any).window;
    server.stop();
  }

  // WebAssembly: a module that imports `env.double` and exports
  // `add(a, b) = double(a + b)`, by hand.
  // prettier-ignore
  const wasm = new Uint8Array([
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,                          // "\0asm", version 1
    0x01, 0x0c, 0x02, 0x60, 0x01, 0x7f, 0x01, 0x7f, 0x60, 0x02, 0x7f, 0x7f, 0x01, 0x7f, // types: (i32) -> i32, (i32, i32) -> i32
    0x02, 0x0e, 0x01, 0x03, 0x65, 0x6e, 0x76, 0x06, 0x64, 0x6f, 0x75, 0x62, 0x6c, 0x65, 0x00, 0x00, // import env.double
    0x03, 0x02, 0x01, 0x01,                                                  // one function, of type 1
    0x07, 0x07, 0x01, 0x03, 0x61, 0x64, 0x64, 0x00, 0x01,                    // export "add"
    0x0a, 0x0b, 0x01, 0x09, 0x00, 0x20, 0x00, 0x20, 0x01, 0x6a, 0x10, 0x00, 0x0b, // a + b, then call double
  ]);
  expect(await asyncs.run_wasm(wasm, 2, 3)).toBe(10);
  expect(await asyncs.instantiate_bytes(wasm)).toBe(true);

  const js = await Bun.file(join(target, "async.js")).text();
  // A namespace's functions, an overload, and a Rust struct as the import object.
  expect(js).toContain("  const module = await WebAssembly.compile(bytes);\n  const imports = { env: { double: (x) => Math.imul(x, 2) } };\n  const instance = await WebAssembly.instantiate(module, imports);\n  return instance.exports.add(a, b);");
  // A dictionary result is a struct: its fields are read as they are.
  expect(js).toContain("source.instance.exports.add(1, 2) === 3");
  expect(js).toContain("export async function sum(a, b) {\n  return ((await double(a)) + (await double(b))) >>> 0;\n}");
  // Parameters are the body's variables: no `let x = x`.
  expect(js).toContain("export async function countdown(n) {\n  let steps = 0;");
  // What's taken apart is taken apart where it's given, as a plain \`fn\` takes it.
  expect(js).toContain("export async function swap([a, b]) {\n  return [await setTimeout(0, b), a];");
  expect(js).toContain("export async function given({ params }) {\n");
  // An `async` block is an async arrow, called; an `async` closure, an async arrow.
  expect(js).toContain("const block = (async () => ((await double(x)) + 1) >>> 0");
  expect(js).toContain("const add = async (y) => ((await setTimeout(0, y)) + x) >>> 0;");
  // A future in a variable is the promise; `.await` on it is `await`.
  expect(js).toContain("const first = setTimeout(5, 1);");
  expect(js).toContain("return ((await first) + (await second)) >>> 0;");

  const countdown = await Bun.file(join(target, "countdown.js")).text();
  expect(countdown).toContain("return new Promise((resolve) => {\n    setTimeout(resolve, ms);\n  });");
  expect(countdown).toContain("    await sleep(500);\n");
  const fetchJs = await Bun.file(join(target, "fetch.js")).text();
  expect(fetchJs).toContain("  const response = await window.fetch(url);\n  const text = await response.text();\n");
  // `spawn(Box::new(load(..)))`: the call is the promise.
  expect(fetchJs).toContain('const URL = "data:text/plain,Hello from a fetch!";');
  expect(fetchJs).toContain("    load(URL, output);\n");
  // `spawn(Box::new(async move { .. }))` is the promise, unawaited.
  expect(countdown).toContain("    (async () => {\n      await count_down(output, 3);\n      running$1.value = false;\n    })();");
});

// ADRs 0032 and 0044: the playground is Rust, React components one per file,
// compiled by rust-js.
test("the playground is Rust components, compiled to the JS main.ts starts", async () => {
  const read = (path: string) => Bun.file(join(target, "playground", path)).text();
  const lib = await read("lib.jsx");
  expect(lib).toContain('import { App } from "./components/app.jsx";');
  expect(lib).toContain("  root.render(\n    <StrictMode>\n      <App />\n    </StrictMode>");
  const app = await read("components/app.jsx");
  expect(app).toContain('import { Pane } from "./pane.jsx";');
  expect(app).toContain('import { Toolbar } from "./toolbar.jsx";');
  expect(app).toContain("<Pane");
  expect(app).toContain("<Toolbar");
  // Each component's file exports it alone, which Fast Refresh needs.
  const components = [
    ["app", "App"], ["editor", "Editor"], ["example_picker", "ExamplePicker"], ["file_item", "FileItem"],
    ["file_tree", "FileTree"], ["pane", "Pane"], ["result_frame", "ResultFrame"], ["stats_table", "StatsTable"],
    ["status_line", "StatusLine"], ["toolbar", "Toolbar"],
  ];
  for (const [file, name] of components) {
    const js = await read(`components/${file}.jsx`);
    expect([...js.matchAll(/^export (?:async )?(?:function|const) (\w+)/gm)].map((m) => m[1])).toEqual([name]);
  }
  // A folder's entries are a FileTree inside it: the component is recursive.
  const { FileTree } = await import(join(target, "playground/components/file_tree.jsx"));
  const children = [["leaf.rs", { TAG: "File", _0: "nested/leaf.rs" }]];
  const onOpen = () => {};
  const folder = FileTree({
    tree: [["nested", { TAG: "Folder", _0: children }]], depth: 2,
    first: "lib.rs", selected: "nested/leaf.rs", onOpen, onDelete: undefined,
  }).props.children[0];
  const nested = folder.props.children.props.children[1].props.children;
  expect(nested.type).toBe(FileTree);
  expect(nested.props).toMatchObject({ tree: children, depth: 3, first: "lib.rs", selected: "nested/leaf.rs", onOpen });
  expect(await read("components/file_tree.jsx")).toContain("export function FileTree({ tree, depth, first, selected, onOpen, onDelete }) {");
  // The editor's view is made in an effect, and destroyed in its cleanup.
  expect(await read("components/editor.jsx")).toContain("    return () => {\n      editor.destroy();");
  // A let chain on `&on_submit`, tested where it is: a reference is the value.
  expect(await read("components/editor.jsx")).toContain('    if (onSubmit != null && (e.metaKey || e.ctrlKey) && e.key === "Enter") {');
  // A hook, found by its name, and props as React code names them (ADR 0046).
  expect(await read("dark_mode.js")).toContain("export function useDarkMode() {\n  return useSyncExternalStore(");

  const compiler = await read("compiler.js");
  expect(compiler).toContain("import {\n  ConsoleStdout,\n  Directory,\n  File,\n  OpenFile,\n  PreopenDirectory,\n  WASI,\n} from \"@bjorn3/browser_wasi_shim\";");
  for (const name of ["load", "loadExample", "ms", "mb", "compile"]) {
    expect(compiler).toMatch(new RegExp(`^export (async )?function ${name}\\(`, "m"));
  }
  // The downloads all start before any is awaited.
  expect(compiler).toContain("  const module = loadCompiler(start, stat);\n  const sysroot = loadSysroot(start, stat);");
  expect(compiler).toContain('  const module = await WebAssembly.compileStreaming(window.fetch("./rust-js.wasm"));');
  // A `format!` value shown once, in order, is written in place.
  expect(compiler).toContain("  return `${t.toFixed(0)} ms`;");
  expect(compiler).toContain("  const response = await window.fetch(`./sysroot/${name}`);");
  // A trapped compile is an `Err` (ADR 0035), and `instanceof` a binding.
  expect(compiler).toContain("  const started = $try(() => wasi.start(instance));");
  expect(compiler).toContain('  const ok = started.TAG === "Ok" && started._0 === 0;');
  expect(compiler).toContain("    if (entry instanceof Directory) {");
  // The file tree: sorted with a comparator, a copy of the tree's entries.
  expect(await read("tree.js")).toContain("  let entries = tree.slice();\n  entries.sort((a, b) => {");
  // The browser links named and namespace imports as real ES modules.
  const programs = await read("programs.js");
  expect(programs).toContain('type="importmap"');
  expect(programs).toContain("encodeURIComponent(body");
  expect(programs).toContain("await import(");
  // CodeMirror, through imports (ADR 0028), its extensions made once (ADR 0037).
  const codemirror = await read("codemirror.js");
  expect(codemirror).toContain('import { EditorView, basicSetup } from "codemirror";');
  expect(codemirror).toContain("const THEME = new Compartment();");
});

// ADR 0037: `thread_local!` is a variable of its module.
test("thread-locals are module variables", async () => {
  const js = await Bun.file(join(target, "thread_locals.js")).text();
  // Each only read and set is the module's variable, a `let` if it's set
  // (ADR 0270); one whose cell `with` hands out keeps its `{ value }`.
  expect(js).toContain("let COUNT = 0;\nconst LOG = [];\nconst START = { value: (Math.imul(10, 4) + 2) | 0 };");
  expect(js).toContain("  COUNT = (COUNT + 1) >>> 0;\n  return COUNT;");
  expect(js).toContain("  })(LOG);");
  // A closure that only returns is its body, on the key or its value, in place.
  expect(js).toContain("  return START.value;");
  expect(js).toContain("  return LOG.length;");
  // Nothing of std's storage.
  expect(js).not.toContain("__rust_std_internal");
});

// ADR 0036: an iterator is a JS array, and `Ordering` a comparator's number.
test("iterators are array methods, and sorting takes comparators", async () => {
  const js = await Bun.file(join(target, "iterators.js")).text();
  expect(js).toContain("  return $range(0, n).map((i) => Math.imul(i, i) >>> 0);");
  // `|&&x|` is `x`: a reference is the value.
  expect(js).toContain("  return v.filter((x) => x % 2 === 0);");
  expect(js).toContain("  const sum = v.reduce((a, b) => (a + b) | 0, 0);");
  expect(js).toContain("  const anyNegative = v.some((x) => x < 0);");
  expect(js).toContain("  return v.slice(1).slice(0, 2).toReversed();");
  expect(js).toContain('  return words.map((w) => w.toUpperCase()).join("-");');
  // `enumerate()` then `map` is one `map`, whose callback JS gives the index too.
  expect(js).toContain("  return words.map((w, i) => `${i}:${w}`);");
  expect(js).toContain("const fits = words.some((w, i) => $byteLen(w) === i);");
  expect(js).toContain('  return Array.from(s).toReversed().join("");');
  // Numbers sort by `a - b`: JS's own `sort()` would compare them as strings.
  expect(js).toContain("  w.sort((a, b) => a - b);");
  expect(js).toContain("  w.sort((a, b) => $cmp(key(a), key(b)));");
  // `then_with` is `||`: `Equal` is 0.
  expect(js).toContain("  w.sort((a, b) => $cmp(!a, !b) || $cmp(a, b));");
  expect(js).toContain("  if (match === -1) {");
});

// ADR 0035: JS that throws, as a `Result`; and `?`.
// JS's own, from the builtins crate (ADR 0102): `JSON.stringify` of a
// string, its JSON text and a JS string literal, and `replace` with a
// string, in which `$1` is a group.
test("the builtins crate's json::stringify and reg_exp::replace are JS's", () => {
  for (const text of ["plain", 'a "quote" and a \\ backslash', "a line\nbreak, a\ttab, a \u0001", "é and 😀", ""]) {
    expect(builtins.quoted(text)).toBe(JSON.stringify(text));
    expect(JSON.parse(builtins.quoted(text))).toBe(text);
  }
  expect(builtins.quoted('say "hi"\n')).toBe('"say \\"hi\\"\\n"');
  expect(builtins.replaced("a-b-c", "-", "g", "+")).toBe("a+b+c");
  expect(builtins.replaced("a-b-c", "-", "", "+")).toBe("a+b-c");
  expect(builtins.replaced("2026-09-30", "(\\d+)-(\\d+)-(\\d+)", "", "$3/$2/$1")).toBe("30/09/2026");
});

// JS's string methods (ADR 0247), as JS has them: by UTF-16 indexes, so an
// emoji is two, and a negative index of `slice` counts from the end.
test("the builtins crate's string functions are JS's string methods", () => {
  const text = " 😀ab ab ";
  const [parts, at, length] = builtins.parts(text, "ab");
  expect(parts).toEqual([text.slice(1, -1), text.slice(-2), text.substring(1, 3), text.substring(1), text.trim(), text.trimStart(), text.trimEnd()]);
  expect(at).toEqual([text.indexOf("ab"), text.indexOf("ab", 2), text.lastIndexOf("ab")]);
  expect(length).toBe(text.length);
  expect(builtins.units("a😀")).toEqual(["a", "\ud83d", "\ude00"]);
  expect(builtins.replaced_first("a-b-c", "-", "+")).toBe("a+b-c");
  expect(builtins.split_parts("a,b;c,")).toEqual([["a", "b;c", ""], ["a", "b", "c", ""]]);
  expect(builtins.revived('{"a":1,"secret":2,"b":{"secret":3}}')).toEqual({ a: 1, b: {} });
  const js = readFileSync(join(target, "builtins.js"), "utf8");
  expect(js).toContain('return [text.split(","), text.split(/[,;]/)];');
  expect(js).toContain("return JSON.parse(text, hidden);");
  expect(js).toContain("  if (key in value) {\n    return value[key];\n  }");
  expect([builtins.named({ a: 1 }, "a"), builtins.named({ a: 1 }, "b")]).toEqual([1, undefined]);
  expect(() => builtins.revived("{")).toThrow();
  expect([builtins.holds_none({ undefined: 1 }, undefined), builtins.holds_none({}, undefined), builtins.holds_none({ a: 1 }, "a")]).toEqual([true, false, true]);
  expect(js).toContain("export function holds_none(value, key) {\n  return key in value;\n}");
  expect(js).toContain("return { children };");
  expect(builtins.wrapped("text")).toEqual({ children: "text" });
  expect(js).toContain("return key in value;");
  expect([builtins.holds({ a: 1 }, "a"), builtins.holds({}, "toString"), builtins.holds({}, "b")]).toEqual([true, true, false]);
  for (const call of ["text.slice(1, -1)", "text.slice(-2)", "text.substring(1, 3)", "text.substring(1)", "text.trim()", "text.trimStart()", "text.trimEnd()", 'text.indexOf(part)', "text.indexOf(part, 2)", "text.lastIndexOf(part)", "text.length", "text.charAt(i)", "text.replace(from, to)"]) {
    expect(js).toContain(call);
  }
});

// A RegExp of a pattern and flags written as they are is a literal, as JS
// writes one, `/%s/g` (ADR 0243); one JS can't parse is made as it was,
// to throw when it runs.
test("a RegExp of a pattern as it is written is a literal", async () => {
  const dir = fixture("regex-literals");
  writeFileSync(join(dir, "lib.rs"), `use js::reg_exp;

pub fn placeholders(text: &str) -> String {
    reg_exp::replace(text, reg_exp::new("%s", "g"), "_")
}

// A literal's \`/\` is escaped, but in a class, where it needn't be.
pub fn slashes(text: &str) -> String {
    reg_exp::replace(text, reg_exp::new("a/b|[/]", "g"), "-")
}

// An empty pattern's literal is \`/(?:)/\`: \`//\` is a comment.
pub fn emptied(text: &str) -> String {
    reg_exp::replace(text, reg_exp::new("", ""), "^")
}

// A line break can't be in a literal.
pub fn joined(text: &str) -> String {
    reg_exp::replace(text, reg_exp::new("\\n", "g"), " ")
}

// One JS can't parse throws as it's made, not as the module is read.
pub fn unparsed() -> bool {
    reg_exp::test(reg_exp::new("(", ""), "(")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('text.replace(/%s/g, "_")');
  expect(js).toContain('text.replace(/a\\/b|[/]/g, "-")');
  expect(js).toContain('text.replace(/(?:)/, "^")');
  expect(js).toContain('new RegExp("\\n", "g")');
  expect(js).toContain('new RegExp("(", "")');
  const lib = await import(join(dir, "lib.js"));
  expect(lib.placeholders("%s and %s")).toBe("_ and _");
  expect(lib.slashes("a/b /")).toBe("- -");
  expect(lib.emptied("x")).toBe("^x");
  expect(lib.joined("a\nb")).toBe("a b");
  expect(() => lib.unparsed()).toThrow(SyntaxError);
});

// `Object.fromEntries` and `Object.is`, and an `Error`'s message, for what
// was thrown (ADR 0102).
test("the builtins crate's object and js_error functions are JS's", async () => {
  expect(builtins.attributes("Rust source", 2)).toEqual({ "aria-label": "Rust source", "aria-level": "2" });
  expect(builtins.identical(true)).toEqual([true, true]);
  expect(builtins.identical(false)).toEqual([false, true]);
  // Its message is the engine's: "URI malformed" in V8, "URI error" in Bun.
  const message = (() => {
    try {
      decodeURIComponent("%E0%A4%A");
    } catch (e) {
      return (e as Error).message;
    }
  })();
  expect(await builtins.thrown()).toEqual([message, "not an Error"]);
  // A future is the promise it is, unawaited where it's handed on.
  expect(await builtins.promised(4)).toBe(8);
  const js = await Bun.file(join(target, "builtins.js")).text();
  expect(js).toContain("const pending = doubled(n);");
});

// Timers, globals of every JS runtime, as ReScript's standard library has
// them (ADR 0102): a closure after a time, or every so often, until cleared.
test("the builtins crate's timers run a closure, and clearing one stops it", async () => {
  await new Promise<void>((resolve) => builtins.after(5, resolve));
  let fired = false;
  builtins.cancelled(5, () => {
    fired = true;
  });
  let ticks = 0;
  const id = builtins.every(5, () => {
    ticks += 1;
    if (ticks === 3) builtins.stop(id);
  });
  await new Promise((resolve) => setTimeout(resolve, 80));
  expect(fired).toBe(false);
  expect(ticks).toBe(3);
});

// `toFixed` is JS's rounding: a tie away from zero, where `format!` rounds it
// to even, as Rust does; `-0` without its sign, and `1e21` and up as
// `String(x)`. Between ties they agree (ADR 0102).
test("the builtins crate's number::to_fixed is JS's toFixed, not format!'s", () => {
  expect(builtins.fixed(2.5, 0)).toEqual(["3", "2"]);
  expect(builtins.fixed(1.25, 1)).toEqual(["1.3", "1.2"]);
  expect(builtins.fixed(-2.5, 0)).toEqual(["-3", "-2"]);
  expect(builtins.fixed(-0, 0)).toEqual(["0", "-0"]);
  expect(builtins.fixed(1e21, 2)[0]).toBe("1e+21");
  expect(builtins.fixed(1234.5678, 2)).toEqual(["1234.57", "1234.57"]);
});

test("a throwing JS call is a Result, and ? returns early", async () => {
  expect(throws.sum_json("[1, 2, 3]")).toEqual({ TAG: "Ok", _0: 6 });
  const bad = throws.sum_json("[1, 2,");
  expect(bad.TAG).toBe("Err");
  expect(bad._0).toStartWith("SyntaxError");
  // `?` hands the caught error on as it is.
  expect(throws.first_twice("[4, 5]")).toEqual({ TAG: "Ok", _0: 8 });
  expect(throws.first_twice("{")._0).toBeInstanceOf(SyntaxError);
  // A rejected promise is an `Err` at its `.await`.
  expect(await throws.settled(false)).toBe("7");
  expect(await throws.settled(true)).toBe("rejected: no");
  // Any promise, settled: its rejection an `Err`, not a throw.
  expect(await throws.settle_either(false)).toBe("8");
  expect(await throws.settle_either(true)).toBe("rejected: no");
  const [encoded, decoded] = throws.uri("a b&ü");
  expect(encoded).toStartWith("a%20b%26%C3%BC URIError");
  expect(decoded).toEqual({ TAG: "Ok", _0: "a b&ü" });

  const js = await Bun.file(join(target, "throws.js")).text();
  expect(js).toContain("  const match = $try(() => JSON.parse(json));");
  expect(js).toContain('  const result = $try(() => JSON.parse(json));\n  if (result.TAG === "Err") {\n    return result;\n  }');
  expect(js).toContain('await $settle(Promise.reject("no"))');
  const results = await Bun.file(join(target, "results.js")).text();
  // On an option, the value keeps the variable's name.
  expect(results).toContain("  const a = half(n);\n  if (a == null) {\n    return undefined;\n  }");
  expect(results).toContain('    r.TAG === "Ok" ? r._0 : 99,');
});

// ADR 0214: an untagged enum is its payload, TS's union: JS passes in what
// it has, each told apart by its runtime kind, and gets back the value.
test("an untagged enum is its payload, told apart by its runtime kind", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  run([compiler, "test/untagged.rs", "-o", join(target, "untagged.js"), ...withWeb]);
  const untagged = await import(join(target, "untagged.js"));
  expect(untagged.kind("a")).toBe("text a");
  expect(untagged.kind(1.5)).toBe("number 1.5");
  expect(untagged.kind(12n)).toBe("big 12");
  expect(untagged.kind(false)).toBe("flag false");
  expect(untagged.kind(["x", "y"])).toBe("names x+y");
  expect(untagged.kind(/x/)).toBe("pattern");
  expect(untagged.kind(new ArrayBuffer(3))).toBe("bytes 3");
  expect(untagged.kind(new Error("e"))).toBe("error");
  expect(untagged.kind(new TypeError("t"))).toBe("type error");
  expect(untagged.kind((n: number) => n + 1)).toBe("step 2");
  expect(untagged.kind({ x: 1, y: 2 })).toBe("point 3");
  expect(untagged.text("a.png")).toBe("a.png");
  expect(untagged.numbers([1, 2])).toEqual([1, 2]);
  expect(untagged.shown(2.5)).toBe("2.5");
  expect([untagged.label("Recap"), untagged.label(["a", {}])]).toEqual(["Link for Recap", "Link for this heading"]);
  expect([untagged.is_other(["x"]), untagged.is_other(3), untagged.is_other("s")]).toEqual([true, true, false]);

  const js = await Bun.file(join(target, "untagged.js")).text();
  expect(js).toContain("src instanceof Error && !(src instanceof TypeError)");
  expect(js).toContain('typeof src === "object" &&\n    !Array.isArray(src) &&\n    !(src instanceof RegExp) &&\n    !(src instanceof ArrayBuffer) &&\n    !(src instanceof Error)\n  ) {');
  expect(js).toContain("export function text(s) {\n  return s;\n}");
  expect(js).toContain("String(n)");
  expect(js).toContain('return typeof value !== "string";');
});

// A slice's `concat` of parts written out is an array of them, spread, as
// JS writes it, and a part written out as an array its items in place.
test("concat of parts written out is an array of them, spread", async () => {
  const js = await Bun.file(join(target, "combinators.js")).text();
  expect(js).toContain("export function joined(path, last) {\n  return [...path, last];\n}");
  expect(js).toContain("[1, 2, 3]];");
});

// What's there's field, or nothing, is an optional chain.
test("a field of an Option's value is an optional chain", async () => {
  const js = await Bun.file(join(target, "combinators.js")).text();
  expect(js).toContain("export function name_of(named) {\n  return named?.name;\n}");
});

// A closure called in place is its value, closures in it too.
test("then of a closure that only returns is its value in place", async () => {
  const js = await Bun.file(join(target, "combinators.js")).text();
  expect(js).toContain("export function bumped(ok, items) {\n  return ok ? items.map((n) => (n + 1) >>> 0) : undefined;\n}");
});

// ADR 0034: strings are JS strings, and their methods JS's.
test("string methods are JS's, and format! is a template literal", async () => {
  const js = await Bun.file(join(target, "strings.js")).text();
  expect(js).toContain("  return `${name}: ${n} item${n === 1 ? \"\" : \"s\"}`;");
  expect(js).toContain("s.startsWith(\"ab\"), s.endsWith(\"c\"), s.includes(\"b/\"), s.includes(\"/\")");
  expect(js).toContain('  return s.replaceAll("/", " / ").replaceAll("a", "A");');
  expect(js).toContain('  return $stripSuffix(file, ".rs") ?? file;');
  expect(js).toContain('  for (const part of path.split("/")) {');
  expect(js).toContain('  const last = path.split("/").at(-1) ?? "";');
  expect(js).toContain('  return pieces.join(" > ");');
  // A string that grows gets a new one each time: JS strings don't change.
  expect(js).toContain('    s += String(i);\n    s += ",";');
  // A `char` is a one-character string.
  expect(js).toContain('  const c = windows ? "\\\\" : "/";');
  // A string literal pattern is `===` on the JS string, without `!= null` in `Some`.
  expect(js).toContain('  }\n  if (s === "abc" || s === "stats.rs") {');
  expect(js).toContain('  if (top === "ab") {');
});

// ADR 0033: enums with fields, in ReScript's shapes.
test("enums with fields are tagged objects, as in ReScript", async () => {
  const js = await Bun.file(join(target, "enums.js")).text();
  // A variant without fields is its name; one with fields, `{ TAG, _0 }` or named fields.
  expect(js).toContain('  return "Empty";');
  expect(js).toContain("  return { TAG: \"Circle\", _0: r };");
  expect(js).toContain("  return { TAG: \"Rect\", w, h };");
  // Matching tests the name, or the `TAG`, then the fields, in place.
  expect(js).toContain('  if (s === "Empty") {\n    return 0;\n  }\n  if (s.TAG === "Circle") {');
  expect(js).toContain("  }\n  if ((s.TAG === \"Rect\" && s.w === 0) || s === \"Empty\") {");
  // Through a reference, with no copy: the reference is the value.
  expect(js).toContain("  if (t.TAG === \"Leaf\") {\n    return t._0;\n  }\n  return (sum(t._0) + sum(t._1)) | 0;");
  // `Result` is ReScript's `result`.
  expect(js).toContain('  if (match.TAG === "Ok") {\n    return match._0;');
});

// ADR 0031: a `const` is the value rustc computed, under its own name.
test("constants are the values rustc computed, by name", async () => {
  const js = await Bun.file(join(target, "consts.js")).text();
  expect(js).toContain("export const SIZE = 4096;\nconst GREETING = \"hello\";\nconst RATIO = 0.25;\nconst ON = true;");
  expect(js).toContain('const NOTHING = undefined;\nconst LEVEL = "High";');
  // A `const` inside a function goes beside it.
  expect(js).toContain("const STEP = 3;");
  // Each use is a value of its own: copied where it's changed.
  expect(js).toContain("  let p = { ...ORIGIN };\n");
  expect(js).toContain("  return [{ ...p }, { ...ORIGIN }];");
  // A known divisor needs no check for zero.
  expect(js).toContain("  return (SIZE / 1024) >>> 0;");
  // std's are written in place.
  expect(js).toContain("  return [4294967295, -2147483648];");
  expect(js).toContain("  for (const p of PRIMES) {");
});

// ADR 0030: `Some(x)` is `x`, `None` is `undefined`, and `null` counts as `None`.
test("options are the value or undefined", async () => {
  const js = await Bun.file(join(target, "options.js")).text();
  expect(js).toContain("    return (n / 2) | 0;\n  }\n  return undefined;");
  // `Some(0)` needs no `!= null`; `Some(n)` does.
  expect(js).toContain("  if (o === 0) {\n    return 100;\n  }\n  if (o != null && o < 0) {");
  // `if let Some(h) = ..` keeps the value in a `const h`.
  expect(js).toContain("  const h = half(n);\n  if (h != null) {\n    return h;");
  expect(js).toContain("h != null, h == null, h ?? -1");
  // `unwrap_or`'s argument runs even when it isn't needed, as in Rust.
  expect(js).toContain("  const option = half(n);\n  const fallback = bump();\n  const v = option ?? fallback;");
  expect(js).toContain('  return $unwrap(half(n), "an even number");');
  // `==` on options is `==`: `null` from JS equals `undefined` from Rust.
  expect(js).toContain("  return a == b;");
  // `map` puts the closure's body in place, on the option read once.
  expect(js).toContain("h != null ? double(h) : undefined");
  expect(js).toContain("h != null ? h > 2 : undefined");
  expect(js).toContain("option != null ? Math.imul(option[0], option[1]) : undefined");
  // Let chains (ADR 0048): one test when the parts need nothing else,
  expect(js).toContain("  const h = half(n);\n  if (h != null && h > 2) {\n    return h;\n  }\n  return -1;");
  // an `if` inside for a `let` of a call, only made once the rest held, and
  // an `else` of one statement at each level a test fails at, as a person
  // writes it;
  expect(js).toContain("  const h = half(n);\n  if (h != null && h !== 0) {\n    const q = counted(h);\n    if (q != null && q > 1) {\n      v = q;\n    } else {\n      v = 0;\n    }\n  } else {\n    v = 0;\n  }");
  // a longer one after both, in a block the `then` leaves.
  expect(js).toContain("  chain: {\n    const h = half(n);\n    if (h != null) {\n      const q = counted(h);\n      if (q != null && q > 1) {\n        return q;\n      }\n    }\n    calls.value = (calls.value + 10) | 0;\n    return -calls.value | 0;\n  }");
  expect([options.chained_long_else(8), options.chained_long_else(4), options.chained_long_else(3)]).toEqual([2, -11, -10]);
  // A closure of statements is called, by a name.
  expect(js).toContain("  const counted = h != null ? map(h) : undefined;");
  expect(options.same(null, undefined)).toBe(true);
  expect(options.describe(null)).toBe(0);
});

// ADR 0020: structs are objects, tuples are arrays, and only some reads copy.
test("structs and tuples are plain objects and arrays", async () => {
  const js = await Bun.file(join(target, "structs.js")).text();
  // A JS caller builds the same shapes by hand.
  expect(structs.area({ origin: { x: 0, y: 0 }, size: [3, 4] })).toBe(12);
  expect(structs.classify([5, 5])).toBe(2);

  // `Point` is Copy and changed in place in this crate, so reading one copies it...
  expect(js).toContain("let b = { ...a };");
  expect(js).toContain("origin: { ...a },");
  // ...but `Rect` isn't Copy: assigning it moves it, with no copy.
  expect(js).toContain("let s = r;");
  // Returning a variable hands it over.
  expect(js).toContain("return p;");
  // The tuple `(u32, u32)` is never changed in place, so it is taken apart as it is.
  expect(js).toContain("const [q, r] = divmod(a, b);");
  // Fields are listed in declaration order, but the calls run in the order written.
  expect(js).toMatch(/const y = \$div\(100, a, -2147483648\) \| 0;\s+const x = \$rem/);
  // A tuple parameter is taken apart where it is, and `match (a, b)` tests its
  // parts directly, without building an array.
  expect(js).toContain("export function classify([a, b]) {\n  if (a === 0 && b === 0) {");
});

// The counter's JS reads like the Rust: methods, properties, globals, and one
// shared `{ value }`. No wrappers from the webapi crate.
test("the counter's JS is plain DOM code", async () => {
  const js = await Bun.file(join(target, "counter.js")).text();
  expect(js).toContain('const b = document.createElement("button");');
  expect(js).toContain("b.textContent = label;");
  expect(js).toContain("const count = { value: 0 };");
  expect(js).toContain('b.addEventListener("click", () => {');
  expect(js).toContain("count.value = (count.value + by) | 0;");
  expect(js).toContain("output.textContent = String(count.value);");
  expect(js).toContain("app.append(output);");
});

// Each `#[link_name]` form the webapi crate uses (ADR 0024), in test/web_forms.rs.
test("the webapi crate's bindings become plain JS", async () => {
  const js = await Bun.file(join(target, "web_forms.js")).text();
  // A cast is the value itself; a setter, an assignment; a getter, a read.
  expect(js).toContain('const input = document.createElement("input");');
  expect(js).toContain('input.value = "typed";');
  // A result that may be `null` is an `Option` (ADR 0030), unwrapped here.
  expect(js).toContain('const app = $unwrap(document.getElementById("app"), "the page has an #app");');
  expect(js).toContain("return $unwrap(app.textContent) + input.value;");
  // A union is its untagged enum, whose value is the member itself (ADR 0215).
  expect(js).toContain("app.append(input);");
  expect(js).toContain('app.append("!");');
  // Constructors, and a global used as an `EventTarget` through `Deref`.
  expect(js).toContain('const ping = new Event("ping");');
  // A closure returning \`()\` is a block body: JS gets no return value Rust didn't have.
  expect(js).toContain('app.addEventListener("ping", (e) => {\n    e.preventDefault();\n  });');
  expect(js).toContain("window.dispatchEvent(ping);");
  // Optional arguments: `encode_with_input`, and a union's enum.
  expect(js).toContain('const bytes = new TextEncoder().encode(text);');
  expect(js).toContain('const back = new TextDecoder("utf-8").decode(bytes);');
  // An iframe's window is a `Window` (a WindowProxy, in WebIDL); a message's
  // sender, a union, an object; and `performance` of hr-time.
  expect(js).toContain("const sender = e.source;\n  const windowOf = frame.contentWindow;\n  return [Object.is(sender, windowOf), window.performance.now()];");
  // A body of a `Blob` and a fetch of a URL, each the value itself.
  expect(js).toContain("const blob = new Blob();\n  const response = new Response(blob);\n  return [response, window.fetch(url)];");
  // A sequence is a slice, its items as they are (ADR 0219).
  expect(js).toContain('const blob = new Blob([text, "!"]);\n  return [blob, navigator.clipboard.write(items)];');
  // A canvas's size, its setters and getters (HTMLCanvasElement).
  expect(js).toContain('const canvas = document.createElement("canvas");\n  canvas.width = 320;\n  canvas.height = 200;\n  return [canvas.width, canvas.height];');
  const { round_trip } = await import(join(target, "web_forms.js"));
  // "é" is two bytes in UTF-8.
  expect(round_trip("héllo")).toEqual([6, "héllo"]);
});


// ADR 0223: an event's name gives its listener the event its target takes,
// and a tag's name the element it makes, from `@webref/events` and
// `@webref/elements`, as TypeScript's `HTMLElementEventMap` and
// `HTMLElementTagNameMap` do. Each name is a type whose value is its string.
test("webapi's event and tag maps type a listener and an element", () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-maps");
  const source = `use webapi::events::{Click, Keydown};
use webapi::tags::Button;
use webapi::{document, event_target, html_button_element, keyboard_event, mouse_event};
pub fn wire() -> &'static webapi::HTMLButtonElement {
    let button = document::create_element(document, Button);
    html_button_element::set_disabled(button, false);
    event_target::add_event_listener(button, Click, Box::new(|e| {
        let _ = mouse_event::client_x(e);
    }));
    event_target::add_event_listener(document, Keydown, Box::new(|e| {
        let _ = keyboard_event::key(e);
    }));
    button
}
`;
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('const button = document.createElement("button");');
  expect(js).toContain("button.disabled = false;");
  expect(js).toContain('button.addEventListener("click", (e) => {');
  expect(js).toContain('document.addEventListener("keydown", (e) => {');
  // A key's event isn't a mouse's, and a button takes no window's message.
  for (const [wrong, error, message] of [
    ["add_event_listener(document, Keydown, Box::new(|e| {\n        let _ = keyboard_event::key(e);", "add_event_listener(document, Keydown, Box::new(|e| {\n        let _ = mouse_event::client_x(e);", "expected `&MouseEvent`, found `&KeyboardEvent`"],
    ["use webapi::events::{Click, Keydown};", "use webapi::events::{Click, Keydown, Message};\nfn message(b: &webapi::HTMLButtonElement) { event_target::add_event_listener(b, Message, Box::new(|_| ())); }", "the trait `webapi::Listen<webapi::events::Message>` is not implemented for `webapi::HTMLButtonElement`"],
  ]) {
    writeFileSync(join(dir, "lib.rs"), source.replace(wrong, error));
    const result = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "wrong.js"), ...withWeb], { cwd: dir });
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toString()).toContain(message);
  }
});

// SVG's elements are bound, and each SVG tag is a type of its own, as
// TypeScript's `SVGElementTagNameMap` has them: `createElementNS` of SVG's
// namespace and a tag makes its element, `svg_tags::Circle` an
// `SVGCircleElement`, which is an `SVGGeometryElement`, an
// `SVGGraphicsElement`, an `SVGElement` and an `Element`.
test("webapi's SVG elements are made by their tags", () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-svg");
  writeFileSync(join(dir, "lib.rs"), `use webapi::namespaces::Svg;
use webapi::svg_tags::{Circle, Svg as SvgTag};
use webapi::{SVGCircleElement, SVGSVGElement, document, element, svg_geometry_element};
pub fn draw() -> (&'static SVGSVGElement, &'static SVGCircleElement, bool) {
    let svg = document::create_element_ns(document, Svg, SvgTag);
    let circle = document::create_element_ns(document, Svg, Circle);
    element::set_attribute(circle, "r", "4");
    element::append(svg, circle);
    (svg, circle, svg_geometry_element::is_point_in_fill(circle))
}
pub fn named() -> &'static webapi::Element {
    document::create_element_ns_named(document, "http://www.w3.org/2000/svg", "g")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");');
  expect(js).toContain('const circle = document.createElementNS("http://www.w3.org/2000/svg", "circle");');
  expect(js).toContain("return [svg, circle, circle.isPointInFill()];");
  expect(js).toContain('return document.createElementNS("http://www.w3.org/2000/svg", "g");');
});

// ADR 0229: a binding's union parameter is `impl` a sealed trait of its
// members, so each member is passed as it is, `upload("hello")`, with no
// `.into()` and nothing in the JS; another type is an error that names the
// union, and the enum of one, `BodyInit::of`, is matched.
test("a union parameter takes each member as it is", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("union-parameters");
  const source = `use webapi::{Blob, BodyInit, Element, IntoBodyInit, Response, element, response, window};

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum UploadBody<'a> {
    Text(&'a str),
    Blob(&'a Blob),
}

impl<'a> From<&'a str> for UploadBody<'a> {
    fn from(value: &'a str) -> Self {
        UploadBody::Text(value)
    }
}

impl<'a> From<&'a Blob> for UploadBody<'a> {
    fn from(value: &'a Blob) -> Self {
        UploadBody::Blob(value)
    }
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for &str {}
    impl Sealed for &webapi::Blob {}
    impl Sealed for super::UploadBody<'_> {}
    impl<T: Sealed> Sealed for Option<T> {}
}

#[diagnostic::on_unimplemented(message = "\`{Self}\` is not a \`string | Blob\`")]
pub trait IntoUploadBody: sealed::Sealed {}
impl IntoUploadBody for &str {}
impl IntoUploadBody for &Blob {}
impl IntoUploadBody for UploadBody<'_> {}
impl<T: IntoUploadBody> IntoUploadBody for Option<T> {}

impl<'a> UploadBody<'a> {
    #[cfg_attr(rust_js, rust_js::link_name = "this")]
    #[allow(unused_variables)]
    pub fn of(this: impl IntoUploadBody + 'a) -> UploadBody<'a> {
        unreachable!()
    }
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.upload")]
#[allow(unused_variables)]
pub fn upload(body: impl IntoUploadBody) {
    unreachable!()
}

pub fn send(blob: &Blob, body: UploadBody, maybe: Option<&str>) {
    upload("hello");
    upload(blob);
    upload(body);
    upload(maybe);
}

pub fn kind<'a>(body: impl IntoBodyInit + 'a) -> String {
    match BodyInit::of(body) {
        BodyInit::Str(text) => text.to_string(),
        BodyInit::Blob(_) => "blob".to_string(),
        _ => "other".to_string(),
    }
}

pub fn page(el: &Element, blob: &Blob, url: &str) -> (&'static Response, js::Promise<&'static Response>) {
    element::before(el, "text");
    (response::new_with_body(blob), window::fetch(window, url))
}
`;
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[package.metadata.rust-js]\ndeclarations = true\n');
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  // To TypeScript, the trait is the union.
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain("export function kind(body: ReadableStream | Blob | Uint8Array | ArrayBuffer | FormData | string): string;");
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('globalThis.upload("hello");\n  globalThis.upload(blob);\n  globalThis.upload(body);\n  globalThis.upload(maybe);');
  // Of webapi's union, a binding crate's trait, no dictionary: `kind(body)`.
  expect(js).toContain("export function kind(body) {\n  const match = body;\n  if (typeof match === \"string\") {");
  expect(js).toContain('el.before("text");');
  expect(js).toContain("return [new Response(blob), window.fetch(url)];");
  const { send, kind } = await import(join(dir, "lib.js"));
  const sent: unknown[] = [];
  const previous = (globalThis as any).upload;
  (globalThis as any).upload = (body: unknown) => sent.push(body);
  try {
    const blob = new Blob(["b"]);
    send(blob, "body", undefined);
    expect(sent).toEqual(["hello", blob, "body", undefined]);
  } finally {
    (globalThis as any).upload = previous;
  }
  expect([kind("text"), kind(new Blob())]).toEqual(["text", "blob"]);
  writeFileSync(join(dir, "lib.rs"), source.replace('upload("hello");', "upload(1.5);"));
  const wrong = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "wrong.js"), ...withWeb], { cwd: dir });
  expect([wrong.exitCode === 0, wrong.stderr.toString().includes("is not a `string | Blob`")], wrong.stderr.toString()).toEqual([false, true]);
});

// ADR 0260: a binding's callback parameter is a closure, given what JS
// calls it with as a function's parameters are, as react.dev's TopNav
// watches whether the page has scrolled.
test("a binding's callback parameter is a closure", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("callback-parameters");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{Element, IntersectionObserver, IntersectionObserverInit, intersection_observer, intersection_observer_entry, window};

pub fn watch(target: &'static Element, seen: &'static dyn Fn(bool)) -> &'static IntersectionObserver {
    let observer = intersection_observer::new_with_options(
        Box::new(move |entries, _| {
            entries.iter().for_each(|entry| seen(intersection_observer_entry::is_intersecting(entry)));
        }),
        IntersectionObserverInit {
            root_margin: Some("0px 0px"),
            threshold: Some(0.0.into()),
            ..Default::default()
        },
    );
    intersection_observer::observe(observer, target);
    observer
}

pub fn next_frame(then: &'static dyn Fn(f64)) -> u32 {
    window::request_animation_frame(window, Box::new(move |time| then(time)))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("new IntersectionObserver(");
  expect(js).toContain("return window.requestAnimationFrame((time) => {\n    then(time);\n  });");
  const globals = globalThis as any;
  const previous = [globals.IntersectionObserver, globals.window];
  const given: unknown[] = [];
  globals.IntersectionObserver = class {
    constructor(private callback: (entries: unknown[], observer: unknown) => void, options: unknown) {
      given.push(options);
    }
    observe(target: unknown) {
      this.callback([{ isIntersecting: false, target }, { isIntersecting: true, target }], this);
    }
  };
  globals.window = { requestAnimationFrame: (callback: (time: number) => void) => (callback(16), 7) };
  try {
    const { watch, next_frame } = await import(join(dir, "lib.js"));
    const seen: unknown[] = [];
    watch({}, (value: boolean) => seen.push(value));
    expect([seen, given]).toEqual([[false, true], [{ rootMargin: "0px 0px", threshold: 0 }]]);
    const times: number[] = [];
    expect([next_frame((time: number) => times.push(time)), times]).toEqual([7, [16]]);
  } finally {
    [globals.IntersectionObserver, globals.window] = previous;
  }
});

// ADR 0270: a thread-local only read and set, in its own module, is the
// module's variable, a `let` if it's set, as react.dev's errors page caches
// the codes it fetched: nothing shares its cell, so it needs no `{ value }`.
// One another module may read, or whose cell `with` hands out, keeps it.
test("a thread-local only read and set is the module's let", async () => {
  const dir = fixture("module-let");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::{Cell, RefCell};

thread_local! {
    static COUNT: Cell<u32> = Cell::new(0);
    static LOG: RefCell<Vec<String>> = RefCell::new(Vec::new());
    pub static SHARED: Cell<u32> = Cell::new(0);
    static SEEN: Cell<Option<u32>> = const { Cell::new(None) };
}

pub fn seen(n: u32) -> u32 {
    if SEEN.get().is_none() {
        SEEN.set(Some(n));
    }
    SEEN.get().unwrap_or(0)
}

pub fn unseen(n: u32) -> u32 {
    let mut other = None;
    if SEEN.get().is_none() {
        other = Some(n);
    }
    other.unwrap_or(0)
}

pub fn bump() -> u32 {
    COUNT.set(COUNT.get() + 1);
    COUNT.get()
}

pub fn note(line: &str) -> usize {
    LOG.with_borrow_mut(|log| log.push(line.to_string()));
    LOG.with_borrow(|log| log.len())
}

pub fn shared() -> u32 {
    SHARED.set(SHARED.get() + 2);
    SHARED.get()
}

mod counter;

pub fn ticked() -> u32 {
    counter::tick();
    counter::TICKS.get()
}
`);
  writeFileSync(join(dir, "counter.rs"), `use std::cell::Cell;

thread_local! {
    pub(crate) static TICKS: Cell<u32> = Cell::new(0);
}

pub fn tick() {
    TICKS.set(TICKS.get() + 1);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect(readFileSync(join(dir, "counter.js"), "utf8")).toContain("export const TICKS = { value: 0 };");
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("let COUNT = 0;");
  expect(js).toContain("const LOG = [];");
  expect(js).toContain("export const SHARED = { value: 0 };");
  expect(js).toContain("COUNT = (COUNT + 1) >>> 0;");
  expect(js).not.toContain("COUNT.value");
  expect(js).not.toContain("LOG.value");
  // One made by a \`const { .. }\` is too, by its own name.
  expect(js).not.toContain("__RUST_STD_INTERNAL_INIT");
  expect(js).toContain("let SEEN;\n");
  expect(js).toContain("  SEEN ??= n;\n");
  const { seen } = await import(join(dir, "lib.js"));
  expect([seen(3), seen(4)]).toEqual([3, 3]);
  const { unseen } = await import(join(dir, "lib.js"));
  expect(unseen(5)).toBe(0);
  const { bump, note, shared, ticked } = await import(join(dir, "lib.js"));
  expect([bump(), bump(), note("a"), note("b"), shared(), shared(), ticked()]).toEqual([1, 2, 1, 2, 2, 4, 1]);
});

// ADR 0269: what every JS global scope has, a window's, a worker's or Node's,
// `fetch` and `queueMicrotask`, is called bare, as react.dev's errors page
// fetches in Next.js's `getStaticProps`, where there's no `window`.
test("the global scope's functions are called bare", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("global-scope");
  writeFileSync(join(dir, "lib.rs"), `use js::Promise;
use webapi::{Response, global};

pub fn codes(url: &str) -> Promise<&'static Response> {
    global::fetch(url)
}

pub fn later(then: &'static dyn Fn()) {
    global::queue_microtask(Box::new(move || then()));
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return fetch(url);");
  expect(js).toContain("queueMicrotask(");
  expect(js).not.toContain("window");
  const fetched: string[] = [];
  const previous = globalThis.fetch;
  globalThis.fetch = ((url: string) => (fetched.push(url), Promise.resolve(new Response("{}")))) as typeof fetch;
  try {
    const { codes, later } = await import(join(dir, "lib.js"));
    await codes("https://example.com/codes.json");
    const ran: number[] = [];
    later(() => ran.push(1));
    await Promise.resolve();
    expect([fetched, ran]).toEqual([["https://example.com/codes.json"], [1]]);
  } finally {
    globalThis.fetch = previous;
  }
});

// ADR 0273: a `#[path]` module's JS is beside its file, `x/[y].rs`'s
// `x/[y].js`, as Next.js finds a page, and what it imports is seen from
// there: the crate's root, and a file beside the root's.
test("a #[path] module's JS is beside its file", async () => {
  const dir = fixture("path-module");
  mkdirSync(join(dir, "x"));
  writeFileSync(join(dir, "lib.rs"), `#[path = "x/[y].rs"]
pub mod y;

pub fn top() -> u32 {
    y::f() + 1
}

pub fn base() -> u32 {
    40
}
`);
  writeFileSync(join(dir, "x/[y].rs"), `unsafe extern "Rust" {
    #[link_name = "./data.js#default"]
    safe static data: u32;
}

pub fn f() -> u32 {
    crate::base() + data
}
`);
  writeFileSync(join(dir, "data.js"), "export default 1;\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain('import { f } from "./x/[y].js";');
  const y = readFileSync(join(dir, "x/[y].js"), "utf8");
  expect([y.includes('import { base } from "../lib.js";'), y.includes('import data from "../data.js";')]).toEqual([true, true]);
  const { top } = await import(join(dir, "lib.js"));
  expect(top()).toBe(42);
});

// ADR 0035: what a JS function threw is shown as JS shows it, `String(error)`,
// `SyntaxError: ..`, by `{:?}` and by an `unwrap`'s panic.
test("a JS error is shown as JS shows it", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-error");
  writeFileSync(join(dir, "lib.rs"), `use js::{Unknown, json};
pub fn parsed(text: &str) -> Option<&'static Unknown> {
    json::parse(text).unwrap()
}
pub fn kept(text: &str) -> Option<&'static Unknown> {
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(_) => {
            eprintln!("invalid");
            None
        }
    };
    value
}
pub fn valid(text: &str) -> bool {
    match json::parse(text) {
        Ok(_) => true,
        Err(_) => false,
    }
}
pub fn told(text: &str) -> String {
    let told = match json::parse(text) {
        Ok(_) => String::new(),
        Err(error) => format!("{error:?}"),
    };
    told
}
pub fn kept_or_told(text: &str) -> Option<&'static Unknown> {
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error:?}");
            None
        }
    };
    value
}
pub fn shown(text: &str) -> String {
    match json::parse(text) {
        Ok(_) => "parsed".to_string(),
        Err(error) => format!("{error:?}"),
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return String(match._0);");
  // A \`match\` of what it threw, whose \`Ok\` only keeps the value and
  // whose \`Err\` reads no error, is JS's \`try\` (ADR 0035).
  expect(js).toContain('  let value;\n  try {\n    value = JSON.parse(text);\n  } catch {\n    console.error("invalid");\n    value = undefined;\n  }');
  const lib = await import(join(dir, "lib.js"));
  expect(lib.parsed("[1]")).toEqual([1]);
  expect([lib.kept("[1]"), lib.kept("{")]).toEqual([[1], undefined]);
  // One whose \`Err\` reads the error keeps it, \`$try\`'s.
  expect([lib.kept_or_told("[1]"), lib.kept_or_told("{")]).toEqual([[1], undefined]);
  expect([lib.told("1"), lib.told("{").startsWith("SyntaxError")]).toEqual(["", true]);
  expect([lib.valid("[1]"), lib.valid("{")]).toEqual([true, false]);
  expect(() => lib.parsed("{")).toThrow("called `Result::unwrap()` on an `Err` value: SyntaxError");
  expect([lib.shown("{").startsWith("SyntaxError: "), lib.shown("1")]).toEqual([true, "parsed"]);
});

// ADR 0275: a \`#[rust_js::nullable]\` field is TypeScript's \`T | null\`: its
// \`None\` is \`null\`, as Next.js's \`getStaticProps\` gives react.dev's errors
// page its \`errorCode\`, which JSON has no \`undefined\` for. Another such
// field's value is one already, passed on as it is.
test("a nullable field's None is null", async () => {
  const dir = fixture("nullable-null");
  writeFileSync(join(dir, "lib.rs"), `pub struct Props {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub code: Option<String>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub message: Option<String>,
    pub title: Option<String>,
}

pub fn none() -> Props {
    Props { code: None, message: Some("m".to_string()), title: None }
}

pub fn given(code: Option<String>) -> Props {
    Props { code, message: None, title: None }
}

pub fn passed(Props { code, message, .. }: Props) -> Props {
    Props { code, message, title: None }
}

pub fn chosen(code: Option<&str>) -> Props {
    Props {
        code: match code {
            Some(code) if !code.is_empty() => Some(code.to_string()),
            _ => None,
        },
        message: None,
        title: None,
    }
}

pub fn moved(p: Props) -> Props {
    Props { code: p.code, message: Some(p.title.unwrap_or_default()), title: None }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return { code: null, message: "m", title: undefined };');
  expect(js).toContain("return { code: code ?? null, message: null, title: undefined };");
  expect(js).toContain("return { code, message, title: undefined };");
  // A conditional of \`Some\` or \`None\` is one of the value or \`null\`.
  expect(js).toContain("return { code: code || null, message: null, title: undefined };");
  expect(js).toContain('return { code: p.code, message: p.title ?? "", title: undefined };');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.given(undefined), lib.given("1")]).toEqual([{ code: null, message: null, title: undefined }, { code: "1", message: null, title: undefined }]);
  expect(JSON.stringify(lib.passed(lib.given(undefined)))).toBe('{"code":null,"message":null}');
  expect([lib.chosen("1").code, lib.chosen("").code, lib.chosen(undefined).code]).toEqual(["1", null, null]);
});

// ADR 0277: \`let n = n;\`, a variable shadowed by its own value, is \`n\`
// itself where nothing sets \`n\` while the new one is read: no \`n$1\`, as
// react.dev's errors page has \`<Type />\` of \`let Type = from_unknown(Type)\`.
// One of another name is the name written; one a loop sets again keeps its own.
test("a variable shadowed by its own value is the variable", async () => {
  const dir = fixture("shadows");
  writeFileSync(join(dir, "lib.rs"), `pub fn captured(text: String) -> Box<dyn Fn() -> String> {
    let text = text.clone();
    Box::new(move || text.clone())
}
pub fn kept(text: String) -> String {
    let kept = text;
    kept + "!"
}
pub fn before(flag: bool) -> u32 {
    let mut n = 1;
    if flag {
        n = 2;
    }
    let n = n;
    n + 1
}
pub fn looped() -> u32 {
    let mut fs: Vec<Box<dyn Fn() -> u32>> = Vec::new();
    let mut n = 0;
    while n < 2 {
        n += 1;
        let n = n;
        fs.push(Box::new(move || n));
    }
    fs[0]()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function captured(text) {\n  return () => text;\n}");
  expect(js).toContain("  const kept$1 = text;\n");
  expect(js).toContain("  return (n + 1) >>> 0;\n");
  expect(js).toContain("    const n$1 = n;\n    fs.push(() => n$1);");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.captured("a")(), lib.kept("b"), lib.before(true), lib.before(false), lib.looped()]).toEqual(["a", "b!", 3, 2, 1]);
});

// ADR 0272: Node's modules, as @types/node types them, are the JS a person
// writes in Node: an import of `fs`'s, and `process`, a global, as
// react.dev's errors page reads its Markdown in `getStaticProps`.
test("node's modules are imported, and its globals called", async () => {
  const dir = fixture("node");
  run(["node/build.sh", "-o", join(dir, "libnode.rmeta")]);
  writeFileSync(join(dir, "lib.rs"), `use node::{BufferEncoding, fs, process};

pub fn read(path: &str) -> String {
    match fs::read_file_sync(path, BufferEncoding::Utf8) {
        Ok(text) => text,
        Err(_) => "missing".to_string(),
    }
}

pub fn here() -> String {
    process::cwd()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(dir, "libnode.rmeta")}`, "-L", dir]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('import { readFileSync } from "fs";');
  // A \`match\` of what it threw is JS's \`try\`, as the page has it (ADR 0035).
  expect(js).toContain('export function read(path) {\n  try {\n    return readFileSync(path, "utf8");\n  } catch {\n    return "missing";\n  }\n}');
  expect(js).toContain("return process.cwd();");
  writeFileSync(join(dir, "note.md"), "# Note");
  const { read, here } = await import(join(dir, "lib.js"));
  expect([read(join(dir, "note.md")), read(join(dir, "none.md")), here()]).toEqual(["# Note", "missing", process.cwd()]);
});

// ADR 0214: an untagged enum only made, never told apart, may have variants
// of one kind, as Next.js's `getStaticProps` gives `{ props }` or
// `{ notFound: true }`, react.dev's errors page's: each is its payload.
test("an untagged enum only made may hold objects in two variants", async () => {
  const dir = fixture("untagged-objects");
  writeFileSync(join(dir, "lib.rs"), `pub struct Found {
    pub props: u32,
}

pub struct Missing {
    #[cfg_attr(rust_js, rust_js::name = "notFound")]
    pub not_found: bool,
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Page {
    Found(Found),
    Missing(Missing),
}

pub fn page(code: u32) -> Page {
    if code == 0 { Page::Missing(Missing { not_found: true }) } else { Page::Found(Found { props: code }) }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("{ notFound: true }");
  expect(js).toContain("{ props: code }");
  const { page } = await import(join(dir, "lib.js"));
  expect([page(0), page(3)]).toEqual([{ notFound: true }, { props: 3 }]);
});

// ADR 0271: JS's truthiness of a value, `!value` as a person tests one, and
// a value of any shape given a type it's vouched to have, the value itself.
test("a JS value is tested as JS tests it, and cast as it's vouched", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("truthy");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;
pub fn missing(value: Option<&Unknown>) -> bool {
    !js::truthy(value)
}
pub fn present(value: Option<&Unknown>) -> bool {
    js::truthy(value)
}
pub fn shown(value: Option<&Unknown>) -> &'static str {
    if js::truthy(value) { "yes" } else { "no" }
}
pub fn named(name: Option<&str>) -> &'static str {
    if !js::truthy(name) { "none" } else { "some" }
}
pub fn text(value: Option<&'static Unknown>) -> Option<String> {
    unsafe { js::cast(value) }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function missing(value) {\n  return !value;\n}");
  expect(js).toContain("export function present(value) {\n  return !!value;\n}");
  expect(js).toContain('export function shown(value) {\n  if (value) {\n    return "yes";\n  }');
  expect(js).toContain('export function named(name) {\n  if (!name) {\n    return "none";\n  }');
  expect(js).toContain("export function text(value) {\n  return value;\n}");
  const lib = await import(join(dir, "lib.js"));
  const values = [undefined, null, 0, "", false, NaN, "a", 1, {}, [], -1];
  expect(values.map(lib.missing)).toEqual([true, true, true, true, true, true, false, false, false, false, false]);
  expect(values.map(lib.present)).toEqual(values.map(Boolean));
  expect(values.map(lib.shown)).toEqual(values.map((v) => (v ? "yes" : "no")));
  expect(["", "a", undefined].map(lib.named)).toEqual(["none", "some", "none"]);
  expect([lib.text("x"), lib.text(undefined)]).toEqual(["x", undefined]);
});

// ADR 0225: a JS value of unknown shape is a `js::Unknown`, as TypeScript's
// `unknown` and ReScript's are, which `classify` tells by `typeof`, and
// whose properties are read and set by name, as `obj[key]` is.
test("an unknown JS value is classified, and its properties read by name", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("unknown");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Promise, Unknown, classify, json, object};
fn show(value: Option<&Unknown>) -> String {
    match value {
        None => "null".to_string(),
        Some(value) => match classify(value) {
            Kind::String(text) => format!("'{text}'"),
            Kind::Number(n) => n.to_string(),
            Kind::BigInt(n) => format!("{n}n"),
            Kind::Bool(b) => b.to_string(),
            Kind::Function(_) => "fn".to_string(),
            Kind::Array(items) => format!("[{}]", items.iter().map(|item| show(*item)).collect::<Vec<_>>().join(",")),
            Kind::Object(fields) => format!(
                "{{{}}}",
                object::keys(fields).iter().map(|key| format!("{key}:{}", show(js::get(fields, key)))).collect::<Vec<_>>().join(",")
            ),
        },
    }
}
pub fn parsed(text: &str) -> String {
    match json::parse(text) {
        Ok(value) => show(value),
        Err(_) => "invalid".to_string(),
    }
}
pub fn renamed(text: &str) -> String {
    match json::parse(text) {
        Ok(Some(value)) => {
            js::set(value, "name", "new");
            show(Some(value))
        }
        _ => "invalid".to_string(),
    }
}
pub fn shown(value: &Unknown) -> String {
    show(Some(value))
}
pub fn body(response: &webapi::Response) -> Promise<Option<&'static Unknown>> {
    webapi::response::json(response)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // `classify` is the value, each variant told by `typeof`; a key that's a
  // variable is `[key]`, one written that's a name `.name`.
  expect(js).toContain('}\n  if (typeof value === "string") {');
  expect(js).toContain("}\n  if (Array.isArray(value)) {");
  expect(js).toContain('if (typeof value === "function") {');
  expect(js).toContain("Object.keys(value)");
  expect(js).toContain("show(value[key])");
  expect(js).toContain('match._0.name = "new";');
  expect(js).toContain("const match = $try(() => JSON.parse(text));");
  expect(js).toContain("return response.json();");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.parsed('{"a":[1,"x",null,true],"b":{}}'), lib.parsed("12"), lib.parsed("null"), lib.parsed("nope")])
    .toEqual(["{a:[1,'x',null,true],b:{}}", "12", "null", "invalid"]);
  expect(lib.renamed('{"name":"old","n":2}')).toBe("{name:'new',n:2}");
  expect([lib.shown(() => 1), lib.shown({ f: Math.max })]).toEqual(["fn", "{f:fn}"]);
});

// `matches!` of a kind's literal says the literal: `x === "a"` holds of no
// other kind, nor of `null`, so neither `typeof` nor `!= null` is said, and
// what's tested once is read where it's tested, as react.dev's Link tests
// `child.type.mdxName === "inlineCode"`. A temporary with a destructor
// stays in its `const`, which its drop names.
test("matches! of a kind's literal tests the value alone, where it's read", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("matches-literal");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Unknown, classify};
pub fn is_code(value: &Unknown) -> bool {
    matches!(js::get(value, "mdxName").map(classify), Some(Kind::String("inlineCode")))
}
pub fn is_text(value: Option<&Unknown>) -> bool {
    matches!(value.map(classify), Some(Kind::String("inlineCode")))
}
pub fn kind_is(value: &Unknown) -> bool {
    matches!(classify(value), Kind::String("inlineCode"))
}
pub fn listed(value: Option<&Unknown>) -> u32 {
    if let Some(value) = value
        && matches!(classify(value), Kind::Array(_))
    {
        1
    } else {
        0
    }
}
pub fn is_string(value: &Unknown) -> bool {
    matches!(js::get(value, "mdxName").map(classify), Some(Kind::String(_)))
}
pub struct Loud(pub u32);
impl Drop for Loud {
    fn drop(&mut self) {}
}
fn make(n: u32) -> Loud {
    Loud(n)
}
pub fn is_three(n: u32) -> bool {
    matches!(make(n).0, 3)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('export function is_code(value) {\n  return value.mdxName === "inlineCode";\n}');
  expect(js).toContain('export function is_text(value) {\n  return value === "inlineCode";\n}');
  expect(js).toContain('export function kind_is(value) {\n  return value === "inlineCode";\n}');
  // An array is never \`null\`: \`Array.isArray\` alone says it.
  expect(js).toContain("export function listed(value) {\n  if (Array.isArray(value)) {");
  expect(js).toContain('export function is_string(value) {\n  return typeof value.mdxName === "string";\n}');
  const lib = await import(join(dir, "lib.js"));
  const values = [{ mdxName: "inlineCode" }, { mdxName: "pre" }, { mdxName: null }, {}, { mdxName: 5 }];
  expect(values.map(lib.is_code)).toEqual([true, false, false, false, false]);
  expect(values.map((v) => lib.is_string(v))).toEqual([true, true, false, false, false]);
  expect(["inlineCode", "pre", 5, null, undefined].map((v) => lib.is_text(v))).toEqual([true, false, false, false, false]);
  expect(["inlineCode", "pre", 5, {}].map((v) => lib.kind_is(v))).toEqual([true, false, false, false]);
  expect([lib.is_three(3), lib.is_three(4)]).toEqual([true, false]);
  expect([[1], undefined, null, "a"].map((v) => lib.listed(v))).toEqual([1, 0, 0, 0]);
});

// What's never nullish is a `js::Unknown` too, as any value is TypeScript's
// `unknown`; and `js::string` is JS's `String(value)`, as `result += value`
// makes one: react.dev's console line joins its children's text so.
test("a defined value is an unknown one, and any value is a string as JS makes it", async () => {
  const withWeb = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("unknown-upcast");
  writeFileSync(join(dir, "lib.rs"), `use js::{Unknown, string, unknown};
pub fn message<'a>(text: &'a str, other: Option<&'a Unknown>, use_text: bool) -> Option<&'a Unknown> {
    if use_text { Some(unknown(text)) } else { other }
}
pub fn joined(parts: Vec<Option<&Unknown>>) -> String {
    let mut result = String::new();
    for part in parts {
        result.push_str(&string(part));
    }
    result
}
pub fn counted(n: u32) -> String {
    string(Some(unknown(&n)))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return text;");
  expect(js).toContain("String(part)");
  expect(js).toContain("return String(n);");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.message("a", 1, true), lib.message("a", 1, false), lib.message("a", undefined, false)]).toEqual(["a", 1, undefined]);
  expect(lib.joined(["a", 1, undefined, null, { b: 1 }])).toBe("a1undefinednull[object Object]");
  expect(lib.counted(7)).toBe("7");
});

// ADR 0225: a WebIDL parameter typed `any` takes a value as JS has it, and
// one the browser copies, `postMessage`'s, `pushState`'s and
// `structuredClone`'s, only a value it copies as it is: `js::StructuredClone`.
// A closure, which it can't copy, and `Some(None)`, which would arrive as
// rust-js's box, are rustc's errors; `reportError` takes anything still.
test("webapi's any parameters take any value as JS has it, and a clone only one it copies", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("any-params");
  const source = `use js::StructuredClone;
use webapi::{history, window};
pub struct Saved {
    pub page: u32,
}
// Its fields are numbers, which the browser copies.
unsafe impl StructuredClone for Saved {}
pub fn save(page: u32) {
    history::push_state(window::history(window), Saved { page }, "");
}
pub fn copy(text: &str) -> Option<&'static js::Unknown> {
    window::structured_clone(window, text)
}
pub fn copies(blob: &webapi::Blob) -> Option<&'static js::Unknown> {
    window::structured_clone(window, (vec![Some(1.5), None], "a", blob))
}
pub fn send(text: String) {
    window::post_message(window, Some(text), "*");
}
pub fn report(error: &js::JsError) {
    window::report_error(window, error);
    window::report_error(window, || ());
}
`;
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('window.history.pushState({ page }, "");');
  expect(js).toContain("return window.structuredClone(text);");
  expect(js).toContain('return window.structuredClone([[1.5, undefined], "a", blob]);');
  expect(js).toContain('window.postMessage(text, "*");');
  expect(js).toContain("window.reportError(error);");
  for (const [written, error] of [
    ["window::structured_clone(window, || ())", "the trait `js::StructuredClone` is not implemented for closure"],
    ["window::structured_clone(window, Some(None::<i32>))", "the trait `js::Defined` is not implemented for `std::option::Option<i32>`"],
    ["window::structured_clone(window, window)", "the trait `js::StructuredClone` is not implemented for `webapi::Window`"],
  ]) {
    writeFileSync(join(dir, "lib.rs"), source.replace("window::structured_clone(window, text)", written));
    const result = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "wrong.js"), ...withWeb], { cwd: dir });
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toString()).toContain(error);
  }
});

// ADR 0225: JSON is a typed value, `js::Json`, as ReScript's `JSON.t` is,
// and an object of it a `js::Dict`, ReScript's `dict`, TypeScript's
// `Record<string, T>`: a JSON `null` is `None`, a key that isn't there too,
// told apart by `dict::get`.
test("JSON is a typed value, and its objects dictionaries", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("json");
  writeFileSync(join(dir, "lib.rs"), `use js::{Dict, Json, dict};
fn show(value: Option<Json>) -> String {
    match value {
        None => "null".to_string(),
        Some(Json::String(text)) => format!("'{text}'"),
        Some(Json::Number(n)) => n.to_string(),
        Some(Json::Bool(b)) => b.to_string(),
        Some(Json::Array(items)) => format!("[{}]", items.iter().map(|item| show(*item)).collect::<Vec<_>>().join(",")),
        Some(Json::Object(fields)) => format!(
            "{{{}}}",
            dict::entries(fields).into_iter().map(|(key, value)| format!("{key}:{}", show(value))).collect::<Vec<_>>().join(",")
        ),
    }
}
pub fn parsed(text: &str) -> String {
    match Json::parse(text) {
        Ok(value) => show(value),
        Err(_) => "invalid".to_string(),
    }
}
pub fn lookup(text: &str, key: &str) -> String {
    match Json::parse(text) {
        Ok(Some(Json::Object(fields))) => match dict::get(fields, key) {
            None => "missing".to_string(),
            Some(value) => show(*value),
        },
        _ => "not an object".to_string(),
    }
}
pub fn round_trip(text: &str) -> String {
    match Json::parse(text) {
        Ok(Some(value)) => Json::stringify(&value),
        _ => "null".to_string(),
    }
}
pub fn built() -> Vec<String> {
    let numbers: &Dict<f64> = dict::from_entries(vec![("a".to_string(), 1.0)]);
    dict::set(numbers, "b", 2.0);
    dict::keys(numbers)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // A lookup is the runtime's `$dictGet`, imported with its other helpers.
  expect(js).toContain('import { $dictGet, $displayF64, $someValue, $try } from "@rust-js/runtime";');
  expect(js).toContain("Object.entries(value)");
  expect(js).toContain("const match$1 = $dictGet(match._0, key);");
  expect(js).toContain("return JSON.stringify(match._0);");
  expect(js).toContain('const numbers = Object.fromEntries([["a", 1]]);\n  numbers.b = 2;');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.parsed('{"a":[1,"x",null,true],"b":{}}'), lib.parsed("null"), lib.parsed("nope")])
    .toEqual(["{a:[1,'x',null,true],b:{}}", "null", "invalid"]);
  expect([lib.lookup('{"a":null,"b":1}', "a"), lib.lookup('{"a":null,"b":1}', "b"), lib.lookup('{"a":null}', "c"), lib.lookup('{"a":null}', "toString"), lib.lookup("[1]", "a")])
    .toEqual(["null", "1", "missing", "missing", "not an object"]);
  expect(lib.round_trip('{"a": [1, null]}')).toBe('{"a":[1,null]}');
  expect(lib.built()).toEqual(["a", "b"]);
});

// An element's constructor is WebIDL's `[HTMLConstructor]`, which only a
// custom element's class can call: `new HTMLDivElement()` in a page throws
// "Illegal constructor". So webapi binds none, and an element is made with
// `document::create_element`. Other constructors, `new Event`, it binds.
test("the webapi crate binds no element constructor, which a page can't call", () => {
  const lib = readFileSync(join(root, "webapi", "src", "lib.rs"), "utf8");
  expect(lib.match(/#\[link_name = "new HTML\w*"\]/g) ?? []).toEqual([]);
  expect(lib).toContain('#[link_name = "new Event"]');
  expect(lib).toContain('#[link_name = "new TextEncoder"]');
});

// ADR 0047: a type's methods are an object named after it.
test("methods are their type's object of functions", async () => {
  const js = await Bun.file(join(target, "methods.js")).text();
  expect(js).toContain("export const Counter = {\n  new(step) {\n    return { count: 0, step };\n  }");
  // `self` is named after its type; `&mut self` changes the object itself.
  expect(js).toContain("  tick(counter) {\n    counter.count = (counter.count + counter.step) >>> 0;\n  }");
  expect(js).toContain("      Counter.tick(next);");
  // A call is the method with its receiver first, as Rust's `Counter::tick(&mut c)`.
  expect(js).toContain("  return Counter.value(Counter.ticked(Counter.new(step), times));");
  // Each type's `new` is its own, and the object ends with a blank line.
  expect(js).toContain("};\n\nexport const Pair = {\n  new(a, b) {");
});

test("methods across modules, in a thread-local, and camelCase", async () => {
  const { fixture, compiler } = await import("./support");
  const { writeFileSync } = await import("node:fs");
  const dir = fixture("methods");
  writeFileSync(join(dir, "lib.rs"), `#[rust_js::camel_case]
const _: () = ();
use std::cell::Cell;

mod shapes;

thread_local! {
    static SIDE: Cell<u32> = Cell::new(shapes::Square::new(3).side_length());
}

pub fn area_of(side: u32) -> u32 {
    shapes::Square::new(side).area()
}

pub fn first_side() -> u32 {
    SIDE.get()
}
`);
  writeFileSync(join(dir, "shapes.rs"), `pub struct Square {
    pub side: u32,
}

impl Square {
    pub fn new(side: u32) -> Square {
        Square { side }
    }

    pub fn side_length(&self) -> u32 {
        self.side
    }

    pub fn area(&self) -> u32 {
        self.side_length() * self.side_length()
    }
}

/// Only this module uses it, so its object isn't exported.
struct Tally(u32);

impl Tally {
    fn doubled(&self) -> u32 {
        self.0 * 2
    }
}

pub fn tally_of_four() -> u32 {
    Tally(4).doubled()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await Bun.file(join(dir, "lib.js")).text();
  const shapes = await Bun.file(join(dir, "shapes.js")).text();
  expect(lib).toContain("const SIDE = Square.sideLength(Square.new(3));");
  expect(lib).toContain("  return Square.area(Square.new(side));");
  expect(shapes).toContain("export const Square = {\n  new(side) {");
  expect(shapes).toContain("  area(square) {\n    return Math.imul(Square.sideLength(square), Square.sideLength(square)) >>> 0;");
  expect(shapes).toContain("\nconst Tally = {\n  doubled(tally) {\n    return Math.imul(tally[0], 2) >>> 0;\n  },\n};\n\nexport function tallyOfFour() {");
  // Laid out on lines of its own, a lone method still maps back to its Rust.
  const { decodeMappings, lookup } = await import("./sourcemap.ts");
  const segments = decodeMappings((await Bun.file(join(dir, "shapes.js.map")).json()).mappings);
  const jsLines = shapes.split("\n");
  const rsLines = (await Bun.file(join(dir, "shapes.rs")).text()).split("\n");
  for (const [jsText, rustText] of [["Math.imul(tally[0], 2)", "self.0 * 2"], ["doubled(tally) {", "doubled(&self)"]]) {
    const line = jsLines.findIndex((l) => l.includes(jsText));
    const hit = lookup(segments, line, jsLines[line].indexOf(jsText));
    expect([jsText, hit && rsLines[hit.srcLine].slice(hit.srcCol).startsWith(rustText)]).toEqual([jsText, true]);
  }
  const module = await import(join(dir, "lib.js"));
  expect(module.areaOf(4)).toBe(16);
  expect(module.firstSide()).toBe(3);
  expect((await import(join(dir, "shapes.js"))).tallyOfFour()).toBe(8);
});

// ADR 0019: a module is imported when anything of it is used, a `const` or
// a `thread_local!` too; a module that isn't used takes no name from locals.
test("imports follow what's used: functions, consts and thread-locals", async () => {
  const { fixture, compiler } = await import("./support");
  const { writeFileSync } = await import("node:fs");
  const dir = fixture("imports");
  writeFileSync(join(dir, "lib.rs"), `mod editor;
mod helpers;
mod util;

pub fn sized() -> u32 {
    util::SIZE + util::COUNT.get() + helpers::one()
}

pub fn doubled(editor: u32) -> u32 {
    editor * 2
}
`);
  writeFileSync(join(dir, "util.rs"), `use std::cell::Cell;

pub const SIZE: u32 = 4;

thread_local! {
    pub static COUNT: Cell<u32> = Cell::new(3);
}
`);
  writeFileSync(join(dir, "helpers.rs"), "pub fn one() -> u32 {\n    1\n}\n");
  writeFileSync(join(dir, "editor.rs"), "pub fn open() -> u32 {\n    1\n}\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await Bun.file(join(dir, "lib.js")).text();
  expect(lib).toContain('import { COUNT, SIZE } from "./util.js";');
  expect(lib).not.toContain("editor.js");
  expect(lib).toContain("export function doubled(editor) {");
  const module = await import(join(dir, "lib.js"));
  expect(module.sized()).toBe(8);
  expect(module.doubled(3)).toBe(6);
});

// ADR 0020: only a mutated type is copied, and that's decided per type: a
// mutated `Pair<u32>` isn't a reason to copy a `Pair<bool>`. A generic
// function that mutates `Holder<T>` may mutate any `Holder<..>`, though.
test("copies are made for the mutated instantiations of a generic type only", async () => {
  const { fixture, compiler } = await import("./support");
  const { writeFileSync } = await import("node:fs");
  const dir = fixture("copies");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy)]
pub struct Pair<T: Copy> {
    pub a: T,
    pub b: T,
}

#[derive(Clone, Copy)]
pub struct Holder<T: Copy> {
    pub value: T,
}

pub fn bumped(p: Pair<u32>) -> (Pair<u32>, Pair<u32>) {
    let mut q = p;
    q.a += 1;
    (p, q)
}

pub fn twice(p: Pair<bool>) -> (Pair<bool>, Pair<bool>) {
    let q = p;
    (p, q)
}

pub fn set<T: Copy>(holder: &mut Holder<T>, value: T) {
    holder.value = value;
}

pub fn both(h: Holder<bool>) -> (Holder<bool>, Holder<bool>) {
    let mut g = h;
    set(&mut g, !h.value);
    (h, g)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = await Bun.file(join(dir, "lib.js")).text();
  expect(js).toContain("export function bumped(p) {\n  let q = { ...p };");
  expect(js).toContain("export function twice(p) {\n  const q = p;");
  expect(js).toContain("export function both(h) {\n  let g = { ...h };");
  const module = await import(join(dir, "lib.js"));
  expect(module.bumped({ a: 1, b: 2 })).toEqual([{ a: 1, b: 2 }, { a: 2, b: 2 }]);
  expect(module.twice({ a: true, b: false })).toEqual([{ a: true, b: false }, { a: true, b: false }]);
  expect(module.both({ value: true })).toEqual([{ value: true }, { value: false }]);
});

// ADR 0051: `Option<T>` in generic code boxes only what could look like `None`.
test("an Option of a generic T is its value, boxed only when that looks like None", async () => {
  const js = await Bun.file(join(target, "generic_options.js")).text();
  expect(js).toContain("export function pick(x, keep) {\n  if (keep) {\n    return $some(x);");
  expect(js).toContain("  return $someValue(pick(x, keep) ?? $some(fallback));");
  expect(js).toContain("  return option != null ? $some(f($someValue(option))) : undefined;");
  expect(js).toContain("  return $pop(xs);");
  // Code that isn't generic keeps `Some(x)` as `x`.
  expect(await Bun.file(join(target, "options.js")).text()).not.toContain("$some");
  // A JS caller gets plain values, and a box only for what's `None`-like.
  expect(genericOptions.pick(5, true)).toBe(5);
  expect(genericOptions.pick("a", true)).toBe("a");
  expect(genericOptions.pick(undefined, true)).toEqual({ $someNone: 0 });
  expect(genericOptions.pick(undefined, false)).toBeUndefined();
});

// ADR 0052: the crate's own `Default`, `From` and `Clone`. A clone is a copy
// only of what could be told apart, and a hand-written one is called.
test("std trait impls are direct calls, and a clone copies only what changes", async () => {
  const js = await Bun.file(join(target, "std_traits.js")).text();
  // Hand-written: called where the type is known, a dictionary for a generic.
  expect(js).toContain("const b = trackedClone_clone(a);");
  expect(js).toContain("const both = twice(copy.tracked, trackedClone());");
  expect(js).toContain("export function twice(x, TClone) {\n  return [TClone.clone(x), TClone.clone(x)];");
  // Derived: written in place, copying only the `Vec` that's pushed to.
  expect(js).toContain("  let t = { ...s, tags: s.tags.slice() };");
  expect(js).toContain("  const dot = \"Dot\";");
  // `From`, once per argument type, and `into()` is the same call.
  expect(js).toContain("const b = metersFromU32_from(3);");
  expect(js).toContain("const c = metersFromF64_from(1.5);");

  const { fixture, compiler } = await import("./support");
  const { writeFileSync } = await import("node:fs");
  const dir = fixture("clones");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone)]
pub struct Point {
    pub x: u32,
}

pub fn read_only(v: &Vec<u32>, p: &Point) -> (Vec<u32>, Point) {
    (v.clone(), p.clone())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  // Nothing changes a `Vec<u32>` or a `Point`: a clone is the value itself.
  expect(await Bun.file(join(dir, "lib.js")).text()).toContain("  return [v, p];");
});

// ADR 0053: `==` is `===` for JS primitives and `$eq` for what compares field
// by field, until a hand-written `eq` is in it: then it's called, part by part.
test("== calls a hand-written eq wherever it's inside, and generics take a dictionary", async () => {
  const js = await Bun.file(join(target, "std_traits.js")).text();
  expect(js).toContain("export function same(a, b, TPartialEq) {\n  return TPartialEq.eq(a, b);");
  expect(js).toContain("versionPartialEq_eq(r1.version, r2.version) && $eq(r1.notes, r2.notes)");
  expect(js).toContain("!(versionPartialEq_eq(r1.version, r3.version) && $eq(r1.notes, r3.notes))");
  expect(js).toContain("    left.TAG === \"Bump\"\n      ? right.TAG === \"Bump\" && versionPartialEq_eq(left._0, right._0)\n      : $eq(left, right");
  // A fieldless variant is a string: only itself is equal to it.
  expect(js).toContain("} === \"Nothing\"");
  // A `T: Eq` is given `T`'s `PartialEq`, and one that compares field by field is `$eq`.
  expect(js).toContain("count_equal(all, version(1, \"z\"), versionPartialEq())");
  expect(js).toContain("{ eq: $eq }");
  // `!=` of a hand-written `PartialEq<f64>` negates its `eq`.
  expect(js).toContain("return [metersPartialEqF64_eq(m, 2), !metersPartialEqF64_eq(m, 3");
});

// ADR 0054: a `fmt` returns the string it writes, and `{}` of a value calls it.
test("a Display impl's fmt returns the string it writes", async () => {
  const js = await Bun.file(join(target, "std_traits.js")).text();
  // One write: its string. One per way through: a `return` each.
  expect(js).toContain("function pointDisplay_fmt(point) {\n  return `(${point.x}, ${point.y})`;\n}");
  expect(js).toContain("  if (figure === \"Dot\") {\n    return \"a dot\";\n  }\n  return `a polygon of ${figure._0.length}`;");
  // More: a string built up, with nested `fmt`s and a helper that writes.
  expect(js).toContain('    f += pointDisplay_fmt(stop);');
  expect(js).toContain('    f += write_loop(route.stops.length);');
  expect(js).toContain("function write_loop(stops) {\n  return ` (a loop of ${stops})`;");
  // `{}` of one, and generics given a dictionary.
  expect(js).toContain("return `${labeled.label}: ${TDisplay.fmt(labeled.value)}`;");
  expect(js).toContain("export function shown(x, TDisplay) {\n  return `<${TDisplay.fmt(x)}>`;");
  expect(js).toContain("{ fmt: $displayF64 }");
});

// ADR 0055: an iterator of the crate's own is a JS iterator, with lazy helpers.
test("an Iterator impl is a JS iterator, lazy until something wants all of it", async () => {
  const js = await Bun.file(join(target, "std_traits.js")).text();
  expect(js).toContain("for (const x of $iterator({ n: 3 }, countdownIterator_next)) {");
  expect(js).toContain("const first = countdownIterator_next(c) ?? 0;");
  // Endless, so only lazy helpers: `drop`, `take`, `find`.
  expect(js).toContain("$iterator(fibonacci(), fibonacciIterator_next).drop(1).take(6).toArray()");
  expect(js).toContain("$iterator(fibonacci(), fibonacciIterator_next).find((x) => x > 50)");
  expect(js).toContain("$iterator({ n: 4 }, countdownIterator_next).toArray().length");
  // A generic `next` may box a `Some` that looks like `None`: unboxed as it comes out.
  expect(js).toContain("      (iterator) => repeatIterator_next(iterator, { clone: (value) => value }),\n      true,\n    )");
});

// `format_args!` is recognized whole, so its arguments are written in place,
// with `const`s only where the order Rust runs them in would change.
test("format! writes its arguments in place, in the order Rust runs them", async () => {
  const js = await Bun.file(join(target, "strings.js")).text();
  // Named arguments come after the others in Rust, but none has effects.
  expect(js).toContain("return `${name}: ${n} item${n === 1 ? \"\" : \"s\"}`;");
  // Shown out of order, with effects: in `const`s first, in Rust's order.
  expect(js).toContain(
    "  const arg = tick(c);\n  const arg$1 = tick(c);\n  const arg$2 = c.value;\n" +
      "  return `${arg$1} ${arg} ${arg} ${arg$2}`;",
  );
  // And a call in one doesn't make the call before it a `const`.
  const traits = await Bun.file(join(root, "test/snapshots/traits/traits.js")).text();
  expect(traits).toContain("label: (self) => `${circleShape_name(self)} of area ${$displayF64(circleShape_area(self))}`");
});

// ADR 0057: an `Ordering` is -1, 0 or 1; derived, the fields in turn with `||`.
test("PartialOrd and Ord compare with $cmp, a hand-written cmp, or the parts in turn", async () => {
  const js = await Bun.file(join(target, "std_traits.js")).text();
  expect(js).toContain("all.sort((a, b) => $cmp(a.major, b.major) || $cmp(a.minor, b.minor));");
  expect(js).toContain("words.sort(wordOrd_cmp);");
  expect(js).toContain("lists.sort((a, b) => $cmpItems(a, b, $cmp));");
  // Generic: dictionaries, `{ cmp }` and `{ partial_cmp }`.
  expect(js).toContain("export function in_order(a, b, TPartialOrd) {\n  return TPartialOrd.partial_cmp(a, b) <= 0;");
  // `NaN` isn't ordered: `$thenCmp` stops at an `undefined`, which `||` wouldn't.
  expect(js).toContain("$thenCmp($partialCmp(p.x, q.x), $partialCmp(p.y, q.y)) < 0");
});

// ADR 0058: format options, where Rust applies them.
test("format options pad, round and change base as Rust does", async () => {
  const js = await Bun.file(join(target, "strings.js")).text();
  // Numbers are ASCII: JS's own padding. Strings count `char`s: `$pad`.
  expect(js).toContain("`[${String(n).padStart(6)}] [${String(n).padEnd(6)}] [${$pad(name, 9, \"^\")}]");
  expect(js).toContain('$pad(name, 9, ">", "*")');
  expect(js).toContain("[0x${(n >>> 0).toString(16)}] [");
  // `{:.1}` rounds a tie to even, exactly, and `{:?}` of an `f64` keeps its `.0`.
  expect(js).toContain("return `${$toFixed(x, 0)} ${$toFixed(x, 1)} ${$toFixed(x, 3).padStart(8)} ${$debugF64(x)}`;");
});

// ADR 0059: a `HashMap` is a JS `Map`, and a `HashSet` a `Set`.
test("HashMap and HashSet are a JS Map and Set", async () => {
  const js = await Bun.file(join(target, "collections.js")).text();
  // The count idiom: the value there, or the one it would start as.
  expect(js).toContain("const current = $orInsert(counts, word, 0);\n    counts.set(word, (current + 1) >>> 0);");
  // A value that's used is the old one; one that isn't is plain `set`.
  expect(js).toContain('  m.set("a", n);\n  const old = $insert(m, "a", (n + 1) >>> 0);');
  expect(js).toContain("$orInsertWith(groups, key, () => []).push(i);");
  expect(js).toContain("const copy = new Map(Array.from(groups).map(([key, value]) => [key, value.slice()]));");
});

// ADR 0060: `{:?}` by the type, and a derived `Debug` is a function of its own.
test("a derived Debug is a function, left out unless something shows the type", async () => {
  const js = await Bun.file(join(target, "std_traits.js")).text();
  expect(js).toContain("function posDebug_fmt(pos) {\n  return `Pos { x: ${$debugF64(pos.x)}, y: ${$debugF64(pos.y)} }`;\n}");
  expect(js).toContain("  if (glyph === \"Dot\") {\n    return \"Dot\";\n  }\n  if (glyph.TAG === \"Ring\") {\n    return `Ring(${$debugF64(glyph._0)})`;");
  // Generic: `T`'s `fmt`, from a dictionary.
  expect(js).toContain("export function debugged(x, TDebug) {\n  return TDebug.fmt(x);");
  // Derived, and never shown: not in the JS at all.
  expect(js).not.toContain("neverShown");
});

// ADR 0063: a `char`'s questions are regular expressions of the Unicode
// properties Rust uses, and `parse` is a `Result` whose `Err` is Rust's message.
test("chars, parse and slices are plain JS with Rust's answers", async () => {
  const js = await Bun.file(join(target, "text.js")).text();
  expect(js).toContain("/^\\p{White_Space}$/u.test(c),\n    /^\\p{Alphabetic}$/u.test(c),");
  expect(js).toContain("const code = c.codePointAt(0);");
  expect(js).toContain("String.fromCharCode(65)");
  // `map_err(|e| e.to_string())` of a fresh `Result`: the message already is one.
  expect(js).toContain("const n = $parseInt(s, 0, 4294967295);");
  expect(js).toContain('text.split(/\\p{White_Space}+/u).filter((word) => word !== "")');
  expect(js).toContain("$slice(v, 1, 3)");
});

// A program that reads text: loops take tuples apart as JS does, and a
// value `{:?}` shows by its parts gets a name first.
test("the calculator's JS is what a person would write", async () => {
  const js = await Bun.file(join(target, "calc.js")).text();
  expect(js).toContain("for (const [i, c] of Array.from(s).entries()) {");
  expect(js).toContain('const arg$1 = first_dup("abcdbe");');
  expect(js).toContain("`Some((${arg$1[0]}, ${$debugStr(arg$1[1], \"'\")}))`");
  expect(js).toContain('$splitBy(text, (c) => !/^[\\p{Alphabetic}\\p{N}]$/u.test(c))');
  // `?` from a `&str` error to a `String` one: the same string, returned as it is.
  expect(js).toContain("_0: \"underflow\" };\n      if (result$1.TAG === \"Err\") {\n        return result$1;");
});

// ADR 0064: a number's methods are `Math`'s where JS agrees with Rust, and
// a helper where it doesn't; an operator is its impl's function.
test("numbers are Math's, operators call their impl, and vec![x; n] fills", async () => {
  const js = await Bun.file(join(target, "numbers.js")).text();
  expect(js).toContain("return Math.sqrt(vec2.x * vec2.x + vec2.y * vec2.y);");
  expect(js).toContain("vec2Add_add(");
  expect(js).toContain("`${$displayF64(Math.floor(x))} ${$displayF64(Math.ceil(x))} ${$displayF64($round(x))}");
  expect(js).toContain("$checked(b - 10, 0, 4294967295)");
  expect(js).toContain(" ${Math.max(b - 100, 0)} ");
  // Each row made again; a struct cloned, since one is changed later.
  expect(js).toContain("Array.from({ length: n }, () => new Array(n).fill(0))");
  expect(js).toContain("Array.from({ length: 3 }, () => ({ ...cell }))");
  expect(js).toContain("for (const [j$1, v] of row.entries()) {");
  // Numbers as JS writes them, and constants shown as their text.
  expect(js).toContain("$displayF64(2.220446049250313e-16)");
  expect(js).not.toContain("((tuple) =>");
});

// ADR 0086: an `i64` or a `u64` is a BigInt, wrapped once per expression,
// and a JS caller passes and gets BigInts.
test("64-bit integers are BigInts, wrapped as release Rust wraps them", async () => {
  const js = await Bun.file(join(target, "wide.js")).text();
  expect(js).toContain("BigInt.asUintN(64, (millis - 1288834974657n) << 22n) |");
  expect(js).toContain("hash = BigInt.asUintN(64, hash * 1099511628211n);");
  // What a mask keeps in range is a number without another wrap.
  expect(js).toContain("return Number(id[0] & 4095n);");
  expect(js).toContain("const worker$1 = BigInt(worker & 1023);");
  // A division by what may be zero, or `-1` of `i64::MIN`, panics as Rust's.
  expect(js).toContain("$bigDiv(7n, zero, -9223372036854775808n)");
  const id = (wide.Id as any).new(1700000000123n, 5, 42);
  expect(id).toEqual([1724551110972166186n]);
  expect((wide.Id as any).millis(id)).toBe(1700000000123n);
  expect((wide.Id as any).worker(id)).toBe(5);
  expect(wide.fnv1a("a")).toBe(0xaf63dc4c8601ec8cn);
  expect(wide.dollars(-9223372036854775808n)).toBe("-$92233720368547758.08");
});

// ADR 0067: `let ... else`, range patterns and `@`, and a `&mut` to a
// map's number, which a write puts back.
test("the store's JS: let-else, ranges, and writes through a map's value", async () => {
  const js = await Bun.file(join(target, "inventory.js")).text();
  expect(js).toContain("let have = store.stock.get(e.item);\n      if (have == null) {\n        return { TAG: \"Err\", _0: `unknown item ${e.item}` };\n      }");
  expect(js).toContain("have = (have - e.qty) >>> 0;\n      store.stock.set(e.item, have);");
  expect(js).toContain("}\n    if (x >= 1 && x < 10) {\n      return `few ${x}`;\n    }\n    if ((x >= 10 && x <= 99) || x >= 200) {");
  // A bound at the type's own end always holds, so it isn't tested.
  expect(js).toContain("if (n <= -1) {");
  expect(js).toContain("? `revenue ${$toFixed(store.revenue, 2)} under ${$toFixed(revenue[0], 2)}`\n    : undefined;");
  // A block's statements go before the `const` of its value.
  expect(js).toContain("  const c = counter;\n  const bump = (n) => {");
});

// ADR 0070: a std function taken as a value is an arrow, and an array's
// constant index below its length is read directly.
test("the report's JS: function values, case mapping, and plain array reads", async () => {
  const js = await Bun.file(join(target, "report.js")).text();
  expect(js).toContain('const parts = line.split(",").map((s) => $trim(s));');
  expect(js).toContain(".flatMap((c) => Array.from(c.toUpperCase()))");
  expect(js).toContain('.map((c) => /^\\p{White_Space}$/u.test(c))');
  expect(js).toContain(".map((n) => Math.sqrt(n))");
  expect(js).toContain("HEADERS[0]");
  expect(js).toContain('$debugParseError($unwrapErr($parseInt("", 0, 255)), "ParseIntError")');
});

// ADR 0071: an iterator stepped through is a `$iter`, which knows where it is.
test("the lexer's JS steps through its source with $next and $peek", async () => {
  const js = await Bun.file(join(target, "lexer.js")).text();
  expect(js).toContain("return { chars: $iter(Array.from(src)), line: 1 };");
  expect(js).toContain("const value = $peek(lexer.chars);");
  expect(js).toContain('} else if (c === "/" && $nextIf(lexer.chars, (item) => item === "/") != null) {');
  expect(js).toContain("let it = $iter(v);");
  expect(js).toContain("const rest = $rest(it);");
  expect(js).toContain("return match.toUpperCase() + $restStr(chars);");
});

// ADR 0074: a `&mut` to a string or a number is a box the caller copies
// back, and a recursive type's clone is a function that calls itself.
test("values' JS: boxes for &mut to primitives, and recursive clones", async () => {
  const js = await Bun.file(join(target, "values.js")).text();
  expect(js).toContain("out.value += \"[\\n\";");
  expect(js).toContain("const out$1 = { value: p };\n  Value.pretty(v, 0, out$1);\n  p = out$1.value;");
  expect(js).toContain("count.value = (count.value + by) >>> 0;");
  expect(js).toContain("const count$3 = { value: stats.hits };");
  expect(js).toContain("stats.hits = count$3.value;");
  expect(js).toContain("const cloneValue = (value) =>");
});

// ADR 0076: a struct's `..base` is worked out after its fields, and a few
// std methods are Rust's own steps.
test("versions' JS: struct update in Rust's order, scan, drain and total_cmp", async () => {
  const js = await Bun.file(join(target, "versions.js")).text();
  expect(js).toContain("{ major: 0, minor: 9, patch: 0 }");
  expect(js).toContain("const patch = tick(order, 1);\n  const major = tick(order, 2);\n  const base = made(order);");
  expect(js).toContain("const value = $nextSome(it);");
  expect(js).toContain("fs.sort((a, b) => $totalCmp(a, b));");
  expect(js).toContain("const tail = $splitOff(v, 5);");
  expect(js).toContain("$lowerExp(1234.5)");
});

// ADR 0061: `impl Iterator` is the type it hides, and a generic iterator is
// whatever JS iterable it's given.
test("generic iterators take arrays and JS iterators alike", async () => {
  const js = await Bun.file(join(target, "std_traits.js")).text();
  expect(js).toContain("export function evens_below(n) {\n  return $range(0, n).filter((x) => x % 2 === 0);");
  expect(js).toContain("export function middle(items, k) {\n  return Iterator.from(items).drop(1).take(k).toArray();");
  expect(js).toContain("for (const x of items) {");
  // One of the crate's own, given where a generic one goes.
  expect(js).toContain("total($iterator({ n }, countdownIterator_next))");
});

// `js::import!("./app.css");` is `import "./app.css";` in its module's JS, at
// the root and in a module (ADR 0110), where `#![rust_js::import]` was: stable
// Rust has no inner attribute of a tool. Its `const _` writes nothing.
test("js::import! imports a module where it's written, and writes nothing of its own", () => {
  const dir = fixture("js-import");
  writeFileSync(join(dir, "lib.rs"), 'js::import!("./app.css");\npub mod panel {\n    js::import!("./panel.css");\n    pub fn f() -> u32 {\n        1\n    }\n}\n#[rust_js::import = "./direct.css"]\nconst _: () = ();\n');
  buildWebapi();
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const top = readFileSync(join(dir, "lib.js"), "utf8"), panel = readFileSync(join(dir, "panel.js"), "utf8");
  expect([top.includes('import "./app.css";'), top.includes('import "./direct.css";'), panel.includes('import "./panel.css";')]).toEqual([true, true, true]);
  expect(top).not.toContain("const _");
  expect(panel).not.toContain("app.css");
});

// `js::directive!("use client");` is the module's directive, its first
// statement, and `js::export_default!(page);` its default export, of a
// function that stays named for the crate's other modules (ADR 0192): what a
// Next.js route is, `app/page.jsx`.
test("js::directive! and js::export_default! make a module a Next.js route", async () => {
  const dir = fixture("js-route");
  writeFileSync(join(dir, "lib.rs"), 'js::directive!("use client");\npub fn page() -> u32 {\n    about::about() + 1\n}\njs::export_default!(page);\npub mod about {\n    pub fn about() -> u32 {\n        2\n    }\n    js::export_default!(about);\n}\npub mod generic {\n    pub fn first<T>(items: Vec<T>) -> Option<T> {\n        items.into_iter().next()\n    }\n    js::export_default!(first);\n}\n');
  buildWebapi();
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const top = readFileSync(join(dir, "lib.js"), "utf8"), about = readFileSync(join(dir, "about.js"), "utf8");
  const statements = (js: string) => js.split("\n").filter((line) => line && !line.startsWith("//"));
  expect(statements(top)[0]).toBe('"use client";');
  expect(statements(about)[0]).not.toBe('"use client";');
  expect(top).toContain("export default page;");
  expect(top).not.toContain("const _");
  const [lib, sub] = [await import(join(dir, "lib.js")), await import(join(dir, "about.js"))];
  expect([lib.default(), lib.page(), sub.default()]).toEqual([3, 3, 2]);
  // A generic function's too, which Rust can't name as a value unless it's
  // given its types.
  expect((await import(join(dir, "generic.js"))).default([7, 8])).toBe(7);
});

// A module's `pub use` of another module's function is a JS re-export,
// `export { helper } from "./inner.js"`, as react.dev's Challenges/index
// re-exports `Challenges` (ADR 0240).
// A JS module's namespace, `#*.Root`, whose bindings a Rust module holds,
// is imported by that module's name, as react.dev's BrandMenu has
// `import * as ContextMenu` and `<ContextMenu.Root>`.
// A closure that only spawns an async block is an async arrow, as
// react.dev's BrandMenu writes `onSelect={async () => { await
// navigator.clipboard.writeText(..) }}`: it starts the same, and gives
// back a promise where nothing reads its `()` (ADR 0257).
test("a closure that only spawns an async block is an async arrow", async () => {
  const dir = fixture("spawning-closure");
  writeFileSync(join(dir, "copy.js"), "export const copied = [];\nexport async function copy(text) { copied.push(text); }\n");
  writeFileSync(join(dir, "lib.rs"), `unsafe extern "Rust" {
    #[link_name = "./copy.js#copy"]
    safe fn copy(text: &str) -> js::Promise<()>;
}

pub fn handler() -> Box<dyn Fn()> {
    Box::new(|| {
        js::spawn(Box::new(async {
            copy("#58C4DC").await;
        }))
    })
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return async () => {\n    await copy("#58C4DC");\n  };');
  const lib = await import(join(dir, "lib.js"));
  const { copied } = await import(join(dir, "copy.js"));
  lib.handler()();
  expect(copied).toEqual(["#58C4DC"]);
});

test("a namespace import is named as the module of its bindings", async () => {
  const dir = fixture("namespace-module");
  writeFileSync(join(dir, "menu.js"), "export function Root() { return 1; }\nexport function Item() { return 2; }\n");
  writeFileSync(join(dir, "lib.rs"), `#[allow(non_snake_case)]
mod ContextMenu {
    unsafe extern "Rust" {
        #[link_name = "./menu.js#*.Root"]
        pub safe fn Root() -> u32;
        #[link_name = "./menu.js#*.Item"]
        pub safe fn Item() -> u32;
    }
}

pub fn both() -> u32 {
    ContextMenu::Root() + ContextMenu::Item()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('import * as ContextMenu from "./menu.js";');
  expect(js).toContain("ContextMenu.Root() + ContextMenu.Item()");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.both()).toBe(3);
});

test("a pub use of another module's function is re-exported from it", async () => {
  const dir = fixture("reexports");
  writeFileSync(join(dir, "lib.rs"), "mod inner;\npub use inner::helper;\npub use inner::other as renamed;\nuse inner::other;\n\npub fn own() -> u32 {\n    other() + 1\n}\n");
  writeFileSync(join(dir, "inner.rs"), "pub fn helper() -> u32 {\n    1\n}\n\npub fn other() -> u32 {\n    2\n}\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('export { helper, other as renamed } from "./inner.js";');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.helper(), lib.renamed(), lib.own()]).toEqual([1, 2, 3]);
});

// A module of re-exports only is a file of them, as react.dev's
// Sidebar/index is `export {SidebarLink} from './SidebarLink'`.
test("a module of only pub uses is a file of re-exports", async () => {
  const dir = fixture("reexports-only");
  writeFileSync(join(dir, "lib.rs"), "mod index;\nmod inner;\n\npub fn own() -> u32 {\n    index::helper() + 1\n}\n");
  writeFileSync(join(dir, "index.rs"), "pub use super::inner::helper;\n");
  writeFileSync(join(dir, "inner.rs"), "pub fn helper() -> u32 {\n    1\n}\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect(readFileSync(join(dir, "index.js"), "utf8")).toContain('export { helper } from "./inner.js";');
  const index = await import(join(dir, "index.js"));
  expect(index.helper()).toBe(1);
});

// ADR 0267: what a module runs when it's loaded, `js::on_load!`, is its
// JS's own statements, as react.dev's Page prefetches CodeBlock with a bare
// `import()`: rustc checks them as a function's body nothing calls.
test("js::on_load! is what the module runs when it's loaded", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("on-load");
  writeFileSync(join(dir, "lib.rs"), `unsafe extern "Rust" {
    #[link_name = "globalThis.loaded"]
    safe fn loaded(what: &str);
}

fn which() -> usize {
    1
}

js::on_load! {
    loaded("module");
    loaded(["first", "second"][which()]);
}

pub fn ready() -> u32 {
    1
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('\nglobalThis.loaded("module");\n');
  expect(js).toContain('import { $index } from "@rust-js/runtime";');
  expect(js).not.toContain("on_load");
  expect(js).not.toContain("const _");
  const seen: string[] = [];
  (globalThis as any).loaded = (what: string) => seen.push(what);
  try {
    const { ready } = await import(join(dir, "lib.js"));
    expect([seen, ready()]).toEqual([["module", "second"], 1]);
  } finally {
    delete (globalThis as any).loaded;
  }
});

// ADR 0266: text is falsy in JS only where it's empty, so text an option
// keeps where it isn't, `filter(|s| !s.is_empty())`, then another's or a
// default, is JS's `||`: `meta.title || route?.title || ""`, as react.dev's
// Page writes it. An array, which is truthy empty, isn't.
test("text kept where it isn't empty, or another, is ||", async () => {
  const dir = fixture("text-or");
  writeFileSync(join(dir, "lib.rs"), `pub struct Meta<'a> {
    pub title: Option<&'a str>,
}

pub struct Route {
    pub title: String,
}

pub fn title<'a>(meta: &Meta<'a>, route: Option<&'a Route>) -> &'a str {
    (meta.title.filter(|title| !title.is_empty()))
        .or(route.map(|route| route.title.as_str()).filter(|title| !title.is_empty()))
        .unwrap_or("")
}

pub fn items(list: Option<Vec<u32>>) -> Vec<u32> {
    list.filter(|list| !list.is_empty()).unwrap_or(vec![1])
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return meta.title || route?.title || "";');
  expect(js).not.toContain("list ||");
  const { title, items } = await import(join(dir, "lib.js"));
  expect([title({ title: "" }, { title: "R" }), title({}, undefined), title({ title: "M" }, { title: "R" }), title({ title: "" }, { title: "" })]).toEqual(["R", "", "M", ""]);
  expect([items([]), items([2]), items(undefined)]).toEqual([[1], [2], [1]]);
});

// ADR 0264: a fieldless variant is its name (ADR 0013), so a `match` giving
// each variant its own name is what's matched, and a function that gives
// back what it's given, its argument: `/images/og-${section}.png`, as
// react.dev's Page writes it. One giving another name is a conditional.
test("an enum's own names are the enum", async () => {
  const dir = fixture("enum-names");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy, PartialEq)]
pub enum Section {
    #[cfg_attr(rust_js, rust_js::name = "learn")]
    Learn,
    #[cfg_attr(rust_js, rust_js::name = "blog")]
    Blog,
}

impl Section {
    pub fn as_str(self) -> &'static str {
        match self {
            Section::Learn => "learn",
            Section::Blog => "blog",
        }
    }

    pub fn heading(self) -> &'static str {
        match self {
            Section::Learn => "learn",
            Section::Blog => "news",
        }
    }
}

pub fn image(section: Section) -> String {
    format!("/images/og-{}.png", section.as_str())
}

pub fn title(section: Section) -> String {
    format!("{} page", section.heading())
}

// Mapped by it, an option's is the option, as react.dev's Page keys its
// SidebarNav, \`key={section}\`.
pub fn key(section: Option<Section>) -> Option<&'static str> {
    section.map(Section::as_str)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("as_str(section) {\n    return section;\n  }");
  expect(js).toContain("return `/images/og-${section}.png`;");
  expect(js).toContain("Section.heading(section)");
  expect(js).toContain("export function key(section) {\n  return section;\n}");
  const { image, title } = await import(join(dir, "lib.js"));
  expect([image("learn"), image("blog"), title("learn"), title("blog")]).toEqual(["/images/og-learn.png", "/images/og-blog.png", "learn page", "news page"]);
});

// ADR 0238: a `String` kept for good, `.leak()`, is the string itself, as
// react.dev's Page gives Seo the image it makes for a component's
// `'static` props: JS frees nothing itself.
test("a leaked String is the string", async () => {
  const dir = fixture("string-leak");
  writeFileSync(join(dir, "lib.rs"), `pub fn named(n: u32) -> &'static str {
    format!("item-{n}").leak()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return `item-${n}`;");
  const { named } = await import(join(dir, "lib.js"));
  expect(named(3)).toBe("item-3");
});

// A struct taken apart through a shared reference is JS's destructuring,
// `const { errorMessage, errorCode } = useErrorDecoderParams();`, as
// react.dev's ErrorDecoder has it: what's borrowed can't change while it
// is (ADR 0244). A `Cell`, which can, is the one JS object either way.
test("a struct taken apart through a reference is destructured", async () => {
  const dir = fixture("ref-destructure");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

pub struct Params {
    pub message: Option<String>,
    pub code: Option<String>,
}

pub struct Counter {
    pub count: Cell<u32>,
    pub label: String,
}

fn params() -> &'static Params {
    Box::leak(Box::new(Params { message: Some("m".to_string()), code: None }))
}

fn first(counter: &Counter) -> &Counter {
    counter
}

pub fn described() -> String {
    let Params { message, code } = params();
    format!("{message:?} {code:?}")
}

pub fn counted() -> u32 {
    let counter = Counter { count: Cell::new(1), label: "ab".to_string() };
    let Counter { count, label } = first(&counter);
    count.set(count.get() + label.len() as u32);
    counter.count.get()
}

// A closure taking a pair apart, inlined where it's called, is its part.
pub fn found(people: Vec<(String, u32)>) -> Option<usize> {
    people.binary_search_by_key(&57, |&(_, age)| age).ok()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const { message, code } = params();");
  const lib = await import(join(dir, "lib.js"));
  expect(js).toContain("$cmp(item[1], 57)");
  expect([lib.described(), lib.counted(), lib.found([["a", 50], ["b", 57]])]).toEqual(['Some("m") None', 3, 1]);
});

// An `if` whose branch leaves has no `else`: what follows it runs only when
// the branch doesn't, as JS writes it and react.dev's Link has it, `if (..)
// { return cloneElement(..); } return child;` (ADR 0237). A branch that
// doesn't leave keeps its `else`.
test("an if whose branch returns has no else", async () => {
  const dir = fixture("no-else-return");
  writeFileSync(join(dir, "lib.rs"), `pub fn classify(n: i32) -> &'static str {
    if n < 0 {
        println!("neg");
        "neg"
    } else if n == 0 {
        println!("zero");
        "zero"
    } else {
        println!("pos");
        "pos"
    }
}
// A branch that leaves by an inner \`if\` both of whose branches do.
pub fn nested(a: bool, b: bool) -> u32 {
    if a {
        if b {
            println!("ab");
            1
        } else {
            println!("a");
            2
        }
    } else {
        println!("none");
        3
    }
}
pub fn counted(n: i32) -> i32 {
    let mut total = 0;
    if n > 0 {
        total += n;
    } else {
        total -= n;
    }
    total
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  if (n < 0) {\n    console.log("neg");\n    return "neg";\n  }\n  if (n === 0) {\n    console.log("zero");\n    return "zero";\n  }\n  console.log("pos");\n  return "pos";\n}');
  expect(js).toContain("} else {\n    total = (total - n) | 0;");
  expect(js).toContain('    console.log("a");\n    return 2;\n  }\n  console.log("none");\n  return 3;\n}');
  const lib = await import(join(dir, "lib.js"));
  const log = console.log;
  console.log = () => {};
  try {
    expect([lib.classify(-1), lib.classify(0), lib.classify(3), lib.counted(2), lib.counted(-2)]).toEqual(["neg", "zero", "pos", 2, 2]);
    expect([lib.nested(true, true), lib.nested(true, false), lib.nested(false, true)]).toEqual([1, 2, 3]);
  } finally {
    console.log = log;
  }
});

// A `match` of a fieldless enum whose arms each give one table's field named
// as their variant is the table read by the value, as react.dev's
// ExpandableCallout reads `variantMap[type]`; an arm of another field keeps
// the conditional. The corpus's `match_index` runs it beside native Rust.
test("a match giving a table's field named as each variant reads the table by it", () => {
  const dir = fixture("match-index");
  writeFileSync(join(dir, "lib.rs"), readFileSync(join(root, "test/corpus/match_index.rs"), "utf8").replace("fn main()", "pub fn main()"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("function variant(kind) {\n  return VARIANTS[kind];\n}");
  expect(js).toContain("function picked(i) {\n  return VARIANTS[pick(i)];\n}");
  expect(js).toContain("const chosen = VARIANTS[kind];\n  return chosen.title;");
  expect(js).toContain('if (kind === "note") {\n    tmp = VARIANTS.pitfall;');
  expect(js).toContain('tmp = OTHER.pitfall;');
});

// `let Some(href) = href.filter(|href| !href.is_empty()) else { .. }` tests
// what the filter does and names `href`, with no `const` of the `Option`:
// of text, which is falsy only empty, `if (!href)`, as react.dev's Link
// has it. An array, which is truthy empty, keeps its `length` test, and
// what's bound of a variable that changes is a copy.
test("a let-else of an Option's filter tests the filter, and binds what it kept", async () => {
  const dir = fixture("let-else-filter");
  writeFileSync(join(dir, "lib.rs"), `pub fn link(href: Option<&str>) -> String {
    let Some(href) = href.filter(|href| !href.is_empty()) else {
        return "none".to_string();
    };
    href.to_uppercase()
}
pub fn named(name: Option<String>) -> String {
    if let Some(n) = name.filter(|n| !n.is_empty()) { n.to_uppercase() } else { "anon".to_string() }
}
pub fn listed(items: Option<Vec<u32>>) -> u32 {
    let Some(items) = items.filter(|items| !items.is_empty()) else {
        return 0;
    };
    items[0]
}
pub fn later(mut href: Option<&str>) -> String {
    let Some(h) = href.filter(|h| !h.is_empty()) else {
        return "none".to_string();
    };
    href = Some("changed");
    format!("{h} {}", href.unwrap_or_default())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('export function link(href) {\n  if (!href) {\n    return "none";\n  }\n  return href.toUpperCase();\n}');
  expect(js).toContain('if (name) {\n    return name.toUpperCase();');
  expect(js).toContain("if (!(items != null && items.length !== 0)) {\n    return 0;\n  }\n  return $index(items, 0);");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.link("/a"), lib.link(""), lib.link(undefined)]).toEqual(["/A", "none", "none"]);
  expect([lib.named("ann"), lib.named(""), lib.named(undefined)]).toEqual(["ANN", "anon", "anon"]);
  expect([lib.listed([7]), lib.listed([]), lib.listed(undefined)]).toEqual([7, 0, 0]);
  expect([lib.later("/a"), lib.later("")]).toEqual(["/a changed", "none"]);
});

// `Some(Direction::Up)` of an `Option` of a unit variant is `d === "Up"`:
// `undefined === "Up"` is false too, so it's said without `d != null`, as
// a constant is. Where the variant is an object's, `d.TAG`, it's needed.
test("a Some of a unit variant is tested as the variant, without a null test", async () => {
  const dir = fixture("option-unit-variant");
  writeFileSync(join(dir, "lib.rs"), 'pub enum Direction {\n    Up,\n    Down,\n}\npub enum Shape {\n    Dot,\n    Line(u32),\n}\npub fn turn(d: Option<Direction>) -> u32 {\n    match d {\n        Some(Direction::Up) => 1,\n        Some(Direction::Down) => 2,\n        None => 0,\n    }\n}\npub fn size(s: Option<Shape>) -> u32 {\n    match s {\n        Some(Shape::Dot) => 1,\n        Some(Shape::Line(n)) => n,\n        None => 0,\n    }\n}\n');
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect([js.includes('if (d === "Up")'), js.includes("d != null"), js.includes('if (s === "Dot")'), js.includes("s != null && s.TAG")]).toEqual([true, false, true, true]);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.turn("Up"), lib.turn("Down"), lib.turn(undefined), lib.size("Dot"), lib.size({ TAG: "Line", _0: 7 }), lib.size(undefined)]).toEqual([1, 2, 0, 1, 7, 0]);
});

// A value with a destructor moved before anything in its scope can leave,
// `hold(l)`'s into what it returns, is never the scope's to drop: no flag,
// no `try` (ADR 0197), as react.dev's ExternalLink moves its children. One
// moved after what may panic keeps them.
test("a value moved before anything can leave needs no drop of its scope", async () => {
  const dir = fixture("moved-first");
  writeFileSync(join(dir, "lib.rs"), 'pub struct Loud(pub u32);\nimpl Drop for Loud {\n    fn drop(&mut self) {}\n}\npub struct Holder {\n    pub l: Loud,\n}\npub fn hold(l: Loud) -> Holder {\n    Holder { l }\n}\npub fn checked(l: Loud, n: u32) -> Holder {\n    assert!(n > 0);\n    Holder { l }\n}\n');
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function hold(l) {\n  return { l };\n}");
  expect(js).toContain("l$live = false;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.hold([1]).l[0], lib.checked([2], 1).l[0]]).toEqual([1, 2]);
});

// The crates a program depends on, js, webapi and react, are stable Rust too
// (ADR 0112): rust-js compiles them, `--rustc`, with its tool registered, so
// they need no `register_tool`, nor any feature, nor `RUSTC_BOOTSTRAP`.
test("the binding crates are stable Rust, which rust-js compiles", () => {
  for (const root of ["builtins/src/lib.rs", "webapi/src/lib.rs", "react/src/lib.rs"]) {
    const source = readFileSync(join(import.meta.dir, "..", root), "utf8");
    expect([root, /^#!\[(feature|register_tool)\b/m.test(source)]).toEqual([root, false]);
  }
  const { RUSTC_BOOTSTRAP: _, ...env } = process.env;
  const dir = fixture("stable-bindings");
  const p = Bun.spawnSync(["react/build.sh", "-o", join(dir, "libreact.rmeta")], { cwd: join(import.meta.dir, ".."), env: { ...env, RUST_JS_COMPILER: compiler } });
  expect(p.stderr.toString()).not.toContain("error");
  expect(p.exitCode).toBe(0);
  for (const name of ["libjs.rmeta", "libwebapi.rmeta", "libreact.rmeta"]) expect(Bun.file(join(dir, name)).size).toBeGreaterThan(0);
});

// A program is plain stable Rust too, for a user's own `cargo check` and
// editor (ADR 0113): a plain rustc compiles the crates it uses, which leave
// out rust-js's attributes, `cfg_attr(rust_js, ..)`, and the vite-react app,
// to which `jsx!` is react's placeholder, and `js::import!` nothing.
test("a plain stable rustc compiles the template's app and the crates it uses", () => {
  const { RUSTC_BOOTSTRAP: _, ...env } = process.env;
  const dir = fixture("plain-rustc");
  const repo = join(import.meta.dir, "..");
  const rustc = (source: string, name: string, externs: string[]) => {
    const p = Bun.spawnSync([
      "rustc", "--edition=2024", "--crate-type=lib", "--emit=metadata", "--target=wasm32-unknown-unknown", `--crate-name=${name}`,
      join(repo, source), "-o", join(dir, `lib${name}.rmeta`), "-L", dir,
      ...externs.flatMap((e) => ["--extern", `${e}=${join(dir, `lib${e}.rmeta`)}`]),
    ], { cwd: repo, env });
    return [name, p.exitCode, p.stderr.toString().split("\n").filter((line) => line.startsWith("error")).slice(0, 3)];
  };
  expect(rustc("builtins/src/lib.rs", "js", [])).toEqual(["js", 0, []]);
  expect(rustc("webapi/src/lib.rs", "webapi", ["js"])).toEqual(["webapi", 0, []]);
  expect(rustc("react/src/lib.rs", "react", ["js", "webapi"])).toEqual(["react", 0, []]);
  expect(rustc("examples/vite-react/src/App.rs", "app", ["js", "webapi", "react"])).toEqual(["app", 0, []]);
});

// An editor's rust-analyzer checks the app as Cargo does, with a plain
// stable rustc: the example's Cargo.toml has App.rs, and the crates it uses.
test("a plain `cargo check` checks the vite-react example", () => {
  const dir = join(fixture("cargo-check-example"), "target");
  const example = join(root, "examples", "vite-react");
  const p = runSync(["cargo", "check", "--offline", "--quiet", "--manifest-path", join(example, "Cargo.toml"), "--target-dir", dir], example, 300_000, { RUSTC_BOOTSTRAP: undefined });
  // No error, and no warning of the crates it uses: the app's are of what's
  // used only in JSX, which react's placeholder `jsx!` doesn't look inside.
  const warnings = p.stderr.split("\n").filter((line) => /^(warning|error)\b/.test(line) && !/\(lib\) generated \d+ warnings?/.test(line));
  expect(warnings.filter((line) => !/^warning: (unused variable: |static `\w+` is never used)/.test(line))).toEqual([]);
  expect(p.stderr).not.toMatch(/`rust-js-\w+` \(lib\) generated/);
  expect(p.code).toBe(0);
}, 300_000);

// One whose arms bind what their subject holds, named where it is, is a
// conditional too (ADR 0209), as react.dev's DocsFooter picks a link or a
// `<div />`. What needs a `const` of its own is statements, as before.
test("a two-arm match whose arms bind places is a conditional expression", async () => {
  const dir = fixture("match-conditional-binds");
  writeFileSync(join(dir, "lib.rs"), `pub struct Item {
    pub title: String,
    pub path: Option<String>,
}
pub fn path(item: Option<&Item>) -> &str {
    let label = match item {
        Some(Item { path: Some(path), .. }) => path.as_str(),
        _ => "none",
    };
    label
}
// The second arm's too.
pub fn either(r: Result<u32, u32>) -> u32 {
    let v = match r {
        Ok(n) => n,
        Err(n) => n + 1,
    };
    v
}
// A guard reads what its arm binds.
pub fn titled(item: Option<&Item>) -> &str {
    let label = match item {
        Some(item) if item.path.is_some() => item.title.as_str(),
        _ => "untitled",
    };
    label
}
// What's owned has a \`const\` of its own, dropped where Rust drops it.
pub struct Loud(pub u32);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
pub fn owned(some: bool) -> u32 {
    let l = if some { Some(Loud(7)) } else { None };
    let n = match l {
        Some(l) => l.0,
        None => 0,
    };
    println!("after");
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('const label = item != null && item.path != null ? item.path : "none";');
  expect(js).toContain('const v = r.TAG === "Ok" ? r._0 : (r._0 + 1) >>> 0;');
  expect(js).toContain('const label = item != null && item.path != null ? item.title : "untitled";');
  expect(js).not.toContain("let tmp");
  const lib = await import(join(dir, "lib.js"));
  const item = { title: "T", path: "/p" };
  expect([lib.path(item), lib.path({ title: "T" }), lib.path(undefined)]).toEqual(["/p", "none", "none"]);
  expect([lib.either({ TAG: "Ok", _0: 1 }), lib.either({ TAG: "Err", _0: 1 })]).toEqual([1, 2]);
  expect([lib.titled(item), lib.titled({ title: "T" })]).toEqual(["T", "untitled"]);
  const logs: string[] = [];
  const log = console.log;
  console.log = (line: string) => logs.push(line);
  try {
    expect([lib.owned(true), lib.owned(false)]).toEqual([7, 0]);
  } finally {
    console.log = log;
  }
  expect(logs).toEqual(["drop 7", "after", "after"]);
});

// A binding's last parameter, a slice, can be JS's rest arguments,
// `#[rust_js::variadic]`: a slice written out is the arguments, as
// react.dev calls `cn("a", className)`, and another one spread (ADR 0221).
test("a variadic binding takes a slice as its rest arguments", async () => {
  const dir = fixture("variadic");
  writeFileSync(join(dir, "lib.rs"), `unsafe extern "Rust" {
    #[link_name = "Math.max"]
    #[rust_js::variadic]
    safe fn max(values: &[f64]) -> f64;
}
pub fn largest(a: f64, b: f64) -> f64 {
    max(&[a, b, 1.0])
}
pub fn largest_of(values: Vec<f64>) -> f64 {
    max(&values)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return Math.max(a, b, 1);");
  expect(js).toContain("return Math.max(...values);");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.largest(-3, -2), lib.largest(4, 2), lib.largest_of([2, 9, 4])]).toEqual([1, 4, 9]);
});

// A unit struct named `#[rust_js::name]` is that string, as a fieldless
// variant is (ADR 0013): `webapi`'s event names, `Click`, are types whose
// value is `"click"` (ADR 0223). One without a name holds nothing.
test("a named unit struct is its name", async () => {
  const dir = fixture("named-unit-struct");
  writeFileSync(join(dir, "lib.rs"), `#[rust_js::name = "click"]
pub struct Click;
pub struct Marker;
const CLICK: Click = Click;
fn pass<E>(event: E) -> E {
    event
}
pub fn click() -> Click {
    Click
}
pub fn passed() -> Click {
    pass(Click)
}
pub fn constant() -> Click {
    CLICK
}
pub fn marker() -> Marker {
    Marker
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return "click";');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.click(), lib.passed(), lib.constant(), lib.marker()]).toEqual(["click", "click", "click", undefined]);
});

// A value shown is only read, through the reference `format_args!` takes,
// which nothing can change before it's shown: a `Copy` one changed elsewhere
// is shown in place, not copied first, as a read of it by value is (ADR 0020).
test("a formatted value is read in place, not copied", async () => {
  const dir = fixture("format-in-place");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy, Debug)]
pub struct P {
    pub x: i32,
}
pub fn show(mut p: P) -> String {
    let q = p;
    p.x += 1;
    let all = [q, p];
    format!("{q:?} {p:?} {all:?}")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const q = { ...p };");
  expect(js).toContain("${pDebug_fmt(q)} ${pDebug_fmt(p)} [${all.map((item) => pDebug_fmt(item))");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.show({ x: 1 })).toBe("P { x: 1 } P { x: 2 } [P { x: 1 }, P { x: 2 }]");
});

// A two-arm `match` as a value, its arms plain and binding nothing, is a
// conditional, as a person writes it (ADR 0209): its subject in place
// where the test reads it once, else in a `const` of its own.
test("a two-arm match that's a value is a conditional expression", async () => {
  const dir = fixture("match-conditional");
  writeFileSync(join(dir, "lib.rs"), `pub enum Kind {
    Primary,
    Secondary,
}
pub fn button_class(kind: Option<Kind>) -> &'static str {
    let class = match kind.unwrap_or(Kind::Primary) {
        Kind::Primary => "bg-link",
        Kind::Secondary => "text-primary",
    };
    class
}
pub fn low(x: u32) -> &'static str {
    let size = match x % 3 {
        0 | 1 => "low",
        _ => "high",
    };
    size
}
// A plain guard is the arm's test too.
pub fn guarded(n: u32, flag: bool) -> &'static str {
    let label = match n {
        0 if flag => "flagged zero",
        _ => "other",
    };
    label
}
// A &mut to a number, a cell, is tested by its value.
pub fn zero(n: &mut i32) -> &'static str {
    let label = match n {
        &mut 0 => "zero",
        _ => "other",
    };
    label
}
// A bool tested against true is the bool, as JS has it.
pub fn emptiness(xs: Vec<u32>) -> &'static str {
    let label = match xs.is_empty() {
        true => "empty",
        false => "some",
    };
    label
}
pub fn toggle(flag: bool) -> &'static str {
    let label = match flag {
        false => "off",
        true => "on",
    };
    label
}
// A first arm that takes everything is the value, its guard the test.
#[allow(unreachable_patterns)]
pub fn every(n: u32, flag: bool) -> &'static str {
    let all = match n {
        _ => "all",
        1 => "one",
    };
    let guarded = match n {
        _ if flag => "flag",
        _ => "no flag",
    };
    if all == "all" { guarded } else { "never" }
}
// A subject with a destructor is dropped where Rust drops it.
pub struct Loud(pub u32);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
pub fn dropped() -> &'static str {
    let label = match Loud(1) {
        Loud(1) => "one",
        _ => "other",
    };
    println!("after");
    label
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('(kind ?? "Primary") === "Primary" ? "bg-link" : "text-primary"');
  expect(js).toContain('match === 0 || match === 1 ? "low" : "high"');
  expect(js).toContain('const label = n === 0 && flag ? "flagged zero" : "other";');
  expect(js).toContain('const label = n.value === 0 ? "zero" : "other";');
  expect(js).toContain('const label = xs.length === 0 ? "empty" : "some";');
  expect(js).toContain('const label = !flag ? "off" : "on";');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.button_class(undefined), lib.button_class("Secondary"), lib.low(4), lib.low(5)]).toEqual(["bg-link", "text-primary", "low", "high"]);
  expect([lib.guarded(0, true), lib.guarded(0, false), lib.guarded(1, true)]).toEqual(["flagged zero", "other", "other"]);
  expect([lib.zero({ value: 0 }), lib.zero({ value: 3 })]).toEqual(["zero", "other"]);
  expect([lib.every(1, true), lib.every(1, false)]).toEqual(["flag", "no flag"]);
  expect([lib.emptiness([]), lib.emptiness([1])]).toEqual(["empty", "some"]);
  expect([lib.toggle(false), lib.toggle(true)]).toEqual(["off", "on"]);
  const logged: string[] = [];
  const log = console.log;
  console.log = (line: string) => logged.push(line);
  try {
    expect(lib.dropped()).toBe("one");
  } finally {
    console.log = log;
  }
  expect(logged).toEqual(["drop 1", "after"]);
});

// `Option::as_deref` of a `String` or a `Vec` is the option itself: a
// `&str` is the string a `String` is, a slice the array (ADR 0211).
test("Option::as_deref of a String or a Vec is the option itself", async () => {
  const dir = fixture("option-as-deref");
  writeFileSync(join(dir, "lib.rs"), `pub fn path_is(path: Option<String>, want: &str) -> bool {
    path.as_deref() == Some(want)
}
pub fn first(items: Option<Vec<u32>>) -> Option<u32> {
    items.as_deref().and_then(|items| items.first().copied())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("as_deref");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.path_is("/learn", "/learn"), lib.path_is("/a", "/learn"), lib.path_is(undefined, "/learn")]).toEqual([true, false, false]);
  expect([lib.first([4, 5]), lib.first([]), lib.first(undefined)]).toEqual([4, undefined, undefined]);
});
