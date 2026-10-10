// How much of Node.js's API the node crate binds: each export of each
// module @types/node declares, `declare module "fs"`, and `process`'s and
// `console`'s members, as `import process from "process"` is the global,
// `+` where the crate binds it, a value by its `link_name`, `fs#readFileSync` or
// `process.cwd`, a type or a class by an item of its name in the module's
// file, http.rs's of `http`, or in lib.rs, the crate's own; `-` where it
// doesn't. Each `node:fs` is its `fs`.
// See docs/decisions/0351-node-coverage.md.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

import { open } from "@rust-js/typescript";

const types = join(import.meta.dir, "../node_modules/@types/node");
const src = join(import.meta.dir, "src");

type Kind = "value" | "type";
export type Module = { name: string; exports: { name: string; kind: Kind; bound: boolean }[] };

// A name both a class and an interface, as TypeScript merges them, is a
// value, whichever file says which first.
class Exports extends Map<string, Kind> {
  override set(name: string, kind: Kind): this {
    return super.set(name, this.get(name) === "value" ? "value" : kind);
  }
}

const kinds: Record<string, Kind> = { interface: "type", type: "type", function: "value", const: "value", enum: "value", namespace: "value" };

const files = (dir: string): string[] =>
  readdirSync(dir).sort().flatMap((f) => {
    const path = join(dir, f);
    if (statSync(path).isDirectory()) return f === "compatibility" || f === "ts5.6" || f === "ts5.7" ? [] : files(path);
    return f.endsWith(".d.ts") ? [path] : [];
  });

// The model leaves a class as text, after its doc comment.
const classOf = (text: string): string | undefined =>
  text.replace(/^(\s*\/\*[\s\S]*?\*\/|\s*\/\/[^\n]*)*\s*/, "").match(/^(?:export )?(?:declare )?(?:abstract )?class (\w+)/)?.[1];

/** Each module's exports, by its name, and what `process` has. */
async function declared(): Promise<Map<string, Map<string, Kind>>> {
  const modules = new Map<string, Map<string, Kind>>();
  const reexports = new Map<string, string[]>();
  for (const file of files(types)) {
    const ts = await open([file]);
    const { declarations } = await ts.read(file);
    await ts.close();
    for (const module of declarations as any[]) {
      // `declare global` and `namespace NodeJS` are the globals', not a module.
      if (module.kind !== "namespace" || module.name.startsWith("node:") || ["global", "NodeJS"].includes(module.name)) continue;
      const own = modules.get(module.name) ?? new Exports();
      // A `declare module`'s declarations are each exported, as TypeScript has
      // it, unless it says `export {}`: then those it marks.
      const marked = module.declarations.some((d: any) => d.kind === "other" && /^export \{\s*\};?$/.test(d.text.trim()));
      const local = new Exports();
      for (const d of module.declarations) {
        const kind = kinds[d.kind];
        if (kind && d.name !== "global") {
          local.set(d.name, kind);
          if (d.exported || !marked) own.set(d.name, kind);
        }
        if (d.kind !== "other") continue;
        const text: string = d.text;
        const cls = classOf(text);
        if (cls) {
          local.set(cls, "value");
          if (!marked || /^(\s*\/\*[\s\S]*?\*\/|\s*\/\/[^\n]*)*\s*export /.test(text)) own.set(cls, "value");
        }
        let m: RegExpMatchArray | null;
        if ((m = text.match(/^export \* from ['"](?:node:)?(.+)['"]/))) {
          reexports.set(module.name, [...(reexports.get(module.name) ?? []), m[1]]);
        } else if ((m = text.match(/^export (type )?\{([^}]*)\}\s*;?\s*$/))) {
          for (const part of m[2].split(",").map((s) => s.trim()).filter(Boolean)) {
            const [name, as] = part.replace(/^type /, "").split(/\s+as\s+/);
            own.set(as ?? name, m[1] || part.startsWith("type ") ? "type" : (local.get(name) ?? "value"));
          }
        }
      }
      // `export = path`: the module is that value, its exports its members,
      // a const's interface's, a class's statics, and its namespace's.
      const assigned = module.declarations
        .map((d: any) => (d.kind === "other" ? d.text.match(/^export = (\w+);/)?.[1] : undefined))
        .find(Boolean);
      if (assigned && !["process", "console"].includes(assigned)) {
        own.clear();
        const interfaces = new Map<string, any>();
        const index = (list: any[]) => {
          for (const d of list) {
            if (d.kind === "interface") interfaces.set(d.name, d);
            if (d.declarations) index(d.declarations);
          }
        };
        index(module.declarations);
        for (const d of module.declarations) {
          if (d.kind === "const" && d.name === assigned && d.type?.kind === "reference") {
            const shape = interfaces.get(d.type.name.split(".").pop());
            for (const m of shape?.members ?? []) if ((m.kind === "method" || m.kind === "property") && /^[A-Za-z_$][\w$]*$/.test(m.name)) own.set(m.name, "value");
          }
          if (d.kind === "namespace" && d.name === assigned) {
            for (const inner of d.declarations) {
              if (kinds[inner.kind]) own.set(inner.name, kinds[inner.kind]);
              const cls = inner.kind === "other" ? classOf(inner.text) : undefined;
              if (cls) own.set(cls, "value");
            }
          }
          if (d.kind === "other" && classOf(d.text) === assigned) {
            for (const m of d.text.matchAll(/^\s+static (?:readonly )?(\w+)/gm)) own.set(m[1], "value");
          }
        }
      }
      modules.set(module.name, own);
    }
  }
  for (const [name, froms] of reexports) {
    for (const from of froms) for (const [e, kind] of modules.get(from) ?? []) modules.get(name)!.set(e, kind);
  }
  // `process` and `console` are `export =` the global: their members are
  // what its interface, `Process` or `Console`, has.
  for (const [name, shape] of [["process", "Process"], ["console", "Console"]]) {
    const file = join(types, `${name}.d.ts`);
    const ts = await open([file]);
    const { declarations } = await ts.read(file);
    await ts.close();
    const members = new Exports();
    const find = (list: any[]) => {
      for (const d of list) {
        if (d.kind === "interface" && d.name === shape) {
          for (const m of d.members) if ((m.kind === "method" || m.kind === "property") && /^[A-Za-z_$][\w$]*$/.test(m.name)) members.set(m.name, "value");
        }
        if (d.declarations) find(d.declarations);
      }
    };
    find(declarations);
    modules.set(name, members);
  }
  return modules;
}

// What the crate binds: each value's `link_name`, `fs#readFileSync` of a
// module's and `process.cwd` of the global's, and each item's name, by
// the module of its file: `http#Server` of http.rs's, lib.rs's of each.
function bindings(): { links: Set<string>; items: Set<string> } {
  const files = readdirSync(src, { recursive: true })
    .map(String)
    .filter((f) => f.endsWith(".rs"))
    .sort();
  const sources = files.map((f) => readFileSync(join(src, f), "utf8")).join("\n");
  // An item of its own, or one it re-exports, `pub use webapi::{URL, ..}`,
  // as `url`'s `URL` is the global webapi binds.
  const items = (f: string) => {
    const text = readFileSync(join(src, f), "utf8");
    return [
      ...[...text.matchAll(/pub (?:struct|enum|type|trait) (\w+)/g)].map((m) => m[1]),
      ...[...text.matchAll(/pub use [\w:]+::\{([^}]*)\}/g)].flatMap((m) => m[1].split(",").map((n) => n.trim()).filter(Boolean)),
      ...[...text.matchAll(/pub use [\w:]+::(\w+);/g)].map((m) => m[1]),
    ];
  };
  return {
    links: new Set([
      ...[...sources.matchAll(/link_name = "(?:new )?([\w/]+)#(\w+)/g)].map((m) => `${m[1]}#${m[2]}`),
      ...[...sources.matchAll(/link_name = "process\.(\w+)"/g)].map((m) => `process#${m[1]}`),
    ]),
    items: new Set(files.flatMap((f) => (f === "lib.rs" ? items(f).map((i) => `*#${i}`) : items(f).map((i) => `${f.slice(0, -3)}#${i}`)))),
  };
}

export async function measure(): Promise<Module[]> {
  const { links, items } = bindings();
  return [...(await declared())]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([name, exported]) => ({
      name,
      exports: [...exported]
        .sort(([a], [b]) => a.localeCompare(b))
        .map(([e, kind]) => ({ name: e, kind, bound: links.has(`${name}#${e}`) || items.has(`${name}#${e}`) || items.has(`*#${e}`) })),
    }));
}

/** The baseline's text: each module's count, then each export, `+` bound. */
export function render(modules: Module[]): string {
  const all = modules.flatMap((m) => m.exports);
  const bound = all.filter((e) => e.bound).length;
  return [
    `# The node crate against @types/node: bun test test/node-coverage.test.ts`,
    `# exports: ${bound} of ${all.length} (${((100 * bound) / all.length).toFixed(1)}%)`,
    ...modules.flatMap((m) => [
      `# ${m.name} ${m.exports.filter((e) => e.bound).length} of ${m.exports.length}`,
      ...m.exports.map((e) => `${e.bound ? "+" : "-"} ${e.kind === "type" ? "type " : ""}${m.name}#${e.name}`),
    ]),
    "",
  ].join("\n");
}
