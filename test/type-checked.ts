// TypeScript's check of what imports a module's JS and its declarations
// (ADR 0196): the one check every test of generated declarations makes,
// each under the settings it asks for, over what every check has (#2103).

import { expect } from "bun:test";
import { writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { underLoad } from "../scripts/timing";
import { root } from "./support";

/** TypeScript's `compilerOptions` a check is made under, over `base`. */
export type Setting = Record<string, unknown>;

/** What every check has: ES modules a bundler resolves, strict, nothing
 * emitted, the declarations checked too, `skipLibCheck` off; a setting may
 * turn one off, `noEmit: false` of a check whose JS is run after. */
const base: Setting = { module: "esnext", moduleResolution: "bundler", strict: true, noEmit: true, skipLibCheck: false };

/** The `@types` of the checkout's packages, what a check that reads Node's
 * or React's names is given. */
export const repoTypes: Setting = { typeRoots: [join(root, "node_modules/@types")] };

/** ES2022, an optional property taking `undefined` only where it says so:
 * what most declarations' tests check under, with what `lib` and `types`
 * each needs. */
export const es2022: Setting = { target: "es2022", exactOptionalPropertyTypes: true };

// The settings an app's TypeScript may have, each one that changes what a
// declaration means to it: a `.d.ts` means the same under every one, so an
// app's TypeScript isn't held to more, or less, than the Rust holds it to.
// What a check is made under where it names none.
const app = { jsx: "react-jsx", ...repoTypes };
export const appSettings: Setting[] = [
  { ...app, allowJs: true },
  // What Vite's template has, and the strictest of TypeScript's checks that
  // read a declaration: an optional property takes `undefined` only where it
  // says so.
  { ...app, verbatimModuleSyntax: true, isolatedModules: true, erasableSyntaxOnly: true, exactOptionalPropertyTypes: true },
  { ...app, module: "nodenext", moduleResolution: "nodenext", allowJs: true },
];

/** What a check found: each line of an error TypeScript wrote, with how
 * it ended, `code`, and all it wrote, `out`, for a test that reads them,
 * each always there but not enumerable, so `toEqual` compares the lines
 * alone (and typed optional, so `toEqual([])` reads as it means). */
export type Checked = string[] & { code?: number; out?: string };

/** TypeScript's errors in `files` of `dir`, which import a module's JS,
 * one or several, under each of `settings`, `appSettings` where it's given
 * none: the same places under each, the first's returned. Each config is
 * named for its first file and its setting, so checks of one directory
 * never meet. */
export function typeChecked(dir: string, files: string | string[], ...settings: Setting[]): Checked {
  const given = typeof files === "string" ? [files] : files;
  const each = (settings.length > 0 ? settings : appSettings).map((setting, at) => {
    const config = join(dir, `tsconfig-${basename(given[0]!)}-${at}.json`);
    writeFileSync(config, JSON.stringify({ compilerOptions: { ...base, ...setting }, files: given }));
    const ran = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", config], { cwd: dir, timeout: underLoad(60_000) });
    const out = ran.stdout.toString() + ran.stderr.toString();
    const errors = ran.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
    return Object.defineProperties(errors as Checked, { code: { value: ran.exitCode ?? 1 }, out: { value: out } });
  });
  const where = (errors: string[]) => errors.map((error) => error.slice(0, error.indexOf(":")));
  each.slice(1).forEach((errors, at) => expect({ [`setting ${at + 1}`]: where(errors) }).toEqual({ [`setting ${at + 1}`]: where(each[0]!) }));
  return each[0]!;
}

/** Whether every name `module`'s JS exports, as JS imports it (`export *`'s
 * and a namespace's too), is declared in its `.d.ts` as a value: a file of
 * `dir` importing and reading each, which TypeScript refuses where the
 * `.d.ts` lacks one; but a split function's body, `f$`, never declared. A checked forwarder in a re-export's place, or a
 * module of the crate's in a namespace's, declares what the JS exports
 * still (#2403 (Y), #2536): `[]` where each is. */
export async function exportsDeclared(dir: string, module: string, ...settings: Setting[]): Promise<Checked> {
  // But a split function's body, `f$` beside its entry `f`, which a Rust
  // call of another module calls and TypeScript never sees, its entry by its
  // name or the module's default (ADR 1193).
  const loaded: Record<string, unknown> = await import(join(dir, module));
  const all = Object.keys(loaded);
  const entry = (name: string) => all.includes(name) || (loaded.default as { name?: string } | undefined)?.name === name;
  const exported = all.filter((name) => !(name.endsWith("$") && entry(name.slice(0, -1))));
  const named = exported.filter((name) => name !== "default");
  const file = `exports-of-${module.replaceAll("/", "-").replace(/\.jsx?$/, "")}.ts`;
  const imported = [...(exported.includes("default") ? ["theDefault"] : []), ...(named.length > 0 ? [`{ ${named.join(", ")} }`] : [])];
  const read = [...(exported.includes("default") ? ["theDefault"] : []), ...named];
  writeFileSync(join(dir, file), `import ${imported.join(", ")} from "./${module}";\n\nexport const all = [${read.join(", ")}];\n`);
  return typeChecked(dir, file, ...(settings.length > 0 ? settings : [es2022]));
}
