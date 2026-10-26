// The runtime as an app's bundler may transpile it (ADR 0919): the JS a
// program imports goes through the app's tooling, and React's Jest runs
// Babel's loose spread over it. A helper that steps a string or a set by
// syntax a common loose transform rewrites gives another answer there.

import { beforeAll, expect, test } from "bun:test";
import { mkdtempSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { parseSync, traverse, transformSync, type NodePath, type types as t } from "@babel/core";
import { buildCompiler, compiler, root, run } from "./support";

beforeAll(buildCompiler, 600_000);

// React's Jest config's loose transforms, and for-of's loose form.
const loose = [
  ["@babel/plugin-transform-spread", { loose: true }],
  ["@babel/plugin-transform-destructuring", { loose: true }],
  ["@babel/plugin-transform-for-of", { loose: true }],
  ["@babel/plugin-transform-template-literals", { loose: true }],
];

// The compiler's runtime, as it is and as the loose transforms leave it.
async function runtimes() {
  const runtime = run([compiler, "--runtime-module"]);
  const dir = mkdtempSync(join(tmpdir(), "rust-js-loose-"));
  writeFileSync(join(dir, "native.mjs"), runtime);
  writeFileSync(join(dir, "loose.mjs"), transformSync(runtime, { babelrc: false, configFile: false, plugins: loose })!.code!);
  return [await import(join(dir, "native.mjs")), await import(join(dir, "loose.mjs"))];
}

// Each helper that counts, cuts or steps text by its chars, or steps a
// map or a set, given what it's given in a program: the same answer
// through the loose transforms as without them.
test("the runtime's helpers answer the same through Babel's loose transforms", async () => {
  const [native, rt] = await runtimes();
  const cases: [string, unknown[]][] = [
    ["$pad", ["dalala", 6, ">", "0"]],
    ["$pad", ["1", 5, ">", "0"]],
    ["$pad", ["a😀", 4, "^", "*"]],
    ["$pad", ["😀😀", 3, "<"]],
    ["$padIntegral", [false, "0x", "ff", { width: 8, zero: true, alternate: true }]],
    ["$formatted", ["a😀bc", { precision: 2 }]],
    ["$formatted", ["a😀bc", { precision: 2, width: 4, align: "Right" }]],
    ["$split", ["a😀b", ""]],
    ["$splitN", ["a😀b", 2, ""]],
    ["$parseChar", ["😀"]],
    ["$parseChar", ["ab"]],
    ["$jsonChar", ["😀"]],
    ["$jsonOneOf", [[["b😀", "a"], "c"]]],
    ["$reverseBits", [1, 8, false]],
    ["$reverseBits", [1n, 64, false]],
    ["$debug", [new Map([["a", 1]])]],
    ["$debug", [new Set([1, 2])]],
    ["$eq", [{ a: 1 }, { a: 1, b: undefined }]],
    ["$eq", [{ a: 1 }, { a: 2 }]],
    ["$setAlgebra", [new Set([1, 2]), new Set([2, 3]), "union"]],
    ["$setAlgebra", [new Set([1, 2]), new Set([2, 3]), "symmetric_difference"]],
  ];
  for (const [name, args] of cases) {
    const answer = (helpers: Record<string, (...args: unknown[]) => unknown>) => {
      try {
        return helpers[name](...args);
      } catch (e) {
        return `threw ${(e as Error).message}`;
      }
    };
    expect([name, answer(rt)]).toEqual([name, answer(native)]);
  }
  expect(rt.$pad("dalala", 6, ">", "0")).toBe("dalala");
  expect(rt.$pad("a😀", 4, "^", "*")).toBe("*a😀*");
  expect(rt.$split("a😀b", "")).toEqual(["", "a", "😀", "b", ""]);
  // A splice's replacement, any iterable, as a `Vec`'s `splice` takes one.
  const items = [1, 2, 3];
  expect(rt.$splice(items, 1, 2, new Set([7, 8]))).toEqual([2]);
  expect(items).toEqual([1, 7, 8, 3]);
});

// What makes those answers hold, in the runtime's source: a spread, of an
// array (`[...x]`, `f(a, ...x)`) or as a call's one argument (`f(...x)`),
// is of what's evidently an array, since Babel's loose spread writes
// `[].concat(x)` and `f.apply(null, x)`, which take any other value whole:
// a string as one item, a set as none. A string's chars are
// `Array.from(s)`, a set's or a map's items `Array.from(m)`.
test("the runtime spreads only what's evidently an array", () => {
  const dir = join(root, "src", "runtime");
  const found: string[] = [];
  for (const file of readdirSync(dir).filter((f) => f.endsWith(".js"))) {
    const source = readFileSync(join(dir, file), "utf8");
    const ast = parseSync(source, { babelrc: false, configFile: false, sourceType: "script" });
    traverse(ast!, {
      SpreadElement(path: NodePath<t.SpreadElement>) {
        if (!path.parentPath.isArrayExpression() && !path.parentPath.isCallExpression() && !path.parentPath.isNewExpression()) return;
        if (!evidentlyArray(path.get("argument"))) {
          found.push(`${file}:${path.node.loc!.start.line}: ...${source.slice(path.node.argument.start!, path.node.argument.end!)}`);
        }
      },
    });
  }
  expect(found).toEqual([]);
});

const ARRAY_STATICS = new Set(["Array.from", "Array.of", "Object.keys", "Object.values", "Object.entries"]);
// Methods only an array has, or whose result is always one.
const ARRAY_METHODS = new Set(["map", "filter", "flatMap", "split", "toSorted", "toReversed", "toSpliced"]);

function evidentlyArray(path: NodePath<t.Node>): boolean {
  if (path.isArrayExpression()) return true;
  if (path.isConditionalExpression()) return evidentlyArray(path.get("consequent")) && evidentlyArray(path.get("alternate"));
  if (path.isParenthesizedExpression()) return evidentlyArray(path.get("expression"));
  if (path.isCallExpression()) {
    const callee = path.get("callee");
    if (!callee.isMemberExpression() || callee.node.computed) return false;
    const name = (callee.node.property as t.Identifier).name;
    const object = callee.get("object");
    if (object.isIdentifier() && ARRAY_STATICS.has(`${object.node.name}.${name}`)) return true;
    if (ARRAY_METHODS.has(name)) return true;
    // A copy of an array is one, `items.slice(1)`; of a string, it's not.
    return name === "slice" && evidentlyArray(object);
  }
  if (path.isIdentifier()) {
    const binding = path.scope.getBinding(path.node.name);
    if (binding === undefined) return false;
    // A rest parameter, `(...args)`.
    if (binding.kind === "param" && binding.path.parentPath?.isRestElement()) return true;
    if (binding.path.isRestElement()) return true;
    // A `const` made of one, `const parts = s.split(p)`.
    if (binding.kind === "const" && binding.path.isVariableDeclarator() && binding.path.get("id").isIdentifier()) {
      const init = binding.path.get("init");
      return init.node !== null && init.node !== undefined && evidentlyArray(init as NodePath<t.Node>);
    }
    return false;
  }
  return false;
}
