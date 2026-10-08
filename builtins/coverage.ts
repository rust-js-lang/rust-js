// How much of JS's standard library the js crate binds, where Rust has no
// type of its own for it: each built-in TypeScript's ES2024 libs declare
// (TypeScript 5.9's `lib.es*.d.ts`, pinned as `typescript-es`), its members,
// statics and constructor, against what src binds by each function's link
// name. `Array`, `String`, `Number`, `Math`, `Map`, `Set` and `Object` are
// Rust's own types' (`Vec`, `str`, `f64`, `HashMap`): not counted.
// See docs/decisions/0283-js-crate-covers-typescripts-es-library.md.

import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

import { open } from "@rust-js/typescript";

/** The built-ins measured: classes, namespaces and objects of functions. */
export const SCOPE = [
  "Date", "RegExp", "Error", "Promise", "Symbol",
  "ArrayBuffer", "SharedArrayBuffer", "DataView",
  "Int8Array", "Uint8Array", "Uint8ClampedArray", "Int16Array", "Uint16Array", "Int32Array", "Uint32Array",
  "Float32Array", "Float64Array", "BigInt64Array", "BigUint64Array",
  "WeakMap", "WeakSet", "WeakRef", "FinalizationRegistry",
  "Proxy", "Reflect", "Atomics", "JSON", "Intl",
  "Intl.Collator", "Intl.DateTimeFormat", "Intl.DisplayNames", "Intl.ListFormat", "Intl.Locale",
  "Intl.NumberFormat", "Intl.PluralRules", "Intl.RelativeTimeFormat", "Intl.Segmenter",
];

/** The global functions measured, `globalThis.parseInt`. */
export const GLOBALS = ["parseInt", "parseFloat", "isNaN", "isFinite", "encodeURI", "encodeURIComponent", "decodeURI", "decodeURIComponent"];

/** A Rust type whose JS class isn't its name. */
const CLASS_OF: Record<string, string> = { JsError: "Error" };

export type Coverage = { all: string[]; covered: string[] };

const libs = new URL("../node_modules/typescript-es/lib/", import.meta.url).pathname;
const src = new URL("./src/", import.meta.url).pathname;

/** Each built-in's members: instance ones by name, `static:` ones, and
 * `constructor` where `new` makes one. */
async function typescript(): Promise<Map<string, Set<string>>> {
  const files = readdirSync(libs)
    .filter((f) => /^lib\.es(5|20(1[5-9]|2[0-4]))(\.[a-z.]+)?\.d\.ts$/.test(f) && !f.includes(".full."))
    .map((f) => join(libs, f));
  const session = await open(files);
  const interfaces = new Map<string, Set<string>>();
  const constructors = new Map<string, { statics: Set<string>; ctor: boolean } | string>();
  const functions = new Set<string>();
  const named = (m: any) => (m.kind === "method" || m.kind === "property") && /^[A-Za-z_$][\w$]*$/.test(m.name);
  const walk = (declarations: any[], prefix: string) => {
    for (const d of declarations) {
      const name = prefix + d.name;
      if (d.kind === "interface") {
        const set = interfaces.get(name) ?? new Set<string>();
        for (const m of d.members) if (named(m)) set.add(m.name);
        interfaces.set(name, set);
      } else if (d.kind === "const") {
        if (d.type.kind === "reference") constructors.set(name, prefix + d.type.name);
        else if (d.type.kind === "object") {
          const statics = new Set<string>(d.type.members.filter(named).map((m: any) => m.name));
          const ctor = d.type.members.some((m: any) => m.kind === "other" && /^new\s*[(<]/.test(m.text));
          constructors.set(name, { statics, ctor });
        }
      } else if (d.kind === "function") {
        functions.add(name);
      } else if (d.kind === "namespace") {
        walk(d.declarations, `${name}.`);
      }
    }
  };
  for (const f of files) walk((await session.read(f)).declarations, "");
  // A constructor interface's construct signatures aren't members the model
  // names: read them from its text.
  const ctorText = new Map<string, boolean>();
  for (const f of files) {
    const text = readFileSync(f, "utf8");
    for (const m of text.matchAll(/interface (\w+Constructor)\s*(?:<[^>]*>)?\s*\{([\s\S]*?)\n\}/g)) {
      if (/^\s*new\s*[(<]/m.test(m[2])) ctorText.set(m[1], true);
    }
  }
  await session.close();
  const out = new Map<string, Set<string>>();
  for (const name of SCOPE) {
    const members = new Set<string>();
    const ctor = constructors.get(name);
    const own = ctor === name;
    if (!own) for (const m of interfaces.get(name) ?? []) members.add(m);
    if (typeof ctor === "string") {
      for (const m of interfaces.get(ctor) ?? []) if (m !== "prototype") members.add(own ? `static:${m}` : `static:${m}`);
      if (!own && ctorText.get(ctor.split(".").pop()!)) members.add("constructor");
    } else if (ctor) {
      for (const m of ctor.statics) if (m !== "prototype") members.add(`static:${m}`);
      if (ctor.ctor) members.add("constructor");
    }
    // A namespace's functions are its statics: `Reflect.apply`, `Intl.getCanonicalLocales`.
    for (const f of functions) if (f.startsWith(`${name}.`) && !f.slice(name.length + 1).includes(".")) members.add(`static:${f.slice(name.length + 1)}`);
    out.set(name, members);
  }
  out.set("globalThis", new Set(GLOBALS.filter((g) => functions.has(g))));
  return out;
}

/** What src binds: each function by its link name and receiver. `new X`
 * is X's constructor; `X.y`, `Intl.X.y`, X's static `y`; a method of an
 * `impl X`, or one taking `this: &X`, X's member, `get y` and `set y` its
 * `y`; any other name a global function's. */
function bindings(): Set<string> {
  const bound = new Set<string>();
  const files = readdirSync(src, { recursive: true }).map(String).filter((f) => f.endsWith(".rs"));
  const classes = new Map<string, string>();
  for (const f of files) {
    const lines = readFileSync(join(src, f), "utf8").split("\n");
    lines.forEach((l, i) => {
      const s = l.match(/^\s*pub struct (\w+)/)?.[1];
      if (!s) return;
      const name = lines.slice(Math.max(0, i - 4), i).join("\n").match(/rust_js::name = "([\w.]+)"/)?.[1];
      classes.set(s, CLASS_OF[s] ?? name ?? s);
    });
  }
  for (const f of files) {
    let depth = 0;
    const impls: { type: string; depth: number }[] = [];
    let link: string | undefined;
    for (const l of readFileSync(join(src, f), "utf8").split("\n")) {
      const impl = l.match(/^\s*impl(?:<[^>]*>)? (\w+)(?:<[^>]*>)? \{/)?.[1];
      link = l.match(/link_name = "([^"]+)"/)?.[1] ?? link;
      const fn = l.match(/^\s*pub (?:safe )?fn (\w+)(?:<[^(]*>)?\((.*)/);
      if (fn) {
        const js = link ?? fn[1];
        const receiver = /^\s*&?(?:mut )?self\b/.test(fn[2]) || /^self\b/.test(fn[2])
          ? impls.at(-1)?.type
          : fn[2].match(/^this: (?:&(?:'\w+ )?)?(\w+)/)?.[1];
        const cls = receiver ? classes.get(receiver) : undefined;
        const path = js.match(/^new ([\w.]+)$/)?.[1];
        if (path) bound.add(`${path}.constructor`);
        else if (/^[A-Z][\w]*(\.[A-Z]\w*)*\.\w+$/.test(js)) {
          const dot = js.lastIndexOf(".");
          bound.add(`${js.slice(0, dot)}.static:${js.slice(dot + 1)}`);
        } else if (cls && /^(?:(get|set) )?[\w$]+$/.test(js)) bound.add(`${cls}.${js.replace(/^(get|set) /, "")}`);
        else if (!receiver && /^[a-zA-Z]\w*$/.test(js)) bound.add(`globalThis.${js}`);
        link = undefined;
      }
      for (const c of l) {
        if (c === "{") depth++;
        else if (c === "}") {
          depth--;
          while (impls.length && impls.at(-1)!.depth > depth) impls.pop();
        }
      }
      if (impl) impls.push({ type: impl, depth });
    }
  }
  return bound;
}

export async function measure(): Promise<Coverage> {
  const ts = await typescript();
  const bound = bindings();
  const all = [...ts].flatMap(([name, members]) => [...members].map((m) => `${name}.${m}`)).sort();
  return { all, covered: all.filter((x) => bound.has(x)) };
}

/** The baseline's text: a summary, then each member bound. */
export function render(c: Coverage): string {
  return [
    `# The js crate against TypeScript's ES2024 library, where Rust has no type of its own: bun test test/builtins-coverage.test.ts`,
    `# members: ${c.covered.length} of ${c.all.length} (${((100 * c.covered.length) / c.all.length).toFixed(1)}%)`,
    ...c.covered,
    "",
  ].join("\n");
}
