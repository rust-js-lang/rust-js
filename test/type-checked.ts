// TypeScript's check of what imports a module's JS and its declarations
// (ADR 0196), the same under each setting that changes what a declaration
// means to an app: the one check the declarations' test files share.

import { expect } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";

// The settings an app's TypeScript may have, each one that changes what a
// declaration means to it: a `.d.ts` means the same under every one, so an
// app's TypeScript isn't held to more, or less, than the Rust holds it to.
const settings: Record<string, object> = {
  bundler: { module: "esnext", moduleResolution: "bundler", allowJs: true },
  // What Vite's template has, and the strictest of TypeScript's checks that
  // read a declaration: an optional property takes `undefined` only where it
  // says so.
  strictest: { module: "esnext", moduleResolution: "bundler", verbatimModuleSyntax: true, isolatedModules: true, erasableSyntaxOnly: true, exactOptionalPropertyTypes: true },
  nodenext: { module: "nodenext", moduleResolution: "nodenext", allowJs: true },
};

/** TypeScript's errors in `file` of `dir`, which imports a module's JS, the same under each of `settings`; its declarations checked too, `skipLibCheck` off. */
export function typeChecked(dir: string, file: string): string[] {
  const each = Object.entries(settings).map(([name, options]) => {
    const config = join(dir, `tsconfig.${name}.json`);
    writeFileSync(config, JSON.stringify({
      compilerOptions: { ...options, jsx: "react-jsx", strict: true, noEmit: true, skipLibCheck: false, typeRoots: [join(root, "node_modules/@types")] },
      files: [file],
    }));
    const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", config], { cwd: dir });
    return { name, errors: checked.stdout.toString().split("\n").filter((line) => line.includes("error TS")) };
  });
  const where = (errors: string[]) => errors.map((error) => error.slice(0, error.indexOf(":")));
  for (const { name, errors } of each.slice(1)) {
    expect({ [name]: where(errors) }).toEqual({ [name]: where(each[0].errors) });
  }
  return each[0].errors;
}
