// How much of TypeScript's DOM the webapi crate binds: each class
// `lib.dom.d.ts` declares (`@types/web`, which TypeScript generates it as),
// and each of its members, against the members src/lib.rs's MDN links name.
// See docs/research/web-platform-coverage.md.

import { readFileSync } from "node:fs";

import idl from "@webref/idl";
import { open } from "@rust-js/typescript";

/** A class's members, and of them its methods, and what each that gives a
 * `NodeListOf<X>` lists, `querySelectorAll`'s `Element`. */
export type Members = {
  members: Set<string>;
  methods: Set<string>;
  statics: Set<string>;
  ctor: boolean;
  extends: string[];
  lists: Map<string, string>;
  /** Its properties TypeScript types as a function, `onclick`'s `((ev) => any) | null`. */
  functions: Set<string>;
};

/** Whether TypeScript's `type` is a function's, or none, `| null`. */
const isFunction = (type: any): boolean =>
  type?.kind === "function" ||
  (type?.kind === "union" &&
    type.types.some((t: any) => t.kind === "function") &&
    type.types.every((t: any) => t.kind === "function" || (t.kind === "keyword" && ["null", "undefined"].includes(t.keyword))));

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
/** What a member's `NodeListOf<X>` (or `| null`) lists, `X`, or its type
 * parameter's constraint, `querySelectorAll<E extends Element>`'s
 * `Element`: of the first overload that names one. */
function listed(member: any): string | undefined {
  const type = member.kind === "method" ? member.returns : member.type;
  const list = (type?.kind === "union" ? type.types : [type]).find((t: any) => t?.kind === "reference" && t.name === "NodeListOf");
  const item = list?.args[0];
  if (item?.kind !== "reference") return undefined;
  const param = (member.typeParameters ?? []).find((p: any) => p.name === item.name);
  const named = param ? param.constraint : item;
  return named?.kind === "reference" ? named.name : undefined;
}

export async function typescript(): Promise<Map<string, Members>> {
  const session = await open([dom]);
  const { declarations } = await session.read(dom);
  await session.close();
  const interfaces = new Map<string, Members>();
  const of = (name: string) => {
    if (!interfaces.has(name)) interfaces.set(name, { members: new Set(), methods: new Set(), statics: new Set(), ctor: false, extends: [], lists: new Map(), functions: new Set() });
    return interfaces.get(name)!;
  };
  const classes = new Set<string>();
  for (const d of declarations as any[]) {
    if (d.kind === "interface") {
      const m = of(d.name);
      for (const e of d.extends) if (e.kind === "reference") m.extends.push(e.name);
      for (const member of d.members) if (member.kind === "method" || member.kind === "property") m.members.add(member.name);
      for (const member of d.members) if (member.kind === "method") m.methods.add(member.name);
      for (const member of d.members) if (member.kind === "property" && isFunction(member.type)) m.functions.add(member.name);
      for (const member of d.members) {
        const item = listed(member);
        if (item && !m.lists.has(member.name)) m.lists.set(member.name, item);
      }
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
    // Its methods and lists, its mixins' and its parents' too.
    const methods = new Set<string>();
    const functions = new Set<string>();
    const lists = new Map<string, string>();
    const add = (n: string, seen: Set<string>) => {
      if (seen.has(n)) return;
      seen.add(n);
      for (const x of of(n).methods) methods.add(x);
      for (const x of of(n).functions) functions.add(x);
      for (const [x, item] of of(n).lists) if (!lists.has(x)) lists.set(x, item);
      for (const e of of(n).extends) add(e, seen);
    };
    add(name, new Set());
    out.set(name, { ...m, members, methods, lists, functions });
  }
  return out;
}

/** What src/lib.rs binds: each module function or method, by the JS its
 * link name says, `get x` and `set x` are `x`, `new X` the constructor,
 * `X.y` and `get X.y` the static `y`, or its own name; and each constant, a member and
 * a static of its name. */
function webapi(): { bound: Set<string>; types: Set<string>; parents: Map<string, string>; objects: Set<string> } {
  const text = readFileSync(lib, "utf8");
  const bound = new Set<string>();
  // Each property read as a `JsObject`, `Document.onclick` before ADR 0653.
  const objects = new Set<string>();
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
    const impl = l.match(/^impl(?:<\w+>)? (\w+)(?:<\w+>)? \{$/)?.[1];
    if (impl) type = impl;
    if (l === "pub trait EventTargetExt: IsA<EventTarget> {") type = "EventTarget";
    if (l === "}") type = "";
    link = l.match(/link_name = "([^"]+)"/)?.[1] ?? link;
    const fn = l.match(/^\s+(?:pub (?:safe )?)?fn (?:r#)?(\w+)/)?.[1];
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
      if (js.startsWith("get ") && /->.*\bJsObject\b/.test(l)) objects.add(`${type}.${member}`);
    }
    if (fn) link = undefined;
  }
  const types = new Set([...text.matchAll(/^pub struct (\w+)/gm)].map((m) => m[1]));
  // What each type derefs to: a member of its parent is one of its own.
  const parents = new Map([...text.matchAll(/^impl Deref for (\w+) \{\n\s+type Target = (\w+);/gm)].map((m) => [m[1], m[2]]));
  return { bound, types, parents, objects };
}

/** Each property TypeScript types as a function that the crate reads as an
 * object, a `JsObject`, where a function JS gives is a `dyn Fn` of its
 * signature (ADR 0653): none. */
export async function functionsAsObjects(): Promise<string[]> {
  const ts = await typescript();
  const { objects } = webapi();
  return [...objects].filter((x) => ts.get(x.slice(0, x.indexOf(".")))?.functions.has(x.slice(x.indexOf(".") + 1))).sort();
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
