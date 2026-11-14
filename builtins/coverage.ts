// How much of JS's standard library the js crate binds, where Rust has no
// type of its own for it: each built-in TypeScript's ES2024 libs declare
// (TypeScript 5.9's `lib.es*.d.ts`, pinned as `typescript-es`), its members,
// statics and constructor, against what src binds by each function's link
// name. `Array`, `String`, `Number`, `Math`, `Map` and `Set` are Rust's own
// types' (`Vec`, `str`, `f64`, `HashMap`): not counted. Of `Object`, its
// constructor's statics are (ADR 0666): its own members, `toString` and
// `hasOwnProperty`, are every value's, and `new Object()` a struct's.
// See docs/decisions/0283-js-crate-covers-typescripts-es-library.md.

import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

import { checkedMembers, open } from "@rust-js/typescript";
import { snake } from "../scripts/rust-names";

/** The built-ins measured: classes, namespaces and objects of functions. */
export const SCOPE = [
  "Date", "RegExp", "Error", "AggregateError",
  "EvalError", "RangeError", "ReferenceError", "SyntaxError", "TypeError", "URIError",
  "Promise", "Symbol",
  "ArrayBuffer", "SharedArrayBuffer", "DataView",
  "Int8Array", "Uint8Array", "Uint8ClampedArray", "Int16Array", "Uint16Array", "Int32Array", "Uint32Array",
  "Float32Array", "Float64Array", "BigInt64Array", "BigUint64Array",
  "WeakMap", "WeakSet", "WeakRef", "FinalizationRegistry",
  "Object", "Proxy", "Reflect", "Atomics", "JSON", "Intl",
  "Intl.Collator", "Intl.DateTimeFormat", "Intl.DisplayNames", "Intl.ListFormat", "Intl.Locale",
  "Intl.NumberFormat", "Intl.PluralRules", "Intl.RelativeTimeFormat", "Intl.Segmenter",
];

/** The built-ins of which only their constructor's statics are measured. */
const STATICS_ONLY = new Set(["Object"]);

/** The global functions measured, `globalThis.parseInt`. */
export const GLOBALS = ["parseInt", "parseFloat", "isNaN", "isFinite", "encodeURI", "encodeURIComponent", "decodeURI", "decodeURIComponent"];

/** A Rust type whose JS class isn't its name. */
const CLASS_OF: Record<string, string> = { JsError: "Error" };

/** A Rust type whose value JS makes by a static of its own: the
 * registry's symbol, `js::SymbolFor(key)`, `Symbol.for(key)` (ADR 0634). */
const MADE_BY: Record<string, string> = { SymbolFor: "Symbol.static:for" };

export type Coverage = { all: string[]; covered: string[] };

const libs = new URL("../node_modules/typescript-es/lib/", import.meta.url).pathname;
const src = new URL("./src/", import.meta.url).pathname;

/** The one library-level selection for member and argument coverage. */
const libraryFiles = () => readdirSync(libs)
  .filter((f) => /^lib\.es(5|20(1[5-9]|2[0-4]))(\.[a-z.]+)?\.d\.ts$/.test(f) && !f.includes(".full."))
  .map((f) => join(libs, f));

/** Native Rust callers derived from the pinned library's argument unions.
 * Unknown future members fail here, rather than leaving narrower bindings green.
 * Compile these callers: deleting a trait member must lose coverage too. */
export async function intlArgumentCases(): Promise<{ source: string; cases: number; sites: string[] }> {
  const files = libraryFiles();
  const session = await open(files);
  const aliases = new Map<string, any>();
  const calls = new Map<string, { params: any[]; invoke: (args: string[]) => string }[]>();
  const formatters = new Set(SCOPE.filter(name => name.startsWith("Intl.") && name !== "Intl.Locale").map(name => name.slice(5)));
  const add = (name: string, params: any[], invoke: (args: string[]) => string) => {
    calls.set(name, [...(calls.get(name) ?? []), {params, invoke}]);
  };
  const walk = (declarations: any[], namespace = "") => {
    for (const d of declarations) {
      if (d.kind === "namespace") walk(d.declarations, namespace + d.name + ".");
      else if (d.kind === "type") aliases.set(namespace + d.name, d.type);
      else if (namespace === "Intl." && d.kind === "function" && d.name === "getCanonicalLocales") {
        add("Intl.getCanonicalLocales", d.params, args => "intl::get_canonical_locales(" + args.join(", ") + ")");
      } else if (namespace === "Intl." && (d.kind === "interface" || (d.kind === "const" && d.type.kind === "object"))) {
        const name = d.name.replace(/Constructor$/, "");
        for (const m of checkedMembers(d.members ?? d.type.members)) {
          if (formatters.has(name) && (m.kind === "construct" || (m.kind === "method" && m.name === "supportedLocalesOf"))) {
            const method = m.kind === "construct" ? "new" : "supported_locales_of";
            add("Intl." + name + "." + method, m.params, args => "intl::" + snake(name) + "::" + method + "(" + args.join(", ") + ")");
          } else if (d.name === "DateTimeFormat" && m.kind === "method" && /^(format|formatToParts|formatRange|formatRangeToParts)$/.test(m.name)) {
            add("Intl.DateTimeFormat." + m.name, m.params, args => "formatter." + snake(m.name) + "(" + args.join(", ") + ")");
          }
        }
      } else if (d.kind === "interface" && (d.name === "Date" || d.name === "String")) {
        for (const m of checkedMembers(d.members)) if (m.kind === "method" && /^toLocale(String|DateString|TimeString|LowerCase|UpperCase)$/.test(m.name) && m.params.length) {
          add(d.name + "." + m.name, m.params, args => d.name === "Date"
            ? "date." + snake(m.name) + "(" + args.join(", ") + ")"
            : "js::string::" + snake(m.name) + '("TEXT", ' + args.join(", ") + ")");
        }
      }
    }
  };
  try { for (const file of files) walk((await session.read(file)).declarations); }
  finally { await session.close(); }
  const values = (type: any): (string | undefined)[] => {
    if (type.kind === "union") return [...new Set<string | undefined>(type.types.flatMap(values))];
    if (type.kind === "reference") {
      const alias = aliases.get(type.name) ?? aliases.get("Intl." + type.name);
      if (alias) return values(alias);
      if (type.name === "Locale") return ['intl::locale::new("de-DE", &intl::LocaleOptions::default()).unwrap()'];
      if (type.name === "Date") return ['js::date::new(Some(&0.0))'];
    }
    if (type.kind === "keyword") {
      if (type.keyword === "string") return ['"en-US"', 'String::from("en-US")', '&String::from("en-US")'];
      if (type.keyword === "number") return ["0.0"];
      if (type.keyword === "bigint") return ["0_i64", "0_u64"];
      if (type.keyword === "undefined") return [undefined];
    }
    if (type.kind === "array") return values(type.element).flatMap(value => {
      if (value === undefined) throw new Error("an Intl array element admits undefined; extend its Rust union");
      return [`&[${value}]`, `vec![${value}]`];
    });
    throw new Error("uncovered Intl argument member: " + JSON.stringify(type));
  };
  const source = ['use js::intl;'];
  const options = new Map<string, string>();
  const emitted = new Set<string>();
  for (const overloads of calls.values()) for (const {params, invoke} of overloads) {
    const variants = params.map(p => {
      if (p.name === "options") {
        const name = p.type.kind === "reference" && p.type.name !== "Pick"
          ? p.type.name.replace(/^Intl\./, "") : "SupportedLocalesOptions";
        const value = "options_" + snake(name);
        options.set(name, value);
        return [value, ...(p.optional ? [undefined] : [])];
      }
      const result = values(p.type);
      return p.optional && !result.includes(undefined) ? [...result, undefined] : result;
    });
    const argument = (value: string | undefined, index: number) => {
      if (value === undefined) return "None";
      if (params[index].name === "options") return params[index].optional ? `Some(${value})` : value;
      if (params[index].optional || variants[index].includes(undefined) || overloads.some(o => o.params[index]?.optional || (o.params[index] && o.params[index].name !== "options" && values(o.params[index].type).includes(undefined)))) return `Some(&${value})`;
      return value;
    };
    for (let i = 0; i < params.length; i++) for (const value of variants[i]) {
      const args = params.map((_, j) => argument(i === j ? value : variants[j][0], j));
      emitted.add("    let _ = " + invoke(args) + ";");
    }
  }
  source.push("pub fn coverage(formatter: &intl::DateTimeFormat, date: &js::Date, " + [...options].map(([name, value]) => `${value}: &intl::${name}`).join(", ") + ") {", ...emitted, "}");
  return {source: source.join("\n") + "\n", cases: emitted.size, sites: [...calls.keys()].sort()};
}

/** Each built-in's members: instance ones by name, `static:` ones, and
 * `constructor` where `new` makes one. */
async function typescript(): Promise<Map<string, Set<string>>> {
  const files = libraryFiles();
  const session = await open(files);
  const interfaces = new Map<string, Set<string>>();
  const heritage = new Map<string, string[]>();
  const constructSignatures = new Set<string>();
  const constructors = new Map<string, { statics: Set<string>; ctor: boolean } | string>();
  const functions = new Set<string>();
  const named = (m: any) => (m.kind === "method" || m.kind === "property") && /^[A-Za-z_$][\w$]*$/.test(m.name);
  const walk = (declarations: any[], prefix: string) => {
    for (const d of declarations) {
      const name = prefix + d.name;
      if (d.kind === "interface") {
        const set = interfaces.get(name) ?? new Set<string>();
        for (const m of checkedMembers(d.members)) if (named(m)) set.add(m.name);
        interfaces.set(name, set);
        if (checkedMembers(d.members).some((m: any) => m.kind === "construct")) constructSignatures.add(name);
        // What it extends, by its name in the namespace it's in.
        const parents = d.extends.filter((e: any) => e.kind === "reference").map((e: any) => prefix + e.name);
        heritage.set(name, [...(heritage.get(name) ?? []), ...parents]);
      } else if (d.kind === "const") {
        if (d.type.kind === "reference") constructors.set(name, prefix + d.type.name);
        else if (d.type.kind === "object") {
          const statics = new Set<string>(checkedMembers(d.type.members).filter(named).map((m: any) => m.name));
          const ctor = checkedMembers(d.type.members).some((m: any) => m.kind === "construct");
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
  // What TypeScript marks `@deprecated`, "a legacy feature for browser compatibility", `RegExp.$1`,
  // by the interface it's in: not counted.
  const deprecated = new Set<string>();
  for (const f of files) {
    const text = readFileSync(f, "utf8");
    const comment = /\/\*\*(?:(?!\*\/)[\s\S])*?@deprecated(?:(?!\*\/)[\s\S])*\*\/\s*(?:readonly\s+)?(?:"([^"]+)"|([A-Za-z_$][\w$]*))/g;
    for (const m of text.matchAll(comment)) {
      const owner = [...text.slice(0, m.index).matchAll(/interface (\w+)/g)].at(-1)?.[1];
      if (owner) deprecated.add(`${owner}.${m[1] ?? m[2]}`);
    }
  }
  await session.close();
  // An interface's members are its own and those of what it extends:
  // `Intl.Locale`'s `region` is its `LocaleOptions`'.
  const inherited = (name: string, seen = new Set<string>()): string[] =>
    (heritage.get(name) ?? []).flatMap((p) => (seen.has(p) ? [] : (seen.add(p), [...(interfaces.get(p) ?? []), ...inherited(p, seen)])));
  for (const name of interfaces.keys()) for (const m of inherited(name)) interfaces.get(name)!.add(m);
  const out = new Map<string, Set<string>>();
  for (const name of SCOPE) {
    const members = new Set<string>();
    const ctor = constructors.get(name);
    const own = ctor === name;
    const local = name.split(".").pop()!;
    if (!own) for (const m of interfaces.get(name) ?? []) if (!deprecated.has(`${local}.${m}`)) members.add(m);
    if (typeof ctor === "string") {
      const of = ctor.split(".").pop()!;
      for (const m of interfaces.get(ctor) ?? []) if (m !== "prototype" && !deprecated.has(`${of}.${m}`)) members.add(`static:${m}`);
      if (!own && constructSignatures.has(ctor)) members.add("constructor");
    } else if (ctor) {
      for (const m of ctor.statics) if (m !== "prototype") members.add(`static:${m}`);
      if (ctor.ctor) members.add("constructor");
    }
    // A namespace's functions are its statics: `Reflect.apply`, `Intl.getCanonicalLocales`.
    for (const f of functions) if (f.startsWith(`${name}.`) && !f.slice(name.length + 1).includes(".")) members.add(`static:${f.slice(name.length + 1)}`);
    out.set(name, STATICS_ONLY.has(name) ? new Set([...members].filter((m) => m.startsWith("static:"))) : members);
  }
  out.set("globalThis", new Set(GLOBALS.filter((g) => functions.has(g))));
  return out;
}

/** What src binds: each function by its link name and receiver. `new X`
 * is X's constructor; `X.y`, `Intl.X.y`, X's static `y`; a method of an
 * `impl X`, or one taking `this: &X`, X's member, `get y` and `set y` its
 * `y`; any other name a global function's. A constant of a class's module,
 * `int8_array::BYTES_PER_ELEMENT`, is its static. */
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
      if (MADE_BY[s]) bound.add(MADE_BY[s]);
    });
  }
  for (const f of files) {
    let depth = 0;
    const impls: { type: string; depth: number }[] = [];
    const modules: { name: string; depth: number }[] = [];
    const snake = (s: string) => s.replace(/([a-z0-9])([A-Z])/g, "$1_$2").toLowerCase();
    let link: string | undefined;
    for (const l of readFileSync(join(src, f), "utf8").split("\n")) {
      const impl = l.match(/^\s*impl(?:<[^>]*>)? (\w+)(?:<[^>]*>)? \{/)?.[1];
      const module = l.match(/^\s*pub mod (\w+) \{/)?.[1];
      const constant = l.match(/^\s*pub const (\w+):/)?.[1];
      if (constant && modules.length) {
        const owner = [...classes.keys()].find((s) => snake(s) === modules.at(-1)!.name);
        if (owner) bound.add(`${classes.get(owner)}.static:${constant}`);
      }
      link = l.match(/link_name = "([^"]+)"/)?.[1] ?? link;
      // A function, or an extern static, `Symbol.iterator`, of no receiver.
      // A keyword's, `r#for`, by its name, `for` (ADR 0376).
      const fn = l.match(/^\s*pub (?:safe )?fn (?:r#)?(\w+)(?:<[^(]*>)?\((.*)/) ?? l.match(/^\s*pub safe static (?:r#)?(\w+)()/);
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
          while (modules.length && modules.at(-1)!.depth > depth) modules.pop();
        }
      }
      if (impl) impls.push({ type: impl, depth });
      if (module) modules.push({ name: module, depth });
    }
  }
  // A member of what a type derefs to is one of its own: a typed array's
  // `buffer`, its `ArrayBufferView`'s (ADR 0741).
  for (const f of files) {
    const text = readFileSync(join(src, f), "utf8");
    for (const [, type, target] of text.matchAll(/^impl(?:<[^>]*>)? (?:core::ops::)?Deref for (\w+)(?:<[^>]*>)? \{\n\s+type Target = (\w+)/gm)) {
      const [of, to] = [classes.get(type) ?? type, classes.get(target) ?? target];
      for (const b of [...bound]) if (b.startsWith(`${to}.`) && !b.includes(".static:") && !b.endsWith(".constructor")) bound.add(`${of}${b.slice(to.length)}`);
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
