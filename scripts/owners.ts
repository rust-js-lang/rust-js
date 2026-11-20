// The table of semantic owners, docs/architecture.md's, kept true (#2162,
// ADR 1175): each question one owner answers, the API the others ask, and
// where it bottoms out in one primitive, that primitive. Two checks hold it:
//
//   completeness  every function of the front end visible outside its
//                 module that answers a question is an API a row names,
//                 under that row's owner, or is marked a helper with its
//                 reason; none is neither (the sweeps' ratchet reached 0
//                 and went, #2162).
//   exclusivity   a row's primitive is read in no file but its owners',
//                 and by one of them; a guess it names after "instead of",
//                 a type's shape that stood in for it, is read in no file
//                 but its owners'.
//
// test/architecture-owners.test.ts runs both; `bun run architecture`
// reports them.

import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

/** The page whose table it is. */
export const OWNER_PAGE = "docs/architecture.md";
/** The table's header row. */
export const HEADER = "| Question | Owner | API | Primitive | Source |";
/** The mark that says a candidate is a helper, not an answer others ask:
 * `// helper: <reason>`, in the comment and attribute lines directly above
 * its `fn`. */
export const HELPER_MARK = /^\s*\/\/ helper: \S/;

export type OwnerRow = {
  /** Its line in the page, from 1. */
  line: number;
  question: string;
  /** Each module of its Owner cell, as written. */
  owners: string[];
  /** Each name of its API cell. */
  api: string[];
  /** Each token of its Primitive cell before "instead of". */
  primitives: string[];
  /** Each token of its Primitive cell after "instead of": a guess the
   * primitive replaced, which only its owners may read (#2159). */
  guesses: string[];
  /** Its Source cell's class, the weakest source any of its answers reads,
   * 1 MIR or rustc's own results to 5 a guess from JS shape (#2200). */
  source: number;
  /** What its Source cell says after the class: the weak APIs, a reason. */
  sourceSaid: string;
};

/** A Source cell: a class from 1, MIR or rustc's
 * own results, 2 a type predicate, 3 a crate-wide fact keyed by type, 4 a
 * fact riding a JS node, to 5 a guess from JS shape, the weakest source
 * any of the row's answers reads; after it, `: ` and the weak APIs, or a
 * reason its source is enough (#2200). */
export const SOURCE = /^([1-5])(?:: (.+))?$/;

/** The rows of each class, 1 to 5. */
export function sourceTally(rows: readonly OwnerRow[]): number[] {
  const tally = [0, 0, 0, 0, 0];
  for (const row of rows) tally[row.source - 1]!++;
  return tally;
}

/** The tally said in a line, as check:fast prints it. */
export const sourceLine = (rows: readonly OwnerRow[]): string => {
  return `owner rows by source (#2200): ${sourceTally(rows).map((n, i) => `${i + 1}: ${n}`).join(", ")}`;
};

/** What parts a Primitive cell: the primitives before it, the guesses they
 * replaced after it. */
export const INSTEAD_OF = /\binstead of\b/;

/** A row's cells: split at each `|` not escaped `\|`, which GitHub splits
 * at inside a code span too. */
export function cells(line: string): string[] {
  const parts = line.trim().replace(/^\|/, "").replace(/\|$/, "").split(/(?<!\\)\|/);
  return parts.map((part) => part.trim());
}

/** The backticked names of a cell: `x`, each of `a`, `b`. */
export const ticked = (cell: string): string[] => [...cell.matchAll(/`([^`]+)`/g)].map((m) => m[1]!.replace(/\\\|/g, "|"));

/** The table's rows, from its header to the first line that isn't one.
 * Each row has the header's four cells and an owner, or it's refused with
 * its line: a row of the wrong shape was skipped by the checks before. */
export function ownerTable(page: string): OwnerRow[] {
  const lines = page.split("\n");
  const start = lines.indexOf(HEADER);
  if (start < 0) throw new Error(`${OWNER_PAGE}: no table of semantic owners headed \`${HEADER}\``);
  const rows: OwnerRow[] = [];
  for (let i = start + 2; i < lines.length && lines[i]!.startsWith("|"); i++) {
    const row = cells(lines[i]!);
    if (row.length !== 5) throw new Error(`${OWNER_PAGE}:${i + 1}: a row of ${row.length} cells, not the header's 5 (escape a \`|\` in a cell as \`\\|\`)`);
    const [question, owner, api, primitive, sourceCell] = row as [string, string, string, string, string];
    const source = SOURCE.exec(sourceCell);
    if (source === null) {
      throw new Error(`${OWNER_PAGE}:${i + 1}: a Source cell \`${sourceCell}\` that isn't a class 1 to 5, then \`: \` and its weak APIs or reason (#2200)`);
    }
    const owners = ticked(owner);
    if (owners.length === 0) throw new Error(`${OWNER_PAGE}:${i + 1}: a row that names no owner module`);
    const [held, replaced = ""] = primitive.split(INSTEAD_OF);
    rows.push({ line: i + 1, question, owners, api: ticked(api), primitives: ticked(held!), guesses: ticked(replaced), source: Number(source[1]), sourceSaid: source[2] ?? "" });
  }
  return rows;
}

/** The repository path `owner` names, as the table spells it: a path
 * from `src/` where it starts so; else a module of the front end,
 * `iterators.rs`, `src/lower/iterators.rs`, or a folder, `drops/`, where
 * it or its submodules are files; else one at `src/`'s root, `names.rs`,
 * or the runtime's, `runtime/steps.js`. The one rule of it, which
 * test/architecture-iterators asks too. */
export function ownerPath(owner: string, files: readonly string[]): string {
  if (owner.startsWith("src/")) return owner;
  const lowered = `src/lower/${owner}`;
  const exists = files.some((f) => f === lowered || f.startsWith(lowered.endsWith(".rs") ? `${lowered.slice(0, -3)}/` : lowered));
  return exists ? lowered : `src/${owner}`;
}

/** Whether `file`, a repository path, is of `owner` (`ownerPath`): that
 * module and its submodules, `src/lower/iterators/`, or what a folder
 * holds. */
export function ofOwner(owner: string, file: string, files: readonly string[]): boolean {
  const base = ownerPath(owner, files);
  if (file === base || (base.endsWith("/") && file.startsWith(base))) return true;
  return base.endsWith(".rs") && file.startsWith(`${base.slice(0, -3)}/`);
}

/** Whether `owner` names a module there is. */
export const ownerFound = (owner: string, files: readonly string[]) => files.some((file) => ofOwner(owner, file, files));

/** Rust source with its comments, string literals and char literals
 * blanked, each line where it was, and its `#[cfg(test)]` modules cut. */
export function rustCode(text: string): string {
  let out = "";
  const n = text.length;
  const newlines = (from: number, to: number) => "\n".repeat((text.slice(from, to).match(/\n/g) ?? []).length);
  for (let i = 0; i < n; ) {
    const c = text[i]!;
    const word = /\w/.test(text[i - 1] ?? "");
    if (c === "/" && text[i + 1] === "/") {
      while (i < n && text[i] !== "\n") i++;
      continue;
    }
    if (c === "/" && text[i + 1] === "*") {
      const from = i;
      let depth = 0;
      do {
        if (text.startsWith("/*", i)) (depth++, (i += 2));
        else if (text.startsWith("*/", i)) (depth--, (i += 2));
        else i++;
      } while (i < n && depth > 0);
      out += newlines(from, i);
      continue;
    }
    const raw = word ? null : /^b?r(#*)"/.exec(text.slice(i, i + 12));
    if (raw) {
      const end = text.indexOf(`"${raw[1]}`, i + raw[0].length);
      const to = end < 0 ? n : end + 1 + raw[1]!.length;
      out += `""${newlines(i, to)}`;
      i = to;
      continue;
    }
    if (c === '"' || (c === "b" && text[i + 1] === '"' && !word)) {
      let j = i + (c === "b" ? 2 : 1);
      while (j < n && text[j] !== '"') j += text[j] === "\\" ? 2 : 1;
      out += `""${newlines(i, j)}`;
      i = j + 1;
      continue;
    }
    if ((c === "'" || (c === "b" && text[i + 1] === "'")) && !word) {
      // A char literal, `'x'`, `'\n'`, `'\u{2028}'`; a lifetime, `'a`, stays.
      const char = /^b?'(?:\\(?:u\{[0-9a-fA-F]+\}|x[0-9a-fA-F]{2}|.)|[^\\'\n])'/u.exec(text.slice(i, i + 14));
      if (char) {
        out += "' '";
        i += char[0].length;
        continue;
      }
    }
    out += c;
    i++;
  }
  const tests = out.search(/#\[cfg\(test\)\]\s*(?:pub(?:\([^)]*\))?\s+)?mod\s/);
  return tests < 0 ? out : out.slice(0, tests);
}

/** The runtime's JS with its comments and string literals blanked, each
 * line where it was. */
export function jsCode(text: string): string {
  let out = "";
  const n = text.length;
  for (let i = 0; i < n; ) {
    const c = text[i]!;
    if (c === "/" && text[i + 1] === "/") {
      while (i < n && text[i] !== "\n") i++;
      continue;
    }
    if (c === "/" && text[i + 1] === "*") {
      const end = text.indexOf("*/", i + 2);
      const to = end < 0 ? n : end + 2;
      out += "\n".repeat((text.slice(i, to).match(/\n/g) ?? []).length);
      i = to;
      continue;
    }
    if (c === '"' || c === "'" || c === "`") {
      let j = i + 1;
      while (j < n && text[j] !== c) j += text[j] === "\\" ? 2 : 1;
      out += `${c}${c}${"\n".repeat((text.slice(i, j).match(/\n/g) ?? []).length)}`;
      i = j + 1;
      continue;
    }
    out += c;
    i++;
  }
  return out;
}

/** A function visible outside its module: `pub`, `pub(crate)` or
 * `pub(in crate::…)`; `pub(super)` too in `src/lower.rs` or a module right
 * under `src/lower/` (`TOP`), whose parent is the whole front end. Not a
 * nested module's `pub(super)`, whose answer only its parent's modules may
 * ask. */
const VISIBLE = /^\s*pub(?:\((?:crate|in\s+crate(?:::\w+)*)\))?\s+(?:(?:const|unsafe|async)\s+)*fn\s+(\w+)/;
const VISIBLE_TOP = /^\s*pub(?:\((?:crate|super|in\s+crate(?:::\w+)*)\))?\s+(?:(?:const|unsafe|async)\s+)*fn\s+(\w+)/;
/** A file whose `pub(super)` reaches the whole front end. */
export const TOP = (file: string) => file === "src/lower.rs" || /^src\/lower\/[^/]+\.rs$/.test(file);
/** A JS tree type: an `Option` of one is lowering's output, not an answer. */
export const JS_TREE = /\b(?:Expr|ExprKind|Stmt|StmtKind|Pattern|Prop|Function|Value)\b|\bjs::/;

/** The return type of the function whose name ends at `from` in `code`,
 * `undefined` where it returns `()`. */
export function returnType(code: string, from: number): string | undefined {
  let i = from;
  for (let angle = 0; i < code.length; i++) {
    if (code[i] === "<") angle++;
    else if (code[i] === ">" && code[i - 1] !== "-") angle--;
    else if (code[i] === "(" && angle === 0) break;
  }
  for (let depth = 0; i < code.length; i++) {
    if (code[i] === "(") depth++;
    else if (code[i] === ")" && --depth === 0) {
      i++;
      break;
    }
  }
  const arrow = /^\s*->\s*/.exec(code.slice(i));
  if (!arrow) return undefined;
  const start = i + arrow[0].length;
  let j = start;
  for (let depth = 0; j < code.length; j++) {
    const c = code[j]!;
    if ("<([".includes(c)) depth++;
    else if (">)]".includes(c) && code[j - 1] !== "-") depth--;
    else if (depth === 0 && (c === "{" || c === ";" || /^\swhere\b/.test(code.slice(j, j + 7)))) break;
  }
  return code.slice(start, j).replace(/\s+/g, " ").trim();
}

/** The enums of `codes` whose every variant is a unit's, `enum Side { Front, Back }`. */
export function fieldlessEnums(codes: Iterable<string>): Set<string> {
  const found = new Set<string>();
  for (const code of codes) {
    for (const m of code.matchAll(/\benum\s+(\w+)(?:<[^{]*>)?\s*\{([^{}]*)\}/g)) if (!/[({]/.test(m[2]!)) found.add(m[1]!);
  }
  return found;
}

/** Whether a function that returns `type` answers a question: `bool`; an
 * `Option` of what isn't a JS tree; one of `enums`; or one of those in
 * `R<…>` or a `Result`. */
export function answers(type: string, enums: ReadonlySet<string>): boolean {
  if (type === "bool") return true;
  const tried = /^(?:R|Result)<(.*)>$/.exec(type);
  if (tried) return answers(firstArgument(tried[1]!), enums);
  const option = /^Option<(.*)>$/.exec(type);
  if (option) return !JS_TREE.test(option[1]!);
  return enums.has(type.replace(/<.*$/, ""));
}

/** The first of a type's arguments, `T` of `T, Box<E, A>`. */
function firstArgument(args: string): string {
  for (let i = 0, depth = 0; i < args.length; i++) {
    if ("<([".includes(args[i]!)) depth++;
    else if (">)]".includes(args[i]!) && args[i - 1] !== "-") depth--;
    else if (args[i] === "," && depth === 0) return args.slice(0, i).trim();
  }
  return args.trim();
}

export type Candidate = { file: string; line: number; name: string; returns: string; helper: boolean };

/** Each function of `text`, the source of `file`, visible outside its
 * module whose return answers a question, and whether a helper mark marks
 * it. */
export function candidates(file: string, text: string, enums: ReadonlySet<string>): Candidate[] {
  const code = rustCode(text);
  const lines = code.split("\n");
  const source = text.split("\n");
  const found: Candidate[] = [];
  let offset = 0;
  for (let at = 0; at < lines.length; offset += lines[at]!.length + 1, at++) {
    const visible = (TOP(file) ? VISIBLE_TOP : VISIBLE).exec(lines[at]!);
    if (!visible) continue;
    const returns = returnType(code, offset + visible.index + visible[0].length);
    if (returns === undefined || !answers(returns, enums)) continue;
    let helper = false;
    for (let above = at - 1; above >= 0 && /^\s*(?:\/\/|#\[)/.test(source[above]!); above--) helper ||= HELPER_MARK.test(source[above]!);
    found.push({ file, line: at + 1, name: visible[1]!, returns, helper });
  }
  return found;
}

/** Whether a row names `candidate`: its name, or `Type::name`, in the API
 * cell, its file of the row's owners. */
export const names = (row: OwnerRow, candidate: Candidate, files: readonly string[]) =>
  row.api.some((api) => api === candidate.name || api.endsWith(`::${candidate.name}`)) && row.owners.some((owner) => ofOwner(owner, candidate.file, files));

/** A candidate as completeness says it: its file and its name. */
export const listed = (candidate: Candidate) => `${candidate.file} ${candidate.name}`;

/** What completeness finds: the candidates no row names and no mark marks,
 * sorted, which must be none; and those both a row names and a mark marks,
 * which say two things. */
export function completeness(rows: OwnerRow[], all: Candidate[], files: readonly string[]) {
  const named = (candidate: Candidate) => rows.some((row) => names(row, candidate, files));
  return {
    unclassified: [...new Set(all.filter((c) => !c.helper && !named(c)).map(listed))].sort(),
    both: all.filter((c) => c.helper && named(c)).map((c) => `${c.file}:${c.line} ${c.name}`),
  };
}

/** A primitive's pattern: its token, any run of spaces in it any run, and
 * a word's edge where it starts or ends with one. */
export function primitivePattern(token: string): RegExp {
  const escaped = token
    .trim()
    .split(/\s+/)
    .map((part) => part.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"))
    .join("\\s+");
  return new RegExp(`${/^\w/.test(token) ? "(?<![\\w$])" : ""}${escaped}${/\w$/.test(token) ? "(?![\\w$])" : ""}`);
}

/** What exclusivity finds: each primitive or guess read in a file not its
 * row's owners', `token in file`, and each primitive read by none of its
 * owners, which holds nothing; a guess may be read by none, replaced
 * everywhere. `code` is each source's code (`rustCode`, `jsCode`). */
export function exclusivity(rows: OwnerRow[], code: ReadonlyMap<string, string>) {
  const files = [...code.keys()];
  const outside: string[] = [];
  const unread: string[] = [];
  for (const row of rows) {
    const own = (file: string) => row.owners.some((owner) => ofOwner(owner, file, files));
    for (const [token, guess] of [...row.primitives.map((t) => [t, false] as const), ...row.guesses.map((t) => [t, true] as const)]) {
      const pattern = primitivePattern(token);
      const readers = files.filter((file) => pattern.test(code.get(file)!));
      for (const file of readers.filter((file) => !own(file))) outside.push(`\`${token}\` (line ${row.line}) in ${file}`);
      if (!guess && !readers.some(own)) unread.push(`\`${token}\` (line ${row.line})`);
    }
  }
  return { outside, unread };
}

/** The repository's sources, in name order: each `.rs` of `src/` and each
 * `.js` of `src/runtime/`. */
export function sources(root: string): string[] {
  const walk = (directory: string): string[] =>
    readdirSync(join(root, directory), { withFileTypes: true })
      .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
      .flatMap((entry) => (entry.isDirectory() ? walk(`${directory}/${entry.name}`) : [`${directory}/${entry.name}`]));
  return walk("src").filter((file) => file.endsWith(".rs") || (file.startsWith("src/runtime/") && file.endsWith(".js")));
}

/** The front end's files, whose candidates completeness asks of. */
export const ofFrontEnd = (file: string) => file === "src/lower.rs" || (file.startsWith("src/lower/") && file.endsWith(".rs"));

/** Both checks over the repository at `root`. */
export function ownersState(root: string) {
  const files = sources(root);
  const text = new Map(files.map((file) => [file, readFileSync(join(root, file), "utf8")]));
  const code = new Map(files.map((file) => [file, file.endsWith(".js") ? jsCode(text.get(file)!) : rustCode(text.get(file)!)]));
  const rows = ownerTable(readFileSync(join(root, OWNER_PAGE), "utf8"));
  const enums = fieldlessEnums([...code.entries()].filter(([file]) => file.endsWith(".rs")).map(([, c]) => c));
  for (const row of rows) for (const api of row.api) enums.add(api.replace(/^.*::/, ""));
  const all = files.filter(ofFrontEnd).flatMap((file) => candidates(file, text.get(file)!, enums));
  return { files, text, code, rows, all, ...completeness(rows, all, files), ...exclusivity(rows, code) };
}

if (import.meta.main) {
  const state = ownersState(join(import.meta.dir, ".."));
  const helpers = state.all.filter((c) => c.helper).length;
  console.log(`${state.rows.length} owner rows; ${state.all.length} answers visible outside their modules: ${state.all.length - helpers - state.unclassified.length} named in rows, ${helpers} helpers, ${state.unclassified.length} unclassified`);
  const byArea = new Map<string, number>();
  for (const line of state.unclassified) {
    const dir = line.split(" ")[0]!.replace(/^src\/lower\//, "").replace(/\/[^/]*$|\.rs$/, "");
    byArea.set(dir, (byArea.get(dir) ?? 0) + 1);
  }
  for (const [dir, n] of [...byArea].sort((a, b) => b[1] - a[1]).slice(0, 25)) console.log(`${String(n).padStart(5)}  ${dir}`);
  for (const line of state.outside) console.log(`primitive read outside its owners: ${line}`);
  console.log(sourceLine(state.rows));
}
