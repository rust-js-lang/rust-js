// The TypeScript Member protocol, loaded in the isolated source workspace.
import type { Mutation } from "../../mutations";
const tests = ["test/typescript.test.ts", "-t", "construct signatures retain"];
export const mutations: Mutation[] = [
  {
    name: "const-type-parameter-unprinted",
    breaks: "a `const` type parameter is printed without its modifier, so a caller's literals widen (ADR 1164)",
    file: "typescript/index.js",
    find: "          p.const ? [f.createToken(SyntaxKind.ConstKeyword)] : undefined,\n",
    replace: "          undefined,\n",
    tests: ["test/typescript.test.ts", "-t", "a conditional type, and an object alias"],
  },
  {
    name: "type-parameter-default-unprinted",
    breaks: "a type parameter's default isn't printed, so `CommitDetails` without its reason is an error, where upstream's `Reason extends string = string` takes any (ADR 1164)",
    file: "typescript/index.js",
    find: "          p.default ? typeNode(p.default) : undefined,\n",
    replace: "          undefined,\n",
    tests: ["test/typescript.test.ts", "-t", "a conditional type, and an object alias"],
  },
  {
    name: "distributive-members-undocumented",
    breaks: "a distributive alias's members print no doc comment, where upstream's `ChangeDetails` carries each for a caller's editor (ADR 1164)",
    file: "typescript/index.js",
    find: 'const object = aliased?.kind === "conditional" ? aliased.whenTrue : aliased;',
    replace: "const object = aliased;",
    tests: ["test/typescript.test.ts", "-t", "a conditional type, and an object alias"],
  },
  {
    name: "object-alias-members-undocumented",
    breaks: "a type alias's of an object prints no member's doc comment, where an interface's does: a type map's keys lose theirs (ADR 1164)",
    file: "typescript/index.js",
    find: 'const objectAlias = object?.kind === "object";',
    replace: "const objectAlias = false;",
    tests: ["test/typescript.test.ts", "-t", "a conditional type, and an object alias"],
  },
  {
    name: "conditional-type-unprinted",
    breaks: "a conditional type is printed as its check alone, so `EventOf<R>` is `R`, any key's event its name (ADR 1164)",
    file: "typescript/index.js",
    find: "      return f.createConditionalTypeNode(\n",
    replace: "      return typeNode(t.check);\n      return f.createConditionalTypeNode(\n",
    tests: ["test/typescript.test.ts", "-t", "a conditional type, and an object alias"],
  },
  {
    name: "typescript-construct-unread",
    breaks:
      "construct signatures remain opaque source instead of structured parameters and return types",
    file: "typescript/index.js",
    find: "case SyntaxKind.ConstructSignature:",
    replace: "case SyntaxKind.NeverKeyword:",
    tests,
  },
  {
    name: "typescript-construct-is-call",
    breaks: "a construct signature is printed as a call signature, losing new",
    file: "typescript/index.js",
    find: "return f.createConstructSignatureDeclaration(",
    replace: "return f.createCallSignatureDeclaration(",
    tests,
  },
  {
    name: "typescript-member-unknown-accepted",
    breaks: "the shared Member guard accepts an unknown protocol kind silently",
    file: "typescript/index.js",
    find: "throw new Error(`a member isn't one TypeScript is given: ${JSON.stringify(m)}`);",
    replace: "break;",
    tests,
  },
  {
    "name": "typescript-computed-key-read-as-string",
    "breaks": "a computed symbol becomes a quoted bracketed string key",
    "file": "typescript/index.js",
    "find": "return { computed: path(name.expression) };",
    "replace": "return sourceOf(name, text);",
    "tests": [
      "test/typescript-symbols.test.ts"
    ]
  },
  {
    "name": "typescript-computed-key-printed-as-string",
    "breaks": "the writer turns the model’s symbol key into a different string property",
    "file": "typescript/index.js",
    "find": "    return f.createComputedPropertyName(\n      rest.reduce(\n        (object, part) =>\n          f.createPropertyAccessExpression(object, undefined, id(part), NodeFlags.None),\n        id(first),\n      ),\n    );",
    "replace": "    return string(\"[\" + name.computed + \"]\");",
    "tests": [
      "test/typescript-symbols.test.ts"
    ]
  },
  {
    "name": "typescript-computed-key-loses-owner",
    "breaks": "a qualified symbol key loses its receiver path",
    "file": "typescript/index.js",
    "find": "path(node.expression) + \".\" + node.name.text",
    "replace": "node.name.text",
    "tests": [
      "test/typescript-symbols.test.ts"
    ]
  },
  {
    "name": "typescript-computed-key-unchecked-expression",
    "breaks": "the symbol-path model silently admits an unsupported expression",
    "file": "typescript/index.js",
    "find": "if (!parts.every((part) => isIdentifierText(part))) {",
    "replace": "if (false) {",
    "tests": [
      "test/typescript-symbols.test.ts"
    ]
  },
  {
    "name": "typescript-inventory-literal-collides-with-symbol",
    "breaks": "inventory labels conflate a bracketed string property with a symbol",
    "file": "typescript/index.js",
    "find": "return name.startsWith(\"[\") || name.startsWith(\"\\\"\") ? JSON.stringify(name) : name;",
    "replace": "return name;",
    "tests": [
      "test/typescript-symbols.test.ts"
    ]
  },
  {
    "name": "typescript-computed-key-optional-path-narrowed",
    "breaks": "an optional computed access is silently turned into an unconditional symbol path",
    "file": "typescript/index.js",
    "find": "node.kind === SyntaxKind.PropertyAccessExpression && !node.questionDotToken",
    "replace": "node.kind === SyntaxKind.PropertyAccessExpression",
    "tests": [
      "test/typescript-symbols.test.ts"
    ]
  },
  {
    "name": "typescript-parameter-default-unread",
    "breaks": "a generic declaration loses its default while being read",
    "file": "typescript/index.js",
    "find": "...(p.defaultType && { default: readType(p.defaultType, text) }),",
    "replace": "...{},",
    "tests": [
      "test/typescript-defaults.test.ts"
    ]
  },
  {
    "name": "typescript-parameter-default-is-constraint",
    "breaks": "a literal default is replaced by the broader generic constraint",
    "file": "typescript/index.js",
    "find": "default: readType(p.defaultType, text)",
    "replace": "default: readType(p.constraint ?? p.defaultType, text)",
    "tests": [
      "test/typescript-defaults.test.ts"
    ]
  },
  {
    "name": "typescript-parameter-constraint-unread",
    "breaks": "preserving a default loses its generic constraint",
    "file": "typescript/index.js",
    "find": "...(p.constraint && { constraint: readType(p.constraint, text) }),",
    "replace": "...{},",
    "tests": [
      "test/typescript-defaults.test.ts"
    ]
  },
];
