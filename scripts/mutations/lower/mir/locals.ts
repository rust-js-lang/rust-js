// Mutations of src/lower/mir/locals.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-mut-borrowed-unchanged",
    breaks: "`e += inc()` reads `e` before the closure that changes it runs",
    file: "src/lower/mir/locals.rs",
    find: "                            locals.changed[borrowed.local] = true;\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","assignment_order"],
  },
  {
    name: "mir-whole-mut-borrow-unwritten",
    breaks: "`Box::new_uninit()`, written through `write`, is a `const` assigned again",
    file: "src/lower/mir/locals.rs",
    find: "                            locals.writes[borrowed.local] += 1;\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","box_uninit"],
  },
  {
    name: "mir-pattern-names-renamed",
    breaks: "what a parameter's pattern binds is named again, unlike its pattern's names",
    file: "src/lower/mir/locals.rs",
    find: "            if let Some((_, name)) = named.iter().find(|(l, _)| *l == local) {\n",
    replace: "            if let Some((_, name)) = named.iter().find(|(l, _)| *l == local).filter(|_| false) {\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-await-result-named",
    breaks: "what `.await` gives is a variable of the desugaring's name, `result`",
    file: "src/lower/mir/locals.rs",
    find: "                && !info.source_info.span.is_desugaring(rustc_span::DesugaringKind::Await)\n",
    replace: "",
    tests: ["test/compiler.test.ts","-t","async code becomes"],
  },
  {
    name: "mir-patterned-folded",
    breaks: "a temporary a `let` pattern of one field takes apart is folded, `const getProps = f().get_props`",
    file: "src/lower/mir/locals.rs",
    find: "            && !state.patterned.contains(&local)\n",
    replace: "",
    tests: ["test/lowering-matches.test.ts", "-t", "pattern of one field"],
  },
  {
    "name": "mir-self-named-self",
    "breaks": "a method's `self` from MIR is named `self`, not after its type",
    "file": "src/lower/mir/locals.rs",
    "find": "ty::Adt(adt, _) if info.name.as_str() == \"self\" =>",
    "replace": "ty::Adt(adt, _) if false =>",
    "tests": ["test/lowering-names.test.ts", "-t", "a method's self from MIR is named after its type"]
  },
  {
    name: "mir-derived-params-unnamed",
    breaks: "MIR's derived impl's parameters are `_1`, not `point`",
    file: "src/lower/mir/locals.rs",
    find: "                && (!info.source_info.span.in_external_macro(sm) || body.local_kind(place.local) == LocalKind::Arg)\n",
    replace: "                && !info.source_info.span.in_external_macro(sm)\n",
    tests: ["test/lowering-strings.test.ts", "-t", "a writer from its MIR returns the string it writes"],
  },
  {
    name: "pin-macro-unnamed",
    breaks: "what `pin!` pins is named after its type, not the binding its `Pin` is",
    file: "src/lower/mir/locals.rs",
    find: "                user[pinned] = user[pin].clone();\n",
    replace: "",
    tests: ["test/snapshots.test.ts", "-t", "async"],
  },
];
