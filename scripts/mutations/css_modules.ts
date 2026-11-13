// Mutations of src/css_modules.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "css-module-icss-unrefused",
    breaks:
      "a CSS Module of ICSS's `:export { primary: red }` gives its keys without `primary`, which css-loader exports, so a binding of it is refused or taken by the bundler it's built with (#1617, ADR 1133)",
    file: "src/css_modules.rs",
    find: "    if let Some(name) = sheet.rules.0.iter().find_map(icss_rule) {\n",
    replace: "    if let Some(name) = sheet.rules.0.iter().find_map(icss_rule).filter(|_| false) {\n",
    tests: ["test/css-modules.test.ts", "-t", "ICSS"],
  },
  {
    name: "css-module-error-line-from-zero",
    breaks: "a CSS Module Lightning CSS can't read is said to be wrong a line before where it is, its line counted from 0 (ADR 1133)",
    file: "src/css_modules.rs",
    find: '        format!(", at line {}, column {}", at.line + 1, at.column)\n',
    replace: '        format!(", at line {}, column {}", at.line, at.column)\n',
    tests: ["test/css-modules.test.ts", "-t", "ICSS"],
  },
  {
    name: "css-binding-keyword-unraw",
    breaks: "`rust-js bindings` prints a getter of a class `type` as `fn type`, which doesn't compile (ADR 1133)",
    file: "src/css_modules.rs",
    find: "    Some(match KEYWORDS.contains(&name.as_str()) {\n",
    replace: "    Some(match false && KEYWORDS.contains(&name.as_str()) {\n",
    tests: ["test/css-modules.test.ts", "-t", "rust-js bindings"],
  },
  {
    name: "css-binding-shared-name-kept",
    breaks: "`rust-js bindings` prints two getters `a_b`, of `a-b` and of `aB`, which doesn't compile (ADR 1133)",
    file: "src/css_modules.rs",
    find: "        let kept = !shared.contains(name);\n",
    replace: "        let kept = true;\n",
    tests: ["test/css-modules.test.ts", "-t", "rust-js bindings"],
  },
  {
    name: "css-binding-numeral-named",
    breaks: "`rust-js bindings` prints a getter of a class `10` as `fn 10`, which doesn't compile (ADR 1133)",
    file: "src/css_modules.rs",
    find: "    if name.is_empty() || name.starts_with(|c: char| c.is_numeric()) || UNRAW.contains(&name.as_str()) {\n",
    replace: "    if name.is_empty() || UNRAW.contains(&name.as_str()) {\n",
    tests: ["test/css-modules.test.ts", "-t", "rust-js bindings"],
  },
  {
    name: "css-binding-of-a-plain-stylesheet",
    breaks: "`rust-js bindings site.css` prints a binding of a file that's no CSS Module, which the check then refuses (ADR 1133)",
    file: "src/css_modules.rs",
    find: '    if !crate::program::is_relative_specifier(specifier) || !specifier.ends_with(".module.css") {\n',
    replace: "    if !crate::program::is_relative_specifier(specifier) {\n",
    tests: ["test/css-modules.test.ts", "-t", "rust-js bindings"],
  },
];
