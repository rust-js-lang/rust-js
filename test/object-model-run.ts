// Runs a program whose items a library holds (`object-model.test.ts`),
// each value the library gives it read through a counting `Proxy`: a
// membrane at the library's exports, as JS would give the program an
// object it made. What the program does with each, every read of a
// property, write, test of a key, listing of keys, call and the `this` a
// call is given, `new`, is logged in the order it's made; what the
// library does with its own values isn't, as it's given them back whole.
// Under Node, as the generated JS runs; how `main` ended goes to the
// outcome file, as `corpus-run.ts` writes it.
//
//   node test/object-model-run.ts program.js library/lib.js outcome.json log.json

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

import { observe } from "./oracle.ts";

const [program, library, outcomeFile, logFile] = process.argv.slice(2);
// Its arguments as `corpus-run.ts` gives them, native's count (ADR 0391).
process.argv = [process.argv[0], program, outcomeFile];

const log: string[] = [];
// What a log's own showing of a value reads, and what the library does
// while it runs, are none of the program's reads.
let quiet = 0;
const note = (line: string) => {
  if (quiet === 0) log.push(line);
};

/** Each proxy's object, and each object's proxy, so the program sees one
 * proxy of an object however it reaches it, and the library its object. */
const targets = new WeakMap<object, object>();
const proxies = new WeakMap<object, object>();
const paths = new WeakMap<object, string>();
const made = new Map<string, number>();

const isObject = (v: unknown): v is object => (typeof v === "object" && v !== null) || typeof v === "function";

/** What the library is given back of what it gave: its own object. */
function raw(value: unknown): unknown {
  return isObject(value) ? (targets.get(value) ?? value) : value;
}

/** What the program gave the library, which is the program's own however
 * the library gives it back, `untagged(queue)`'s pointer. */
const programs = new WeakSet<object>();

/** `value`, given the library: its own, the program's noted. */
function given(value: unknown): unknown {
  const own = raw(value);
  if (isObject(own) && !targets.has(value as object)) programs.add(own);
  return own;
}

/** An object whose methods read slots a `Proxy` hasn't, a `Map`'s, or that
 * no getter could answer, given as it is. */
function exotic(value: object): boolean {
  return (
    value instanceof Map ||
    value instanceof Set ||
    value instanceof WeakMap ||
    value instanceof WeakSet ||
    value instanceof Promise ||
    value instanceof Date ||
    value instanceof RegExp ||
    value instanceof ArrayBuffer ||
    ArrayBuffer.isView(value)
  );
}

const key = (k: string | symbol) => (typeof k === "symbol" ? `[${k.description}]` : `.${k}`);

/** `value` as a log shows it: one the library gave by where it was read,
 * the program's own by its kind, whose form a readability pass may choose,
 * and any other as JSON. */
function show(value: unknown): string {
  if (isObject(value)) {
    const path = paths.get(value);
    if (path !== undefined) return path;
    return typeof value === "function" ? "a function" : Array.isArray(value) ? "an array" : "an object";
  }
  if (typeof value === "bigint") return `${value}n`;
  return JSON.stringify(value) ?? String(value);
}

/** A call's arguments as the log shows them: those a call leaves off, a
 * last `undefined`, aren't any the callee can tell from none. */
function shown(args: unknown[]): string {
  let end = args.length;
  while (end > 0 && args[end - 1] === undefined) end--;
  return args.slice(0, end).map(show).join(", ");
}

/** The library's call of `f`, given what it gave back as its own. */
function libraryCall(run: () => unknown): unknown {
  quiet++;
  try {
    return run();
  } finally {
    quiet--;
  }
}

/** What the program sees of `value`, which the library gave at `path`: a
 * counting proxy of an object or a function, the same one each time. An
 * array is JS's own, whose items no getter answers: its items are counted
 * as they're read, not its indices nor how its methods read it. */
function counted(value: unknown, path: string, exported = false, bound: unknown[] = [], protocol = false, method = false): unknown {
  if (!isObject(value) || targets.has(value) || programs.has(value)) return value;
  const kept = proxies.get(value);
  if (kept) return kept;
  if (exotic(value)) return value;
  const array = Array.isArray(value);
  const fn = typeof value === "function";
  const at = () => paths.get(proxy) ?? path;
  const proxy: object = new Proxy(value, {
    get(t, k) {
      // Read while the library runs, of a proxy the program left in its
      // own object, `root.children[0]`: the library's own value, so
      // `Rc::ptr_eq` of it is as it was made.
      if (quiet > 0) return Reflect.get(t, k, t);
      // A symbol's, `Symbol.iterator`'s, is JS's protocol, not a field: its
      // method runs as the library's, on its own object, uncounted.
      // An array's is JS's own, run on the proxy, whose items it gives
      // counted.
      if (typeof k === "symbol") {
        const v = Reflect.get(t, k, array ? proxy : t);
        return typeof v === "function" && !array ? counted(v, `${at()}${key(k)}`, false, [], true) : v;
      }
      // A function of what the module exports, a class's or a namespace's,
      // `Parser.expr`, is the module's, read where JS reads a callee: what
      // it makes is counted, and a constant's data. Of a function, only
      // its own constant is data: `bind` and `call` are JS's.
      const own = Reflect.getOwnPropertyDescriptor(t, k);
      const data = own !== undefined && "value" in own && typeof own.value !== "function";
      const callee = (exported || fn) && !data;
      // A function bound to arguments is the same call, given them first.
      if (fn && k === "bind") {
        return (self: unknown, ...more: unknown[]) =>
          counted((t as (...a: unknown[]) => unknown).bind(raw(self), ...more.map(raw)), at(), false, [...bound, ...more]);
      }
      const v = libraryCall(() => Reflect.get(t, k, t));
      // A method's read is where JS reads a callee, once for a loop's
      // `next` as `for..of` reads it: its call is what's counted.
      if (!array && !callee && typeof v !== "function") note(`get ${at()}${key(k)}`);
      // What the proxy must give as it is (an own constant), and a
      // prototype, which `instanceof` compares, aren't counted further.
      if (k === "prototype" || k === "constructor" || (own && !own.configurable && !own.writable)) return v;
      if (array && (typeof k === "symbol" || !/^\d+$/.test(k))) return v;
      return counted(v, `${at()}${key(k)}`, false, [], false, true);
    },
    set(t, k, v) {
      if (!array && typeof k !== "symbol") note(`set ${at()}${key(k)} = ${show(v)}`);
      return Reflect.set(t, k, given(v), t);
    },
    has(t, k) {
      if (!array && typeof k !== "symbol") note(`has ${at()}${key(k)}`);
      return Reflect.has(t, k);
    },
    deleteProperty(t, k) {
      note(`delete ${at()}${key(k)}`);
      return Reflect.deleteProperty(t, k);
    },
    ownKeys(t) {
      if (!array) note(`keys of ${at()}`);
      return Reflect.ownKeys(t);
    },
    apply(t, self, args) {
      // What it's given past its parameters, `map`'s index and array, it
      // can't read; the `this` of a method, read of an object, it can.
      const takes = (t as (...a: unknown[]) => unknown).length;
      const read = [...bound, ...(takes === 0 ? args : args.slice(0, takes))];
      if (!protocol) note(`call ${at()}(${shown(read)})${method ? ` this ${self === undefined ? "undefined" : show(self)}` : ""}`);
      return counted(
        libraryCall(() => Reflect.apply(t as (...a: unknown[]) => unknown, raw(self), args.map(given))),
        `${at()}()`,
      );
    },
    construct(t, args) {
      note(`new ${at()}(${shown(args)})`);
      return counted(
        libraryCall(() => Reflect.construct(t as new (...a: unknown[]) => object, args.map(given))),
        `new ${at()}`,
      ) as object;
    },
  });
  // Each object of a path its own name, `new Queen#2` the second made; a
  // bound function its function's.
  const nth = bound.length > 0 ? 1 : (made.get(path) ?? 0) + 1;
  if (bound.length === 0) made.set(path, nth);
  proxies.set(value, proxy);
  targets.set(proxy, value);
  paths.set(proxy, nth === 1 ? path : `${path}#${nth}`);
  return proxy;
}

// The library, each of its modules the program imports loaded first, as
// the program's import would load it, its exports given the program
// through the membrane: the program's import of it is made an import of a
// module of the same names, each counted. A module's exports are named as
// they are, the module being the library's only one or the program's
// namespace of it, `Plugin.isEnabled`, naming it where it's read.
const libraryDir = dirname(resolve(library));

/** The names a module exports of a `let`, a binding it may assign again,
 * which a module of the program's can only read live as itself. */
function reassigned(text: string): Set<string> {
  const lets = new Set([...text.matchAll(/^(?:export )?let (\w+)/gm)].map((m) => m[1]));
  const names = new Set([...text.matchAll(/^export let (\w+)/gm)].map((m) => m[1]));
  for (const list of text.matchAll(/^export \{([^}]*)\}/gm)) {
    for (const item of list[1].split(",")) {
      const [local, as] = item.trim().split(/\s+as\s+/);
      if (local && lets.has(local)) names.add((as ?? local).replace(/^"|"$/g, ""));
    }
  }
  return names;
}
const programDir = dirname(resolve(program));
const membranes = new Map<string, string>();
const modules: Record<string, unknown>[] = [];
let source = readFileSync(program, "utf8");
const specifiers = [...source.matchAll(/from\s+(["'])([^"']+)\1/g)].map((m) => m[2]);
for (const specifier of specifiers) {
  const file = specifier.startsWith(".") ? resolve(programDir, specifier) : undefined;
  if (file === undefined || !file.startsWith(`${libraryDir}/`) || membranes.has(specifier)) continue;
  // Through a module of its own, as a module that exports a `then` is a
  // thenable `import()` would call.
  const boxed = join(programDir, `object-model-${process.pid}-${modules.length + 1}-box.mjs`);
  writeFileSync(boxed, `import * as module from ${JSON.stringify(pathToFileURL(file).href)};\nexport const exports = { module };\n`);
  const exports: Record<string, unknown> = (await import(pathToFileURL(boxed).href)).exports.module;
  const live = reassigned(readFileSync(file, "utf8"));
  const names = Object.keys(exports).filter((name) => !live.has(name));
  modules.push(Object.fromEntries(names.map((name) => [name, counted(exports[name], name, true)])));
  const membrane = `object-model-${process.pid}-${modules.length}.mjs`;
  membranes.set(specifier, membrane);
  writeFileSync(
    join(programDir, membrane),
    [
      // By names of their own, as an export may be named `case` or `given`.
      ...names.map((name, i) => `const $${i} = globalThis.$objectModel[${modules.length - 1}][${JSON.stringify(name)}];`),
      `export { ${names.map((name, i) => `$${i} as ${JSON.stringify(name)}`).join(", ")} };`,
      // What the library assigns again, a `static mut`'s `let`, the
      // module's own binding, read as it is now, uncounted.
      ...(live.size > 0 ? [`export { ${[...live].map((name) => JSON.stringify(name)).join(", ")} } from ${JSON.stringify(pathToFileURL(file).href)};`] : []),
      "",
    ].join("\n"),
  );
}
(globalThis as { $objectModel?: unknown }).$objectModel = modules;
source = source.replaceAll(/(from\s+)(["'])([^"']+)\2/g, (whole, from: string, quote: string, specifier: string) => {
  const membrane = membranes.get(specifier);
  return membrane === undefined ? whole : `${from}${quote}./${membrane}${quote}`;
});
// A program that uses none of the library's items as its own, each
// inlined or copied, runs as it is, reading nothing it gave.
const membraned = membranes.size === 0 ? resolve(program) : join(programDir, `object-model-${process.pid}.mjs`);
if (membranes.size > 0) writeFileSync(membraned, source);

let ended = false;
process.once("exit", (code) => {
  if (!ended) writeFileSync(outcomeFile, JSON.stringify({ exit: code }));
  writeFileSync(logFile, log.join("\n"));
});
const run = await import(pathToFileURL(membraned).href).then(
  ({ entry }) => entry,
  (error) => () => {
    throw error;
  },
);
const outcome = observe(run);
ended = true;
writeFileSync(outcomeFile, JSON.stringify(outcome));
