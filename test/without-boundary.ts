// A module's JS as it reads without the boundary of what JS gives Rust
// (#2403, ADRs 1192–1196), its checks and the split they make, so a test of
// another feature asserts that feature's JS alone, as it did before them,
// while its code still runs with them. A boundary suite, a snapshot and a
// test of behaviour keep both, which is where the boundary's own bugs show. The checks are recognised by the names the runtime makes them by,
// each `$check…` and `$brand…` function std's runtime's boundary group declares, never
// by a pattern of this file's own, so it can't strip what isn't one.
//
// Removed: a statement that calls one, `$checkU32(n, "f(n)");`, or assigns
// what one gives, `rule = $checkCallback(rule, ..);`; a call that wraps a
// value, `$brand(key, value)` and `$checkResult(value, ..)`, its value
// left; a module's checker, `function checkOrder(value) { .. }`, once its
// body is only checks; and each of their names in an import.
import { readFileSync } from "node:fs";
import { join } from "node:path";

// `@rust-js/std`'s runtime, which bundles the boundary's helpers as a group
// of its own; read there, never in the compiler's sources, so a test that
// strips them isn't taken for one that reads the compiler (scripts/testMap.ts).
const runtime = readFileSync(join(import.meta.dir, "..", "std", "runtime.js"), "utf8");

/** The names the runtime makes the checks by: its boundary group's, from
 * its first comment, each `$check..` or `$brand..` up to the next group's
 * first, so never std's own `$checkRange` or `$checkedDiv`, elsewhere. */
export const checkHelpers: ReadonlySet<string> = (() => {
  const from = runtime.indexOf("// What a JS caller gives a Rust parameter, checked against its Rust type");
  if (from < 0) throw new Error("withoutBoundary: std's runtime has no boundary group");
  const names: string[] = [];
  for (const [, name] of runtime.slice(from).matchAll(/^export function (\$\w+)\(/gm)) {
    if (!/^\$(?:check|brand)/.test(name!)) break;
    names.push(name!);
  }
  return new Set(names);
})();

/** The helpers that wrap a value, leaving it: their first argument's, or
 * `$brand`'s second. */
const wrappers: Record<string, number> = { $brand: 1, $checkResult: 0 };

/** The printer's width, which an import's names are laid out at. */
const width = 100;

/** Where the call whose `(` is at `open` ends, past its `)`. */
function closing(js: string, open: number): number {
  let depth = 0;
  for (let i = open; i < js.length; i++) {
    const c = js[i];
    if (c === '"' || c === "'" || c === "`") {
      i = stringEnd(js, i);
      continue;
    }
    if (c === "(") depth++;
    if (c === ")" && --depth === 0) return i + 1;
  }
  throw new Error(`withoutChecks: no end of the call at ${open}`);
}

/** Where the string that starts at `at` ends, at its closing quote. */
function stringEnd(js: string, at: number): number {
  const quote = js[at];
  for (let i = at + 1; i < js.length; i++) {
    if (js[i] === "\\") i++;
    else if (js[i] === quote) return i;
  }
  return js.length;
}

/** The top-level arguments of the call whose `(` is at `open`. */
function args(js: string, open: number, end: number): string[] {
  const parts: string[] = [];
  let depth = 0;
  let start = open + 1;
  for (let i = open + 1; i < end - 1; i++) {
    const c = js[i];
    if (c === '"' || c === "'" || c === "`") {
      i = stringEnd(js, i);
      continue;
    }
    if (c === "(" || c === "[" || c === "{") depth++;
    if (c === ")" || c === "]" || c === "}") depth--;
    if (c === "," && depth === 0) {
      parts.push(js.slice(start, i).trim());
      start = i + 1;
    }
  }
  parts.push(js.slice(start, end - 1).trim());
  return parts.filter((part) => part !== "");
}

/** Where the bracket that opens at `open` closes, past it. */
function bracketEnd(js: string, open: number): number {
  let depth = 0;
  for (let i = open; i < js.length; i++) {
    const c = js[i];
    if (c === '"' || c === "'" || c === "`") {
      i = stringEnd(js, i);
      continue;
    }
    if (c === "(" || c === "[" || c === "{") depth++;
    if ((c === ")" || c === "]" || c === "}") && --depth === 0) return i + 1;
  }
  return js.length;
}

/** `js` with each call whose only argument is an object or an array on
 * lines of its own, `push(\n  {\n    a,\n  },\n)`, which a wrapper taken
 * away leaves, hugging it, `push({\n  a,\n})`, as the printer lays a
 * call out that has no wrapper there. */
function hugged(js: string): string {
  let out = js;
  const lone = /\(\n([ \t]*)([{[])\n/g;
  for (let m = lone.exec(out); m; m = lone.exec(out)) {
    const open = m.index + m[0].length - 2;
    const end = bracketEnd(out, open);
    const after = /^,?\n[ \t]*\)/.exec(out.slice(end));
    if (!after) continue;
    const inner = out.slice(open, end).replace(/\n  /g, "\n");
    out = out.slice(0, m.index + 1) + inner + ")" + out.slice(end + after[0].length);
    lone.lastIndex = m.index + 1;
  }
  return out;
}

/** `js` without its checks. */
function withoutChecks(js: string): string {
  let out = js;
  const names = [...checkHelpers].sort((a, b) => b.length - a.length).map((name) => name.replace("$", "\\$"));
  // Each statement of a check, `$check..(..);`, `x = $check..(..);` or an
  // enum's checker's `const tag = $checkTag(..);`, its line's indent and its
  // newline with it.
  const statement = new RegExp(`(^|\\n)([ \\t]*)(?:(?:const )?[\\w$.]+ = )?(${names.join("|")})\\(`, "g");
  for (let m = statement.exec(out); m; m = statement.exec(out)) {
    if (m[3]! in wrappers) continue;
    const open = m.index + m[0].length - 1;
    let end = closing(out, open);
    if (out[end] === ";") end++;
    out = out.slice(0, m.index + m[1]!.length) + out.slice(end + (out[end] === "\n" ? 1 : 0));
    statement.lastIndex = m.index;
  }
  // Each call that wraps a value, its value.
  const wrapping = new RegExp(`(${Object.keys(wrappers).map((name) => name.replace("$", "\\$")).join("|")})\\(`, "g");
  for (let m = wrapping.exec(out); m; m = wrapping.exec(out)) {
    const open = m.index + m[0].length - 1;
    const end = closing(out, open);
    let value = args(out, open, end)[wrappers[m[1]!]!]!;
    // Laid out over lines, its value on a line of its own two past the
    // wrapper's: each of its lines back as the value alone lays out.
    if (/^\(\s*\n/.test(out.slice(open))) value = value.replace(/\n  /g, "\n");
    // An object literal an arrow gives, `() => $brand(k, { n: 0 })`, kept an
    // expression, `() => ({ n: 0 })`, not read as a block.
    const arrow = /=>\s*$/.test(out.slice(0, m.index)) && value.startsWith("{");
    out = out.slice(0, m.index) + (arrow ? `(${value})` : value) + out.slice(end);
    wrapping.lastIndex = m.index;
  }
  // A struct's checker that's its table's constant, `const checkPoint =
  // /* @__PURE__ */ $checkShape(..);`, and the blank line after it (#2403).
  const shape = /(^|\n)const check[A-Z]\w* = \/\* @__PURE__ \*\/ \$checkShape\(/g;
  for (let m = shape.exec(out); m; m = shape.exec(out)) {
    const open = m.index + m[0].length - 1;
    let end = closing(out, open);
    if (out[end] === ";") end++;
    if (out[end] === "\n") end++;
    if (out[end] === "\n") end++;
    out = out.slice(0, m.index + m[1]!.length) + out.slice(end);
    shape.lastIndex = m.index;
  }
  // An enum's checker's test of a variant its checks emptied.
  out = out.replace(/\n[ \t]*if \(tag === "[^"]*"\) \{\n[ \t]*\}(?=\n)/g, "");
  // A module's checker its checks emptied, and the blank line after it.
  out = out.replace(/(^|\n)function check[A-Z]\w*\([^)]*\) \{\n\}\n(\n)?/g, "$1");
  // Each import's names but theirs, laid out as the printer lays one out.
  out = out.replace(/import \{([^}]*)\} from ("[^"]+");\n/g, (all, list: string, from: string) => {
    const kept = list.split(",").map((name) => name.trim()).filter((name) => name !== "" && !checkHelpers.has(name));
    if (kept.length === list.split(",").map((name) => name.trim()).filter((name) => name !== "").length) return all;
    if (kept.length === 0) return "";
    const line = `import { ${kept.join(", ")} } from ${from};`;
    return line.length <= width ? `${line}\n` : `import {\n${kept.map((name) => `  ${name},\n`).join("")}} from ${from};\n`;
  });
  return hugged(out).replace(/\n{3,}/g, "\n\n");
}

// The split (ADR 1193): its owner, `boundary/split.rs`, names a checked
// function's body as its entry is named and `$`, `count$`, whose entry,
// its checks gone, is `return count$(..)` alone. Undone: the entry
// dropped, its body named as it is, exported as it was, each call and
// import of the body the function's. Only a name the split made, a body
// declared here or imported from another of the crate's modules, never a
// word JS keeps apart, `class$`.
function withoutSplit(js: string): string {
  let out = js
  // Each body whose entry is here, `return count$(..)` alone: a pair the
  // split made, never a name a clash or a keyword spelled apart.
  const split: string[] = []
  for (const [, name] of js.matchAll(/^(?:export )?function (\w+)\$\(/gm)) {
    const entry = new RegExp(`^(export )?function ${name}\\(([^)]*)\\) \\{\\n  return ${name}\\$\\(\\2\\);\\n\\}\\n\\n?`, "m")
    const found = entry.exec(out)
    if (!found) continue
    split.push(name!)
    out = out.slice(0, found.index) + out.slice(found.index + found[0].length)
    const exported = found[1] !== undefined
    out = out.replace(new RegExp(`^(export )?function ${name}\\$\\(`, "m"), (_, was) => `${exported || was ? "export " : ""}function ${name}(`)
  }
  // And each body a module variable is, `let getTime$;`, its entry calling it
  // alone: the variable by the entry's name, exported as the entry was.
  for (const [, name] of js.matchAll(/^(?:let|const) (\w+)\$(?: =|;)/gm)) {
    const entry = new RegExp(`^(export )?function ${name}\\(([^)]*)\\) \\{\\n  return ${name}\\$\\(\\2\\);\\n\\}\\n\\n?`, "m")
    const found = entry.exec(out)
    if (!found) continue
    split.push(name!)
    out = out.slice(0, found.index) + (found[1] ? `export { ${name} };\n\n` : "") + out.slice(found.index + found[0].length)
  }
  // And each namespace's method split, its entry and its body members of
  // its object, `fromBitsRetain(bits) { return Perms.fromBitsRetain$(bits); }`
  // then `fromBitsRetain$(bits) { .. }` (#2403): the entry dropped, its body
  // named as it is.
  for (const [, indent, name] of js.matchAll(/^( +)(\w+)\$\(/gm)) {
    const inner = indent + "  "
    const entry = new RegExp(`^${indent}${name}\\(([^)]*)\\) \\{\\n${inner}return [\\w$]+\\.${name}\\$\\(\\1\\);\\n${indent}\\},\\n`, "m")
    const found = entry.exec(out)
    if (!found) continue
    split.push(name!)
    out = out.slice(0, found.index) + out.slice(found.index + found[0].length)
  }
  // And each another module split, the crate's or a library's, by its
  // file, its package or its subpath, imported `f$ as f`.
  for (const [, list] of js.matchAll(/^import \{([^}]*)\} from "[^"]*";$/gm)) {
    for (const [, name] of list!.matchAll(/(\w+)\$ as \1\b/g)) split.push(name!)
  }
  // And one a `use` renames, `close$ as closeFlight`, read by its new name:
  // its import's name alone, but a word JS keeps apart, `new$ as make`.
  out = out.replace(/^import \{([^}]*)\} from "[^"]*";$/gm, (line) =>
    line.replace(/\b(\w+)\$ as (\w+)\b/g, (all, name: string, alias: string) => (name === alias || reserved.has(name) ? all : `${name} as ${alias}`)),
  )
  for (const name of new Set(split)) {
    out = out.replace(new RegExp(`\\b${name}\\$ as ${name}\\b`, "g"), name)
    // Called, read, or a namespace's member, `People.fullName$`.
    out = out.replace(new RegExp(`(?<![\\w$])${name}\\$(?![\\w$])`, "g"), name)
  }
  // And a body read through another module's namespace, the crate's or a
  // library's, `People.fullName$`, but a word JS keeps apart, `People.new$`.
  for (const [, namespace] of js.matchAll(/^import \* as ([\w$]+) from "[^"]*";$/gm)) {
    const spelled = namespace!.replace(/\$/g, "\\$")
    out = out.replace(new RegExp(`(?<![\\w$])${spelled}\\.(\\w+)\\$(?![\\w$])`, "g"), (all, name: string) =>
      reserved.has(name) ? all : `${namespace}.${name}`,
    )
  }
  // Each import laid out again as the printer lays one out, its names now
  // the entries' shorter ones: on one line where it fits.
  return out.replace(/^import \{\n([^}]*)\} from ("[^"]+");$/gm, (all, list: string, from: string) => {
    const names = list.split(",").map((name) => name.trim()).filter((name) => name !== "");
    const line = `import { ${names.join(", ")} } from ${from};`;
    return line.length <= width ? line : all;
  })
}

/** `js` on one line, as a test of what it says, not where it breaks, reads
 * it: a check taken away leaves the lines its wrapper broke the code at,
 * which the printer, laying it out without the check, wouldn't. Each run
 * of whitespace one space, none inside brackets, no trailing comma,
 * no parentheses around all a `return` gives. */
export function oneLine(js: string): string {
  let out = js
    .replace(/\s+/g, " ")
    .replace(/([([{]) /g, "$1")
    .replace(/,? ([)\]}])/g, "$1")
    .replace(/ \./g, ".");
  // And the parentheses the printer puts around all a broken `return` gives.
  for (let at = out.indexOf("return ("); at >= 0; at = out.indexOf("return (", at + 1)) {
    const open = at + "return ".length;
    const end = bracketEnd(out, open);
    if (out[end] === ";") out = out.slice(0, open) + out.slice(open + 1, end - 1) + out.slice(end);
  }
  return out;
}

/** `js` without its boundary: its checks, then the split they make. */
export function withoutBoundary(js: string): string {
  return withoutSplit(withoutChecks(js))
}

/** The words JS keeps apart, which rust-js spells with a `$`, `new$`. */
const reserved = new Set(
  "break case catch class const continue debugger default delete do else enum export extends false finally for function if import in instanceof new null return super switch this throw true try typeof var void while with yield let static implements interface package private protected public await arguments eval".split(" "),
)
