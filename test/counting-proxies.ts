// Counting proxies of what JS made (ADRs 1022, 1087): each object or
// function given as a `Proxy` that logs every read of a property, a
// getter's as a field's, every write, test of a key, listing of keys and
// call, in the order it's made, each by its path. A `{ "$reads": [..] }`
// property is a getter whose answer changes; a `{ "$fn": v }` a function
// that gives `v`. One harness, of getter-reads' cases
// (test/getter-reads-run.ts) and the export net's oracle C
// (test/export-net.ts).

/** A fresh log and the proxies that write it: `counted(value, path)` the
 * proxy of what JS made, `show(value)` a value as the log shows it,
 * `note(line)` a line of the log's own. */
export function countingProxies(): {
  log: string[];
  show: (value: unknown) => string;
  note: (line: string) => void;
  counted: (value: unknown, path: string) => unknown;
} {
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

  // How often each `{ "$reads": [..] }` has been read.
  const answers = new WeakMap<object, number>();

  /** What a getter answers when read: of a `{ "$reads": [a, b, ..] }`, `a` the
   * first time, `b` the second and the last ever after, a getter whose answer
   * changes, which a second read would see (#1908); else `value` itself. */
  function answered(value: unknown): unknown {
    if (typeof value !== "object" || value === null || !Array.isArray((value as { $reads?: unknown }).$reads)) return value;
    const reads = (value as { $reads: unknown[] }).$reads;
    const n = answers.get(value) ?? 0;
    answers.set(value, n + 1);
    return reads[Math.min(n, reads.length - 1)];
  }

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
    // Each `{ "$reads": [..] }` property a getter of its object, as JS writes
    // one, so what reads it by its descriptor, a boundary check, sees an
    // accessor, not the answers (#2403); each read through the proxy runs it.
    for (const k of keys) {
      const spec = (value as Record<string, unknown>)[k];
      if (typeof spec === "object" && spec !== null && Array.isArray((spec as { $reads?: unknown }).$reads)) {
        Object.defineProperty(value, k, { get: () => answered(spec), enumerable: true, configurable: true });
      }
    }
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
        return counted(answered(Reflect.get(t, k, receiver)), `${path}${key(k)}`);
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

  return { log, show, note, counted };
}
