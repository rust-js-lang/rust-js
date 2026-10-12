// What JS has for the code rust-js writes: Node's and browsers', not Bun's
// own. Bun gives its iterators a `join` and `includes`, its maps a
// `getOrInsert`, and more, which Node and browsers don't have: code that
// calls one passes in Bun and throws there. The tests' preload takes Bun's
// away (`asNode`), and `node-surface.test.ts` checks against Node that it's
// all of them.

/** The objects code reaches built-ins through, by the name a list gives. */
export function surfaces(): Record<string, object | undefined> {
  const iterator = Object.getPrototypeOf(Object.getPrototypeOf([].values()));
  return {
    "Iterator.prototype": iterator,
    "Array.prototype": Array.prototype,
    "String.prototype": String.prototype,
    "Map.prototype": Map.prototype,
    "Set.prototype": Set.prototype,
    "Promise.prototype": Promise.prototype,
    Object,
    Array,
    Iterator: (globalThis as { Iterator?: object }).Iterator,
    Math,
    Number,
    BigInt,
    Promise,
    JSON,
  };
}

/** Bun's own, which Node doesn't have, by where they are. */
export const bunOnly: Record<string, string[]> = {
  "Iterator.prototype": ["chunks", "includes", "join", "windows"],
  "Map.prototype": ["getOrInsert", "getOrInsertComputed"],
  Iterator: ["concat", "zip", "zipKeyed"],
  Math: ["sumPrecise"],
};

/** Bun's own built-ins taken away, as Node has none of them. */
export function asNode() {
  const at = surfaces();
  for (const [where, names] of Object.entries(bunOnly)) {
    for (const name of names) delete (at[where] as Record<string, unknown> | undefined)?.[name];
  }
}
