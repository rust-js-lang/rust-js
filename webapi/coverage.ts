// How much of TypeScript's DOM the webapi crate binds: each class
// `lib.dom.d.ts` declares (`@types/web`, which TypeScript generates it as),
// and each of its members, against the members src/lib.rs's MDN links name.
// See docs/research/web-platform-coverage.md.

import { readFileSync } from "node:fs";

import { open } from "@rust-js/typescript";

export type Members = { members: Set<string>; statics: Set<string>; ctor: boolean; extends: string[] };

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
    if (!interfaces.has(name)) interfaces.set(name, { members: new Set(), statics: new Set(), ctor: false, extends: [] });
    return interfaces.get(name)!;
  };
  const classes = new Set<string>();
  for (const d of declarations as any[]) {
    if (d.kind === "interface") {
      const m = of(d.name);
      for (const e of d.extends) if (e.kind === "reference") m.extends.push(e.name);
      for (const member of d.members) if (member.kind === "method" || member.kind === "property") m.members.add(member.name);
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
    out.set(name, { ...m, members });
  }
  return out;
}

/** What src/lib.rs binds, by the MDN link on each function: `get x` and
 * `set x` are `x`, `new X` the constructor, `X.y` the static `y`. */
function webapi(): { bound: Set<string>; types: Set<string> } {
  const text = readFileSync(lib, "utf8");
  const lines = text.split("\n");
  const bound = new Set<string>();
  for (let i = 0; i < lines.length; i++) {
    const mdn = lines[i].match(/developer\.mozilla\.org\/docs\/Web\/API\/(\w+)\/(\w+)/);
    if (!mdn) continue;
    let link: string | undefined;
    for (let j = i + 1; j < Math.min(i + 4, lines.length) && !link; j++) {
      link = lines[j].match(/link_name = "([^"]+)"/)?.[1] ?? lines[j].match(/pub (?:safe )?fn (\w+)/)?.[1];
    }
    const member =
      link?.startsWith("new ") ? "constructor"
      : link && /^[A-Z]\w*\.\w+$/.test(link) ? `static:${link.split(".")[1]}`
      : mdn[2];
    bound.add(`${mdn[1]}.${member}`);
  }
  const types = new Set([...text.matchAll(/^pub struct (\w+)/gm)].map((m) => m[1]));
  return { bound, types };
}

export async function measure(): Promise<Coverage> {
  const ts = await typescript();
  const { bound, types } = webapi();
  const all: string[] = [];
  for (const [name, m] of ts) {
    for (const x of m.members) all.push(`${name}.${x}`);
    for (const x of m.statics) all.push(`${name}.static:${x}`);
    if (m.ctor) all.push(`${name}.constructor`);
  }
  all.sort();
  const classes = [...ts.keys()].sort();
  return { all, covered: all.filter((x) => bound.has(x)), classes, typed: classes.filter((c) => types.has(c)) };
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
