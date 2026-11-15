// The TypeScript Member protocol, loaded in the isolated source workspace.
import type { Mutation } from "../../mutations";
const tests = ["test/typescript.test.ts", "-t", "construct signatures retain"];
export const mutations: Mutation[] = [
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
];
