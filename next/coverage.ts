// How much of Next.js's public API the next crate binds: each export of
// each public module, from Next.js's own .d.ts, `+` where the crate binds
// it, a value by its `link_name`, a type by an item of its name in the
// crate; `-` where it doesn't.
// See docs/decisions/0346-next-coverage.md.

import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

import { open } from "@rust-js/typescript";

const next = join(import.meta.dir, "../examples/next/node_modules/next");
const crate = join(import.meta.dir, "src");

// The modules an app imports, as Next.js documents them, each its file. The
// rest are build tooling (babel, jest), Next.js internals (client,
// constants, types), or typed only once an app is built (root-params).
const MODULES: Record<string, string> = {
  next: "index.d.ts",
  "next/app": "app.d.ts",
  "next/cache": "cache.d.ts",
  "next/document": "document.d.ts",
  "next/dynamic": "dynamic.d.ts",
  "next/error": "error.d.ts",
  "next/font/google": "font/google/index.d.ts",
  "next/font/local": "font/local/index.d.ts",
  "next/form": "form.d.ts",
  "next/head": "head.d.ts",
  "next/headers": "headers.d.ts",
  "next/image": "image.d.ts",
  "next/legacy/image": "legacy/image.d.ts",
  "next/link": "link.d.ts",
  "next/navigation": "navigation.d.ts",
  "next/offline": "offline.d.ts",
  "next/og": "og.d.ts",
  "next/router": "router.d.ts",
  "next/script": "script.d.ts",
  "next/server": "server.d.ts",
  "next/web-vitals": "web-vitals.d.ts",
};

type Kind = "value" | "type";
export type Member = { name: string; bound: boolean };
export type Module = { name: string; exports: { name: string; kind: Kind; bound: boolean; members: Member[] }[] };

// Each type's and class's own members, by its file and name, `path#Route`,
// as two files may each have a `Route` of their own: an interface's, and
// what it extends of its file's; an object type's, of an intersection's or
// a union's parts too, `Omit`'s, `Pick`'s and `Partial`'s of one; a class's
// properties, methods and statics. What a type has of React's, an element's
// attributes, is React's, counted by its crate.
const shapes = new Map<string, string[]>();

function shapesOf(path: string, declarations: any[]) {
  const index = new Map<string, any>();
  const indexed = (list: any[]) => {
    for (const d of list) {
      if (d.name && !index.has(d.name)) index.set(d.name, d);
      const cls = d.kind === "other" ? classText(d.text) : undefined;
      if (cls && !index.has(cls.name)) index.set(cls.name, { kind: "class", ...cls });
      if (d.declarations) indexed(d.declarations);
    }
  };
  indexed(declarations);
  const named = (m: any) => (m.kind === "property" || m.kind === "method") && typeof m.name === "string";
  const of = (d: any, seen: Set<any>): string[] => {
    if (!d || seen.has(d)) return [];
    seen.add(d);
    if (d.kind === "interface") {
      const inherited = (d.extends ?? []).flatMap((e: any) => (e.kind === "reference" ? of(index.get(e.name), seen) : []));
      return [...d.members.filter(named).map((m: any) => m.name), ...inherited];
    }
    if (d.kind === "type") return typed(d.type, seen);
    if (d.kind === "class") return d.members;
    return [];
  };
  const typed = (t: any, seen: Set<any>): string[] => {
    if (!t) return [];
    switch (t.kind) {
      case "object":
        return t.members.filter(named).map((m: any) => m.name);
      case "intersection":
      case "union":
        return t.types.flatMap((part: any) => typed(part, seen));
      case "reference": {
        const keys = (k: any): string[] => (k?.kind === "literal" ? [k.value] : k?.kind === "union" ? k.types.flatMap(keys) : []);
        const [first, second] = t.args ?? [];
        if (["Partial", "Required", "Readonly", "NonNullable"].includes(t.name)) return typed(first, seen);
        if (t.name === "Omit") return typed(first, seen).filter((m) => !keys(second).includes(m));
        if (t.name === "Pick") return typed(first, seen).filter((m) => keys(second).includes(m));
        return of(index.get(t.name.split(".").pop()), seen);
      }
      default:
        return [];
    }
  };
  for (const [name, d] of index) {
    if (shapes.has(`${path}#${name}`) || !["interface", "type", "class"].includes(d.kind)) continue;
    // Not one named private, `_bfl`, as Next.js names its internals.
    const members = [...new Set(of(d, new Set()))].filter((m) => !m.startsWith("_"));
    if (members.length) shapes.set(`${path}#${name}`, members);
  }
}

/** A class's name and members, of its text: each property, method, getter
 * and static, not a private one nor its constructor. */
function classText(text: string): { name: string; members: string[] } | undefined {
  const body = text.replace(/\/\*[\s\S]*?\*\//g, "").replace(/\/\/[^\n]*/g, "");
  const name = body.match(/^\s*(?:export )?(?:default )?(?:declare )?(?:abstract )?class (\w+)/)?.[1];
  if (!name) return undefined;
  // A React component's instance members are its own, `render`: what it
  // offers is its statics, `Document.getInitialProps`.
  const component = /class \w+(?:<[^>]*>)? extends (?:React\.)?(?:Pure)?Component\b/.test(body);
  const members = [...body.matchAll(/^ {4}((?:(?:public|static|readonly|get|set|declare|abstract|override)\s+)*)([A-Za-z_$][\w$]*)\??\s*[(:<]/gm)]
    .filter((m) => !/private|protected/.test(m[1]) && m[2] !== "constructor" && (!component || /static/.test(m[1])))
    .map((m) => m[2]);
  return { name, members: [...new Set(members)] };
}

function file(from: string, specifier: string): string | undefined {
  const base = specifier.startsWith("next/") ? join(next, specifier.slice("next/".length)) : resolve(dirname(from), specifier);
  for (const candidate of [`${base}.d.ts`, join(base, "index.d.ts"), base.replace(/\.js$/, ".d.ts")]) {
    if (existsSync(candidate) && statSync(candidate).isFile()) return candidate;
  }
  return undefined;
}

// An export: a value or a type, and where it's declared, `path#name`, its
// shape's key.
type Entry = { kind: Kind; origin?: string };

const read = new Map<string, Map<string, Entry>>();

async function exportsOf(path: string, seen: Set<string> = new Set()): Promise<Map<string, Entry>> {
  if (read.has(path)) return read.get(path)!;
  if (seen.has(path)) return new Map();
  seen.add(path);
  const exported = new Map<string, Entry>();
  const local = new Map<string, Entry>();
  const declare = (name: string, kind: Kind) => local.set(name, { kind, origin: `${path}#${name}` });
  // A session reads the files it's opened with: one each.
  const ts = await open([path]);
  const { declarations } = await ts.read(path);
  await ts.close();
  shapesOf(path, declarations);
  // What the file imports, by its name here: the module it's of, and its
  // name there, a default import's `default`, which the model leaves as text.
  const imports = new Map<string, [string, string]>();
  for (const d of declarations as any[]) {
    if (d.kind === "import") for (const n of d.names) imports.set(n, [d.from, n]);
    const m = d.kind === "other" && d.text.match(/^import (?:type )?(\w+)\b[^;]*? from ['"](.+)['"]/);
    if (m) imports.set(m[1], [m[2], "default"]);
  }
  // A name of the file's own, or one it imports, as that module exports it.
  const own = async (name: string): Promise<Entry | undefined> => {
    if (local.has(name)) return local.get(name);
    const [from, there] = imports.get(name) ?? [];
    const target = from ? file(path, from) : undefined;
    return target ? (await exportsOf(target, seen)).get(there!) : undefined;
  };
  const kindOf = (d: { kind: string }): Kind => (["interface", "type"].includes(d.kind) ? "type" : "value");
  const reexport = async (specifier: string, names: [string, string][], typeOnly: boolean) => {
    const target = file(path, specifier);
    const there = target ? await exportsOf(target, seen) : new Map<string, Entry>();
    for (const [name, as] of names) {
      const entry = there.get(name);
      exported.set(as, { kind: typeOnly ? "type" : (entry?.kind ?? "value"), origin: entry?.origin });
    }
  };
  for (const d of declarations) {
    if ("name" in d && typeof d.name === "string" && ["interface", "type", "const", "function", "class", "enum"].includes(d.kind)) {
      declare(d.name, kindOf(d));
      if ("exported" in d && d.exported) exported.set(d.name, local.get(d.name)!);
    }
    if (d.kind === "export-from") await reexport(d.from, d.names, false);
    if (d.kind === "export-default") exported.set("default", (await own(d.name)) ?? { kind: "value" });
    if (d.kind === "other") {
      // The model leaves a class as text, after its doc comment.
      const text: string = d.text.replace(/^(\s*\/\*[\s\S]*?\*\/)*\s*/, "");
      let m: RegExpMatchArray | null;
      if ((m = text.match(/^export (?:declare )?(?:abstract )?(class|namespace|enum|const|function) (\w+)/))) {
        declare(m[2], "value");
        exported.set(m[2], local.get(m[2])!);
      } else if ((m = text.match(/^(?:declare )?(?:abstract )?(?:class|namespace|enum|const|function) (\w+)/))) {
        declare(m[1], "value");
      } else if ((m = text.match(/^export \* from ['"](.+)['"]/))) {
        const target = file(path, m[1]);
        if (target) for (const [name, entry] of await exportsOf(target, seen)) if (name !== "default") exported.set(name, entry);
      } else if ((m = text.match(/^export (type )?\{([\s\S]*?)\} from ['"](.+)['"]/))) {
        const names = m[2].split(",").map((s) => s.trim()).filter(Boolean).map((s) => {
          const [name, as] = s.replace(/^type /, "").split(/\s+as\s+/);
          return [name, as ?? name] as [string, string];
        });
        await reexport(m[3], names, Boolean(m[1]));
      } else if ((m = text.match(/^export (type )?\{([\s\S]*?)\}\s*;?\s*$/))) {
        // A name of the module's own, or one it imports, exported as it is.
        for (const s of m[2].split(",").map((s) => s.trim()).filter(Boolean)) {
          const [name, as] = s.replace(/^type /, "").split(/\s+as\s+/);
          const entry = await own(name.replace(/^type /, ""));
          exported.set(as ?? name, { kind: m[1] || s.startsWith("type ") ? "type" : (entry?.kind ?? "value"), origin: entry?.origin });
        }
      } else if ((m = text.match(/^export default (\w+)/))) {
        exported.set("default", (await own(m[1])) ?? { kind: "value" });
      }
    }
  }
  // `export default function dynamic` is the default, not a `dynamic`: the
  // model has it as any exported function.
  for (const m of readFileSync(path, "utf8").matchAll(/^export default (?:declare )?(?:async )?(?:function|class) (\w+)/gm)) {
    exported.set("default", exported.get(m[1]) ?? { kind: "value", origin: `${path}#${m[1]}` });
    exported.delete(m[1]);
  }
  read.set(path, exported);
  return exported;
}

// What the crate binds: each value's `link_name`, a constructor's, `new
// next/server#NextRequest`, and a static's, `next/server#NextResponse.json`,
// its class's; and each item's name: a
// type's, or a value's an item stands for, a class's struct, or the enum of
// a const object's strings, `RedirectType`.
function bindings(): { links: Set<string>; items: Set<string> } {
  const rust = (dir: string): string[] =>
    readdirSync(dir).flatMap((f) => (statSync(join(dir, f)).isDirectory() ? rust(join(dir, f)) : f.endsWith(".rs") ? [join(dir, f)] : []));
  const sources = rust(crate).map((f) => readFileSync(f, "utf8")).join("\n");
  return {
    links: new Set([...sources.matchAll(/link_name = "(?:new )?(next[^"#]*)#(\w+)/g)].map((m) => `${m[1]}#${m[2]}`)),
    items: new Set([...sources.matchAll(/pub (?:struct|enum|type|trait) (\w+)|pub use [\w:]+ as (\w+);|pub mod ([A-Z]\w*)/g)].map((m) => m[1] ?? m[2] ?? m[3])),
  };
}

// Each struct's members, by its name: its fields' JS names, not a flattened
// one's, which is another's; the methods of its `impl`s, by their link names,
// `get cookies` a `cookies`; a static of it, `next/server#NextResponse.json`;
// and an enum's, its variants' payloads'.
// A type alias's, `pub type ProxyConfig = MiddlewareConfig`, are its type's.
function rustMembers(): Map<string, Set<string>> {
  const rust = (dir: string): string[] =>
    readdirSync(dir).flatMap((f) => (statSync(join(dir, f)).isDirectory() ? rust(join(dir, f)) : f.endsWith(".rs") ? [join(dir, f)] : []));
  const members = new Map<string, Set<string>>();
  const add = (owner: string, member: string) => members.set(owner, (members.get(owner) ?? new Set()).add(member));
  const aliases: [string, string][] = [];
  const payloads: [string, string][] = [];
  // The block a `{` at `at` opens.
  const block = (text: string, at: number) => {
    let depth = 0;
    for (let i = at; i < text.length; i++) {
      if (text[i] === "{") depth++;
      else if (text[i] === "}" && --depth === 0) return text.slice(at + 1, i);
    }
    return "";
  };
  // A flattened field's are its struct's, React's attributes' among them.
  const flattened: [string, string][] = [];
  for (const file of [...rust(crate), ...rust(join(import.meta.dir, "../react/src"))]) {
    const text = readFileSync(file, "utf8");
    const react = file.includes("/react/src/");
    for (const m of text.matchAll(/pub struct (\w+)[^;{(]*\{/g)) {
      // React's own are only what a flattened field of this crate's holds.
      const owner = react ? `react::${m[1]}` : m[1];
      let name: string | undefined;
      let flatten = false;
      for (const line of block(text, m.index! + m[0].length - 1).split("\n")) {
        name = line.match(/rust_js::name = "([^"]+)"/)?.[1] ?? name;
        flatten ||= /rust_js::flatten/.test(line);
        const field = line.match(/^\s*pub (?:r#)?(\w+):\s*(?:&(?:'\w+ )?)?(?:[\w:]+::)?(\w*)/);
        if (field) {
          if (flatten) flattened.push([owner, `react::${field[2]}`]);
          else add(owner, name ?? field[1]);
          name = undefined;
          flatten = false;
        }
      }
    }
    if (react) continue;
    for (const m of text.matchAll(/\nimpl(?:<[^>]*>)? (\w+)(?:<[^>]*>)? \{/g)) {
      for (const link of block(text, m.index! + m[0].length - 1).matchAll(/link_name = "(?:get |set )?([\w$]+)"/g)) add(m[1], link[1]);
    }
    for (const m of text.matchAll(/link_name = "(?:new )?next[^"#]*#(\w+)\.(\w+)"/g)) add(m[1], m[2]);
    for (const m of text.matchAll(/pub type (\w+)(?:<[^>]*>)? = (?:[\w:]+::)?(\w+)/g)) aliases.push([m[1], m[2]]);
    // A setter of what a trait's types are, `set getInitialProps` of `this:
    // impl NextComponentType`, is the trait's member; and a trait's are its
    // supertrait's too.
    for (const m of text.matchAll(/link_name = "set (\w+)"\)\]\s*pub fn \w+(?:<[^(]*>)?\(this: impl (\w+)</g)) add(m[2], m[1]);
    for (const m of text.matchAll(/pub trait (\w+)(?:<[^>]*>)?: (\w+)</g)) aliases.push([m[1], m[2]]);
    // A type's `Deref` target's, as JS's subclass has its superclass's.
    for (const m of text.matchAll(/impl(?:<[^>]*>)? Deref for (\w+)(?:<[^>]*>)? \{\s*type Target = (\w+)/g)) aliases.push([m[1], m[2]]);
    // An untagged enum's, a union of objects, are its payloads'.
    for (const m of text.matchAll(/pub enum (\w+)[^{]*\{/g)) {
      for (const payload of block(text, m.index! + m[0].length - 1).matchAll(/^\s*\w+\((\w+)/gm)) payloads.push([m[1], payload[1]]);
    }
    for (const m of text.matchAll(/pub use [\w:]*?(\w+) as (\w+);/g)) aliases.push([m[2], m[1]]);
  }
  const merged = (owner: string, seen: Set<string>): void => {
    for (const [from, into] of flattened) {
      if (from !== owner || seen.has(into)) continue;
      seen.add(into);
      merged(into, seen);
      for (const member of members.get(into) ?? []) add(owner, member);
    }
  };
  for (const [owner] of flattened) if (!owner.startsWith("react::")) merged(owner, new Set());
  for (const [union, payload] of payloads) for (const member of members.get(payload) ?? []) add(union, member);
  for (const [alias, target] of aliases) for (const member of members.get(target) ?? []) add(alias, member);
  return members;
}

export async function measure(): Promise<Module[]> {
  const { links, items } = bindings();
  const owned = rustMembers();
  const modules: Module[] = [];
  for (const [name, path] of Object.entries(MODULES)) {
    const exports = [...(await exportsOf(join(next, path)))]
      .filter(([e]) => !e.startsWith("_"))
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([e, { kind, origin }]) => ({
        name: e,
        kind,
        bound: links.has(`${name}#${e}`) || items.has(e),
        // Of the type it is where it's declared, `ImageProps` of get-img-props'.
        members: (shapes.get(origin ?? "") ?? []).sort().map((m) => ({ name: m, bound: owned.get(e)?.has(m) ?? false })),
      }));
    modules.push({ name, exports });
  }
  return modules;
}

/** The baseline's text: each module's counts, then each export, `+` bound,
 * and its members, each a line of its own after it. */
export function render(modules: Module[]): string {
  const all = modules.flatMap((m) => m.exports);
  const members = all.flatMap((e) => e.members);
  const percent = (n: number, of: number) => `${n} of ${of} (${((100 * n) / of).toFixed(1)}%)`;
  return [
    `# The next crate against Next.js's public modules: bun test test/next-coverage.test.ts`,
    `# exports: ${percent(all.filter((e) => e.bound).length, all.length)}`,
    `# members: ${percent(members.filter((e) => e.bound).length, members.length)}`,
    ...modules.flatMap((m) => {
      const own = m.exports.flatMap((e) => e.members);
      return [
        `# ${m.name} ${m.exports.filter((e) => e.bound).length} of ${m.exports.length}, members ${own.filter((e) => e.bound).length} of ${own.length}`,
        ...m.exports.flatMap((e) => [
          `${e.bound ? "+" : "-"} ${e.kind === "type" ? "type " : ""}${m.name}#${e.name}`,
          ...e.members.map((member) => `${member.bound ? "+" : "-"} ${m.name}#${e.name}.${member.name}`),
        ]),
      ];
    }),
    "",
  ].join("\n");
}
