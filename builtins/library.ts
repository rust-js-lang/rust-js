// The js crate's builtins written from TypeScript's own library, the one the
// pinned engine selects, `lib: ["esnext"]` (#1903): each root's members,
// their types, optionality and inheritance as the library declares them,
// merged across the files TypeScript merges, bound by the forms
// `builtins/render.ts` writes, named and thrown as `builtins/policy.ts`
// says where the library doesn't (#1904). A form the generator doesn't
// bind is refused by its name and its file, never written as `any` or
// narrowed. Each group it writes is its spec's no longer: Intl.Locale first.

import { open } from "@rust-js/typescript";
import { snake } from "../scripts/rust-names";
import { attributesOf, NAMES, runs, THROWS } from "./policy";
import { argument, classed, close, line, method, options, page, record, union } from "./render";

/** A declaration and the library file it's in. */
export type Fragment = { decl: any; file: string };
/** Each declaration by its qualified name, `Intl.Locale`, its fragments. */
export type Library = Map<string, Fragment[]>;

/** The library's declarations, of `lib`'s files, the engine's selection. */
export async function readLibrary(lib = ["esnext"]): Promise<Library> {
  const session = await open([], { lib });
  const library: Library = new Map();
  const walk = (declarations: any[], prefix: string, file: string) => {
    for (const decl of declarations) {
      if (decl.kind === "namespace" && decl.name === "global" && prefix === "") walk(decl.declarations, "", file);
      else if (decl.kind === "namespace") walk(decl.declarations, `${prefix}${decl.name}.`, file);
      else if ("name" in decl) library.set(`${prefix}${decl.name}`, [...(library.get(`${prefix}${decl.name}`) ?? []), { decl, file }]);
    }
  };
  try {
    for (const file of await session.libraries()) walk((await session.read(file)).declarations, "", file);
  } finally {
    await session.close();
  }
  return library;
}

/** What the generator doesn't bind, by its name and its file. */
export class Refused extends Error {}
const refuse = (name: string, from: Fragment | undefined, what: string): never => {
  throw new Refused(
    `builtins/library.ts: ${name}${from ? ` (${from.file.split("/").pop()})` : ""}: ${what} isn't a form the js crate binds yet: map it, never \`any\``,
  );
};

/** The name `name` has in `owner`'s namespace: `Intl.Locale`'s `LocaleOptions`,
 * `Intl.LocaleOptions`, or a global one. */
const resolve = (library: Library, owner: string, name: string): string => {
  const namespace = owner.includes(".") ? owner.slice(0, owner.lastIndexOf(".") + 1) : "";
  return library.has(namespace + name) ? namespace + name : name;
};

/** The fragments of `name` of `kind`. */
const fragments = (library: Library, name: string, kind: string): Fragment[] =>
  (library.get(name) ?? []).filter((f) => f.decl.kind === kind);

/** An interface's members, its own then those of what it extends that it
 * doesn't redeclare, each with the fragment it's in. */
export function members(library: Library, name: string): { member: any; from: Fragment }[] {
  const own = fragments(library, name, "interface").flatMap((from) => from.decl.members.map((member: any) => ({ member, from })));
  const named = new Set(own.map(({ member }) => member.name));
  const inherited = fragments(library, name, "interface")
    .flatMap((from) => from.decl.extends.filter((e: any) => e.kind === "reference").map((e: any) => resolve(library, name, e.name)))
    .flatMap((parent: string) => members(library, parent))
    .filter(({ member }) => !named.has(member.name));
  return [...own, ...inherited];
}

/** Whether `name` is a class: a constant of a constructor the library declares. */
const isClass = (library: Library, name: string) =>
  fragments(library, name, "const").some((f) => f.decl.type.kind === "object" && f.decl.type.members.some((m: any) => m.kind === "construct"));

/** A union's string literals, through aliases; none of another union. */
const literals = (library: Library, owner: string, type: any): string[] | undefined => {
  if (type.kind === "reference") {
    const alias = fragments(library, resolve(library, owner, type.name), "type")[0];
    return alias ? literals(library, owner, alias.decl.type) : undefined;
  }
  if (type.kind === "literal" && typeof type.value === "string") return [type.value];
  if (type.kind === "union") {
    const each = type.types.map((t: any) => literals(library, owner, t));
    return each.every((e: string[] | undefined) => e) ? each.flat() : undefined;
  }
  return undefined;
};

/** What `group` writes, and what it wrote already, once each. */
type Writing = { library: Library; written: Set<string>; records: string[] };

/** The enum of a string union `key` names, written once: its Rust name the
 * policy's, its values the library's, in its order. */
function unionNamed(w: Writing, key: string, values: string[]): string {
  const named = NAMES[key] ?? refuse(key, undefined, "a string union with no Rust name in builtins/policy.ts");
  if (!w.written.has(named.rust)) {
    w.written.add(named.rust);
    union(named.rust, `The values of \`${key}\`, each as JS writes it.`, values);
  }
  return named.rust;
}

/** An alias of a string union, `Intl.LocaleHourCycleKey`, written as its enum. */
export function writeUnion(w: Writing, name: string): string {
  const alias = fragments(w.library, name, "type")[0] ?? refuse(name, undefined, "a missing alias");
  const values = literals(w.library, name, alias.decl.type) ?? refuse(name, alias, "an alias that isn't a string union");
  return unionNamed(w, name, values);
}

/** Where a type is: given in options, `&'a str`; or read, `String`. */
type Position = "options" | "read";

/** `type`'s Rust, of `owner`'s `member`, at `position`. */
function rust(w: Writing, type: any, position: Position, at: { owner: string; member: string; from: Fragment }): string {
  const { library } = w;
  if (type.kind === "keyword") {
    if (type.keyword === "string") return position === "options" ? "&'a str" : "String";
    if (type.keyword === "boolean") return "bool";
    if (type.keyword === "number") return "f64";
  }
  if (type.kind === "array" && position === "read") return `Vec<${rust(w, type.element, position, at)}>`;
  const values = literals(library, at.owner, type);
  if (values) {
    if (type.kind === "reference") return writeUnion(w, resolve(library, at.owner, type.name));
    return unionNamed(w, `${at.owner}.${at.member}`, values);
  }
  if (type.kind === "reference") {
    const name = resolve(library, at.owner, type.name);
    const alias = fragments(library, name, "type")[0];
    if (alias) return rust(w, alias.decl.type, position, at);
    if (isClass(library, name) && position === "read") return `&'static ${name.split(".").pop()}`;
    if (fragments(library, name, "interface").length && position === "read") {
      writeRecord(w, name);
      return name.split(".").pop()!;
    }
  }
  return refuse(`${at.owner}.${at.member}`, at.from, `a ${position === "read" ? "result" : "given"} type ${JSON.stringify(type)}`);
}

/** An interface a result is, written once as a record of its fields. */
function writeRecord(w: Writing, name: string) {
  if (w.written.has(name)) return;
  w.written.add(name);
  const fields = members(w.library, name).map(({ member, from }) => {
    if (member.kind !== "property") refuse(`${name}.${member.name}`, from, `a ${member.kind} of a record`);
    const ty = rust(w, member.type, "read", { owner: name, member: member.name, from });
    return [member.name, member.optional ? `Option<${ty}>` : ty] as [string, string];
  });
  w.records.push(name);
  record(name.split(".").pop()!, `What \`${name}\` is: its fields as JS gives them.`, fields, name);
}

/** A class of the library, `Intl.Locale`: its options and what its results
 * are, then its type, each property a getter and each method, then its
 * module's constructor. */
export function writeClass(w: Writing, name: string, receiver: string) {
  const { library } = w;
  const short = name.split(".").pop()!;
  const mdn = (path: string) => page(`${short}/${path}`);
  const ctor = fragments(library, name, "const")[0] ?? refuse(name, undefined, "a class with no constructor");
  const construct = ctor.decl.type.members.filter((m: any) => m.kind === "construct");
  if (construct.length !== 1) refuse(name, ctor, `${construct.length} constructor signatures`);
  const [signature] = construct;
  // Its parameters: a tag of a union the policy names, or options.
  const params = signature.params.map((p: any) => {
    const at = { owner: `new ${name}(${p.name})`, member: p.name, from: ctor };
    if (p.type.kind === "union") {
      const named = NAMES[at.owner] ?? refuse(at.owner, ctor, "a parameter's union with no Rust name in builtins/policy.ts");
      const members = p.type.types.flatMap((t: any) => {
        const target = t.kind === "reference" ? resolve(library, name, t.name) : undefined;
        const alias = target ? fragments(library, target, "type")[0] : undefined;
        const kind = alias ? alias.decl.type : t;
        if (kind.kind === "keyword" && kind.keyword === "string") return ["&str", "String", "&String"];
        if (target && isClass(library, target)) return [`&${target.split(".").pop()}`];
        return refuse(at.owner, ctor, `a parameter union member ${JSON.stringify(t)}`);
      });
      if (!w.written.has(named.rust)) {
        w.written.add(named.rust);
        argument(named.rust, `string | ${name}`, members);
      }
      return { js: p.name, rust: `${snake(p.name)}: impl ${named.rust}` };
    }
    if (p.type.kind === "reference" && fragments(library, resolve(library, name, p.type.name), "interface").length) {
      const options = resolve(library, name, p.type.name);
      writeOptions(w, options);
      const optionsName = options.split(".").pop()!;
      return { js: p.name, rust: `${snake(p.name)}: ${p.optional ? `Option<&${optionsName}<'_>>` : `&${optionsName}<'_>`}` };
    }
    return refuse(at.owner, ctor, `a constructor parameter ${JSON.stringify(p.type)}`);
  });
  // What its results are, records and enums, before its type.
  const own = members(library, name);
  const getters = own.filter(({ member }) => member.kind === "property");
  const methods = own.filter(({ member }) => member.kind === "method");
  const results = new Map<string, string>();
  for (const { member, from } of [...getters, ...methods]) {
    const type = member.kind === "method" ? member.returns : member.type;
    const optional = member.optional || (type.kind === "union" && type.types.some((t: any) => t.kind === "keyword" && t.keyword === "undefined"));
    const defined = type.kind === "union" ? { ...type, types: type.types.filter((t: any) => !(t.kind === "keyword" && t.keyword === "undefined")) } : type;
    const single = defined.kind === "union" && defined.types.length === 1 ? defined.types[0] : defined;
    if (member.kind === "method" && member.params.length) refuse(`${name}.${member.name}`, from, "a method's parameters");
    const ty = rust(w, single, "read", { owner: name, member: member.name, from });
    results.set(member.name, optional ? `Option<${ty}>` : ty);
  }
  line(`/// [\`${name}\`](${page(short)}), as TypeScript's library declares it.`);
  line(`#[cfg_attr(rust_js, rust_js::name = ${JSON.stringify(name)})]`);
  line(`#[cfg_attr(rust_js, rust_js::types = ${JSON.stringify(name)})]`);
  line(`pub struct ${short}(PhantomData<JsObject>);`);
  line();
  classed(short);
  line();
  line(`impl ${short} {`);
  for (const { member } of getters) {
    const ty = results.get(member.name)!;
    method(`[\`${receiver}.${member.name}\`](${mdn(member.name)}).`, `get ${member.name}`, `${snake(member.name)}(&self) -> ${ty}`, "    ", ty.startsWith("Option<") ? ["none_undefined"] : []);
  }
  for (const { member } of methods) {
    const ty = results.get(member.name)!;
    const quiet = attributesOf(`${name}.prototype.${member.name}`);
    method(`[\`${receiver}.${member.name}()\`](${mdn(member.name)}).`, member.name, `${snake(member.name)}(&self) -> ${ty}`, "    ", [...(ty.startsWith("Option<") ? ["none_undefined"] : []), ...quiet]);
  }
  close();
  line("}");
  line();
  line(`pub mod ${snake(short)} {`);
  line("    use super::*;");
  line();
  const js = `new ${name}`;
  const throws = THROWS[js];
  const made = `&'static ${short}`;
  method(
    `[\`${js}(${params.map((p: any) => p.js).join(", ")})\`](${mdn(short)})${throws ? `, or the \`${throws.error}\` it throws` : ""}.`,
    js,
    `new(${params.map((p: any) => p.rust).join(", ")}) -> ${throws ? `Result<${made}, &'static JsError>` : made}`,
    "    ",
    runs(js),
  );
  close();
  line("}");
  line();
}

/** An interface given as options, written once: each field an `Option`,
 * as JS reads an `undefined` option. */
function writeOptions(w: Writing, name: string) {
  if (w.written.has(name)) return;
  w.written.add(name);
  const fields = members(w.library, name).map(({ member, from }) => {
    if (member.kind !== "property" || !member.optional) refuse(`${name}.${member.name}`, from, "an options member that isn't an optional property");
    return [member.name, rust(w, member.type, "options", { owner: name, member: member.name, from })] as [string, string];
  });
  options(name.split(".").pop()!, `What \`${name}\` is given: each a part, \`None\` not given.`, fields, [], name);
}

/** A writer of `library`'s groups. */
export function writing(library: Library): Writing {
  return { library, written: new Set(), records: [] };
}
