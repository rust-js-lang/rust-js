// How much of TypeScript's DOM the webapi crate binds: each class
// `lib.dom.d.ts` declares (`@types/web`, which TypeScript generates it as),
// and each of its members, against the members src/lib.rs's MDN links name.
// See docs/research/web-platform-coverage.md.

import { readFileSync } from "node:fs";

import idl from "@webref/idl";
import { open } from "@rust-js/typescript";

/** A class's members, and of them its methods. */
export type Members = { members: Set<string>; methods: Set<string>; statics: Set<string>; ctor: boolean; extends: string[] };

export type Coverage = {
  /** Every member of TypeScript's classes, `Element.append`. */
  all: string[];
  /** Those the crate binds. */
  covered: string[];
  /** TypeScript's classes, and those the crate has a type of. */
  classes: string[];
  typed: string[];
};

const dom = new URL("./node_modules/@types/web/index.d.ts", import.meta.url).pathname;
const lib = new URL("./src/lib.rs", import.meta.url).pathname;

/** TypeScript's classes and their members: their own, their mixins'
 * (`ParentNode`'s), not their parent class's, which they have by `Deref`;
 * their statics, `static:supports`, and a constructor. */
export async function typescript(): Promise<Map<string, Members>> {
  const session = await open([dom]);
  const { declarations } = await session.read(dom);
  await session.close();
  const interfaces = new Map<string, Members>();
  const of = (name: string) => {
    if (!interfaces.has(name)) interfaces.set(name, { members: new Set(), methods: new Set(), statics: new Set(), ctor: false, extends: [] });
    return interfaces.get(name)!;
  };
  const classes = new Set<string>();
  for (const d of declarations as any[]) {
    if (d.kind === "interface") {
      const m = of(d.name);
      for (const e of d.extends) if (e.kind === "reference") m.extends.push(e.name);
      for (const member of d.members) if (member.kind === "method" || member.kind === "property") m.members.add(member.name);
      for (const member of d.members) if (member.kind === "method") m.methods.add(member.name);
    } else if (d.kind === "const" && d.type.kind === "object" && d.type.members.some((x: any) => x.name === "prototype")) {
      classes.add(d.name);
      const m = of(d.name);
      for (const x of d.type.members) {
        if (x.kind === "other" && /^new\s*[(<]/.test(x.text)) m.ctor = true;
        else if (x.name && x.name !== "prototype") m.statics.add(x.name);
      }
    }
  }
  const everything = (name: string, seen: Set<string>): string[] =>
    of(name).extends.flatMap((e) => (seen.has(e) ? [] : (seen.add(e), [...of(e).members, ...everything(e, seen)])));
  const mixins = (name: string, seen: Set<string>): string[] =>
    of(name).extends.flatMap((e) =>
      classes.has(e) || seen.has(e) ? [] : (seen.add(e), [...of(e).members, ...mixins(e, seen)]),
    );
  const out = new Map<string, Members>();
  for (const name of classes) {
    const m = of(name);
    const parents = new Set(
      m.extends.filter((e) => classes.has(e)).flatMap((e) => [...of(e).members, ...everything(e, new Set())]),
    );
    const members = new Set([...m.members, ...mixins(name, new Set())].filter((x) => !parents.has(x)));
    // Its methods, its mixins' and its parents' too.
    const methods = new Set<string>();
    const add = (n: string, seen: Set<string>) => {
      if (seen.has(n)) return;
      seen.add(n);
      for (const x of of(n).methods) methods.add(x);
      for (const e of of(n).extends) add(e, seen);
    };
    add(name, new Set());
    out.set(name, { ...m, members, methods });
  }
  return out;
}

/** What src/lib.rs binds: each module function or method, by the JS its
 * link name says, `get x` and `set x` are `x`, `new X` the constructor,
 * `X.y` and `get X.y` the static `y`, or its own name; and each constant, a member and
 * a static of its name. */
function webapi(): { bound: Set<string>; types: Set<string>; parents: Map<string, string> } {
  const text = readFileSync(lib, "utf8");
  const bound = new Set<string>();
  // The type a module is of: its JS name, `rust_js::name` or its own.
  let struct = "";
  let named = "";
  let type = "";
  let link: string | undefined;
  for (const l of text.split("\n")) {
    named = l.match(/rust_js::name = "([\w.]+)"/)?.[1] ?? named;
    const s = l.match(/^pub struct (\w+)/)?.[1];
    if (s) {
      struct = named || s;
      named = "";
    }
    if (/^pub mod /.test(l)) type = struct;
    const impl = l.match(/^impl (\w+) \{$/)?.[1];
    if (impl) type = impl;
    if (l === "pub trait EventTargetExt: IsA<EventTarget> {") type = "EventTarget";
    if (l === "}") type = "";
    link = l.match(/link_name = "([^"]+)"/)?.[1] ?? link;
    const fn = l.match(/^\s+(?:pub (?:safe )?)?fn (\w+)/)?.[1];
    const constant = l.match(/^\s+pub const (\w+):/)?.[1];
    if (type && constant) for (const m of [constant, `static:${constant}`]) bound.add(`${type}.${m}`);
    if (type && fn) {
      const js = link ?? fn;
      // `get Notification.permission`, a static attribute, is a static too.
      const plain = js.replace(/^(get|set) /, "");
      const member =
        js.startsWith("new ") ? "constructor"
        // `iter(items)`, `Iterator.from(items)`: what `for..of` calls.
        : js === "Iterator.from" ? "[Symbol.iterator]"
        : /^[A-Z][\w.]*\.\w+$/.test(plain) ? `static:${plain.split(".").pop()}`
        : plain;
      bound.add(`${type}.${member}`);
    }
    if (fn) link = undefined;
  }
  const types = new Set([...text.matchAll(/^pub struct (\w+)/gm)].map((m) => m[1]));
  // What each type derefs to: a member of its parent is one of its own.
  const parents = new Map([...text.matchAll(/^impl Deref for (\w+) \{\n\s+type Target = (\w+);/gm)].map((m) => [m[1], m[2]]));
  return { bound, types, parents };
}

/** The classes WebIDL gives a constructor JS can call: not one TypeScript
 * declares where WebIDL has none, `new Node()`, nor an `[HTMLConstructor]`
 * element's, which only a custom element's `super()` calls. */
async function constructible(): Promise<Set<string>> {
  const all = Object.values((await idl.parseAll()) as Record<string, any[]>).flat();
  // HTML marks the constructor itself, `[HTMLConstructor] constructor();`.
  const html = (x: any) => (x.extAttrs ?? []).some((a: any) => a.name === "HTMLConstructor");
  return new Set(
    all.filter((d) => d.type === "interface" && !html(d) && (d.members ?? []).some((m: any) => m.type === "constructor" && !html(m))).map((d) => d.name),
  );
}

export async function measure(): Promise<Coverage> {
  const ts = await typescript();
  const made = await constructible();
  const { bound, types, parents } = webapi();
  // Bound on the class or on one it derefs to, as a method call finds it.
  const has = (x: string) => {
    const dot = x.indexOf(".");
    for (let c: string | undefined = x.slice(0, dot); c; c = parents.get(c)) if (bound.has(c + x.slice(dot))) return true;
    return false;
  };
  const all: string[] = [];
  for (const [name, m] of ts) {
    for (const x of m.members) all.push(`${name}.${x}`);
    for (const x of m.statics) all.push(`${name}.static:${x}`);
    if (m.ctor && made.has(name)) all.push(`${name}.constructor`);
  }
  all.sort();
  const classes = [...ts.keys()].sort();
  // A CSS descriptor's dashed name, `margin-top`, is the property its
  // camelCase twin is, `marginTop`: bound where that is.
  const camel = (x: string) => x.replace(/-([a-z])/g, (_, c) => c.toUpperCase());
  const covered = (x: string) =>
    x.includes(".static:") || x.endsWith(".constructor") ? bound.has(x) : has(x) || (x.includes("-") && has(camel(x)));
  return { all, covered: all.filter(covered), classes, typed: classes.filter((c) => types.has(c)) };
}

/** The baseline's text: a summary, then each member bound. */
export function render(c: Coverage): string {
  const pct = (a: number, b: number) => ((100 * a) / b).toFixed(1);
  return [
    `# The webapi crate against TypeScript's DOM (@types/web): bun test test/webapi-coverage.test.ts`,
    `# classes: ${c.typed.length} of ${c.classes.length} (${pct(c.typed.length, c.classes.length)}%)`,
    `# members: ${c.covered.length} of ${c.all.length} (${pct(c.covered.length, c.all.length)}%)`,
    ...c.covered,
    "",
  ].join("\n");
}
