// Mutations of src/lower/declarations/written.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "written-keyword-reference",
    breaks:
      "js's `Unknown`, written `unknown`, is a type named `unknown`, not the keyword, so an optional one is `unknown | undefined`",
    file: "src/lower/declarations/written.rs",
    find: "        if from.is_none() && args.is_none() && KEYWORDS.contains(&name.as_str()) {",
    replace: "        if false {",
    tests: ["test/declarations.test.ts", "-t", "null for an Option"],
  },
  {
    name: "written-unread-accepted",
    breaks:
      "a `#[rust_js::types]` rust-js can't read compiles, and its type is declared of what it could, or `any`",
    file: "src/lower/declarations/written.rs",
    find: "            if let Some(why) = why {",
    replace: "            if let Some(why) = why.filter(|_| false) {",
    tests: ["test/declarations-written.test.ts", "-t", "can't read"],
  },
  {
    name: "written-object-unclosed",
    breaks:
      "an object type with no `}`, `{ run<T>(f: () => T): T`, is read as one, where it isn't TypeScript",
    file: "src/lower/declarations/written.rs",
    find: '                self.expect("}")?;\n                break;',
    replace: "                break;",
    tests: ["test/declarations-written.test.ts", "-t", "can't read"],
  },
  {
    name: "written-empty-parameters-grouped",
    breaks:
      "`() => T`, a function of no parameters, is read as a type in parentheses, so `ConsoleTask`'s `run<T>(f: () => T): T` isn't read",
    file: "src/lower/declarations/written.rs",
    find: '        if ahead.eat(")") || ahead.eat("...") {',
    replace: '        if ahead.eat("...") {',
    tests: ["test/declarations.test.ts", "-t", "a binding's object type"],
  },
  {
    name: "written-method-type-parameters-dropped",
    breaks:
      "a method's own type parameters, `run<T>`, are read and left out, so its `T` names nothing",
    file: "src/lower/declarations/written.rs",
    find: "        let type_parameters = self.type_parameters()?;\n        self.space();",
    replace:
      "        self.type_parameters()?;\n        let type_parameters = Vec::new();\n        self.space();",
    tests: ["test/declarations.test.ts", "-t", "a binding's object type"],
  },
  {
    name: "written-readonly-array-mutable",
    breaks:
      "`readonly T[]` is read as `T[]`, which TypeScript lets a callee change",
    file: "src/lower/declarations/written.rs",
    find: "            (true, Written::Array { readonly, .. }) => *readonly = true,",
    replace: "            (true, Written::Array { .. }) => {}",
    tests: ["test/declarations.test.ts", "-t", "a binding's object type"],
  },
  {
    name: "intersection-unread",
    breaks:
      "`#[rust_js::types = \"string[] & { .. }\"]` is an error, `&` unread, so the js crate's `RegExpMatch` doesn't build (#303)",
    file: "src/lower/declarations/written.rs",
    find: '        while self.eat("&") {\n',
    replace: '        while false && self.eat("&") {\n',
    tests: [
      "test/declarations-written.test.ts",
      "-t",
      "intersection is declared as written",
    ],
  },
  {
    name: "written-readonly-index-mutable",
    breaks:
      "`readonly [key: string]: T`, Flow's `{+[key: string]: T}`, is read as `[key: string]: T`, which TypeScript lets a callee change (rd538)",
    file: "src/lower/declarations/written.rs",
    find: "                readonly: readonly_index,",
    replace: "                readonly: false,",
    tests: ["test/declarations-libraries.test.ts", "-t", "declared as written"],
  },
  {
    name: "function-type-is-a-class",
    breaks:
      "TypeScript's `Function`, `js::Function`'s, isn't a function's type, so `Kind::Function` of one is refused as a class `instanceof Function` would test, which a frame's function isn't (ADR 1082)",
    file: "src/lower/declarations/written.rs",
    find: '            } => name == "Function",',
    replace: '            } => name == "Function" && false,',
    tests: ["test/js-functions.test.ts", "-t", "is a js::Function"],
  },
  {
    name: "function-type-is-any-reference",
    breaks:
      "any type a binding names is a function's, `HTMLElement`'s too, so `Kind::Object` of an element is `typeof === \"function\"`, which no element is (ADR 1082)",
    file: "src/lower/declarations/written.rs",
    find: '            } => name == "Function",',
    replace: '            } => !name.is_empty(),',
    tests: [
      "test/compiler.test.ts",
      "-t",
      "an untagged enum's JS object is tested by the class TypeScript names",
    ],
  },
];
