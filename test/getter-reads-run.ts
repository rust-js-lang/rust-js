// Runs a read case's calls (`getter-reads.test.ts`), each given what JS
// made, a counting `Proxy` of each object it's given: every read of a
// property, a getter's as a field's, every write, test of a key, listing
// of keys and call, logged in the order it's made, and what each call gave
// or threw. Under Node, as the generated JS runs.
//
//   node test/getter-reads-run.ts case.js calls.json log.json

import { readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

import { observe } from "./oracle.ts";

const [module, callsFile, logFile] = process.argv.slice(2);
const calls: { fn: string; args: unknown[] }[] = JSON.parse(readFileSync(callsFile, "utf8"));
const lib = await import(pathToFileURL(module).href);

const log: string[] = [];
// Showing a value reads none of it as the program's.
let quiet = 0;
const paths = new WeakMap<object, string>();
const proxies = new WeakMap<object, object>();

/** `value` as a log shows it: a value JS made by where it is, and any
 * other as JSON. */
function show(value: unknown): string {
  if (typeof value === "function" || (typeof value === "object" && value !== null)) {
    const path = paths.get(value);
    if (path !== undefined) return path;
    if (typeof value === "function") return "a function";
  }
  quiet++;
  try {
    return JSON.stringify(value, (_, v) => (typeof v === "bigint" ? `${v}n` : v)) ?? String(value);
  } catch {
    return String(value);
  } finally {
    quiet--;
  }
}

const note = (line: string) => {
  if (quiet === 0) log.push(line);
};

/** What `value`, read at `path`, is to the program: an object or a
 * function a counting proxy of it, the same one each time it's read. A
 * `{ "$fn": v }` is a function that gives `v`. */
function counted(value: unknown, path: string): unknown {
  if (typeof value !== "object" || value === null) return value;
  const kept = proxies.get(value);
  if (kept) return kept;
  // An array is JS's own, whose items no getter answers: what's counted
  // is what's read of the objects it holds, not how an array's methods
  // read the array, `map`'s `constructor` and each index it `has`.
  if (Array.isArray(value)) {
    const items = value.map((item, i) => counted(item, `${path}[${i}]`));
    proxies.set(value, items);
    paths.set(items, path);
    return items;
  }
  const keys = Object.keys(value);
  const target =
    keys.length === 1 && keys[0] === "$fn"
      ? Object.assign(
          function () {
            return (value as { $fn: unknown }).$fn;
          },
          { toJSON: () => "a function" },
        )
      : value;
  const key = (k: string | symbol) => (typeof k === "symbol" ? `[${k.description}]` : `.${k}`);
  const proxy = new Proxy(target as object, {
    get(t, k, receiver) {
      note(`get ${path}${key(k)}`);
      return counted(Reflect.get(t, k, receiver), `${path}${key(k)}`);
    },
    set(t, k, v, receiver) {
      note(`set ${path}${key(k)} = ${show(v)}`);
      return Reflect.set(t, k, v, receiver);
    },
    has(t, k) {
      note(`has ${path}${key(k)}`);
      return Reflect.has(t, k);
    },
    deleteProperty(t, k) {
      note(`delete ${path}${key(k)}`);
      return Reflect.deleteProperty(t, k);
    },
    ownKeys(t) {
      note(`keys of ${path}`);
      return Reflect.ownKeys(t);
    },
    apply(t, self, args) {
      note(`call ${path}(${args.map(show).join(", ")})`);
      return counted(Reflect.apply(t as (...args: unknown[]) => unknown, self, args), `${path}()`);
    },
  });
  proxies.set(value, proxy);
  paths.set(proxy, path);
  return proxy;
}

const results = calls.map(({ fn, args }, i) => {
  note(`${fn}(..)`);
  const given = args.map((arg, k) => counted(arg, `call${i}.arg${k}`));
  // What it gives, shown quietly, not as `observe` reads it.
  let value: unknown;
  const outcome = observe(() => {
    value = lib[fn](...given);
  });
  return "value" in outcome ? { value: show(value) } : outcome;
});
writeFileSync(logFile, JSON.stringify({ log, results }, null, 1));
