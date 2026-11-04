// Mutations of src/lower/declarations/functions.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "alias-param-unwritten",
    breaks:
      "a function's parameter written as an alias is its expansion, `(instance: HTMLElement | null) => void`, where it's `RefCallback<HTMLElement>`",
    file: "src/lower/declarations/functions.rs",
    find: "            Origin::Rust(i) => decl.and_then(|decl| decl.inputs.get(i)),\n",
    replace: "            Origin::Rust(_) => None,\n",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "aliases as the Rust names them",
    ],
  },
  {
    name: "written-return-expanded",
    breaks:
      "a function's result written as an alias is what it stands for, `TocItem[][]`, where it's `Toc[]`",
    file: "src/lower/declarations/functions.rs",
    find: "        let returns = match self.form_ts(signature.result, written) {\n",
    replace: "        let returns = match self.form_ts(signature.result, None) {\n",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "declarations name a type alias",
    ],
  },
  {
    name: "rest-declared-array",
    breaks:
      "a `.d.ts` declares a rest parameter an array argument, `xs: number[]`, which TypeScript callers give whole",
    file: "src/lower/declarations/functions.rs",
    find: "        let rest = matches!(param.form, Form::Rest(_));\n",
    replace: "        let rest = matches!(param.form, Form::Rest(_)) && false;\n",
    tests: [
      "test/rest-parameters.test.ts",
      "-t",
      "rest slice is JS's rest parameter",
    ],
  },
  {
    name: "class-trait-methods-undeclared",
    breaks:
      "a class's `.d.ts` leaves out the methods a trait of classes gives its prototype (rd283)",
    file: "src/lower/declarations/functions.rs",
    find: "            if crate::lower::classes::trait_impl_class(self.tcx, imp) != Some(class) && !extending {\n",
    replace: "            if true {\n",
    tests: ["test/classes-traits.test.ts"],
  },
  {
    name: "class-trait-field-method-declared",
    breaks:
      "a class's `.d.ts` declares a trait of classes' method that gives a field, `internalRoot()`, which its prototype hasn't (rd304)",
    file: "src/lower/declarations/functions.rs",
    find: "                if given.is_some_and(|own| crate::lower::classes::field_accessor(self.tcx, own).is_some()) {\n",
    replace: "                if given.is_some_and(|own| crate::lower::classes::field_accessor(self.tcx, own).is_none()) {\n",
    tests: ["test/classes-traits.test.ts", "-t", "prototype's"],
  },
  {
    name: "subclass-declared-of-the-traits-generics",
    breaks: "a class component's `componentDidUpdate` is declared of the trait's `P`, `prevProps: P`, no type the module has",
    file: "src/lower/declarations/functions.rs",
    find: "                let declaring = given.filter(|_| extending).unwrap_or(item);\n",
    replace: "                let declaring = item;\n",
    tests: [
      "test/class-components.test.ts",
      "-t",
      "an ES class React renders"
    ]
  },
  {
    name: "subclass-declares-the-traits-defaults",
    breaks: "a class component declares every method of react's `Component`, `props()` and lifecycle methods it doesn't write",
    file: "src/lower/declarations/functions.rs",
    find: "            let items: Vec<DefId> = match extending {\n",
    replace: "            let items: Vec<DefId> = match false {\n",
    tests: [
      "test/class-components.test.ts",
      "-t",
      "an ES class React renders"
    ]
  },
  {
    name: "filled-names-unpushed",
    breaks: "a `dyn Fn` field's parameters are named by their types, `link: (node, node2, value)`, where every literal fills it with `|dep, sub, version|` (#543)",
    file: "src/lower/declarations/functions.rs",
    find: "        let names = self.filled_names(field);\n        if names.iter().any(Option::is_some) {\n",
    replace: "        let names = self.filled_names(field);\n        if false {\n",
    tests: ["test/declarations-function-types.test.ts"],
  },
  {
    name: "filled-names-first-literal",
    breaks: "a field filled by two literals that name a parameter apart, `|dep, sub, ..|` and `|dep, target, ..|`, is named by the first's, `sub`",
    file: "src/lower/declarations/functions.rs",
    find: "                    .map(|(i, name)| name.filter(|name| own.get(i).cloned().flatten().as_ref() == Some(name)))\n",
    replace: "                    .map(|(i, name)| name.filter(|_| i < usize::MAX))\n",
    tests: ["test/declarations-function-types.test.ts"],
  },
  {
    name: "filler-closure-unread",
    breaks: "a field filled with a closure, `Rc::new(move |dep, sub, version| ..)`, is named by its types, as one filled with what names none",
    file: "src/lower/declarations/functions.rs",
    find: "        ty::Closure(def_id, _) | ty::FnDef(def_id, _) => Some(*def_id),\n",
    replace: "        ty::FnDef(def_id, _) => Some(*def_id),\n",
    tests: ["test/declarations-function-types.test.ts"],
  },
  {
    name: "filler-block-unpeeled",
    breaks: "a field filled by a block, `{ let s = ..; Rc::new(|..| ..) }`, whose type is the field's once coerced, is named by its types",
    file: "src/lower/declarations/functions.rs",
    find: "                    && let Some(tail) = block.expr\n",
    replace: "                    && let Some(tail) = block.expr.filter(|_| false)\n",
    tests: ["test/declarations-function-types.test.ts"],
  },
  {
    name: "filled-underscore-named",
    breaks: "a parameter every literal writes `_` is declared `_: number`, where it names none",
    file: "src/lower/declarations/functions.rs",
    find: "                let ident = ident.filter(|ident| ident.name != rustc_span::kw::Underscore)?;\n                Some(js_parameter_name(\n",
    replace: "                let ident = (*ident)?;\n                Some(js_parameter_name(\n",
    tests: ["test/declarations-function-types.test.ts"],
  },
  {
    name: "filled-parameter-name-unmade",
    breaks: "a filled parameter keeps Rust's spelling, `inner_write` and `_sub`, where its JS's is `innerWrite` and `sub`",
    file: "src/lower/declarations/functions.rs",
    find: "                Some(js_parameter_name(\n                    self.tcx,\n                    function,\n                    ident,\n                    inputs.map(|inputs| inputs[i]),\n                ))\n",
    replace: "                Some(ident.name.to_string()).filter(|_| inputs.is_some() || i > 0 || true)\n",
    tests: ["test/declarations-function-types.test.ts"],
  },
  {
    name: "function-names-first-pushed",
    breaks: "a fn pointer's written names, `fn(count: u32, _sum: u32)`, lose to its literals' `total(count, total)`",
    file: "src/lower/declarations/functions.rs",
    find: "        (self.function_names.iter().rev())\n            .find(|(named, _)| *named == ty)\n",
    replace: "        (self.function_names.iter())\n            .find(|(named, _)| *named == ty)\n",
    tests: ["test/declarations-function-types.test.ts"],
  },
  {
    name: "written-fn-pointer-in-path-unread",
    breaks: "a fn pointer in a path's arguments, `Class<fn(name: &str, ..)>`, is named by its types, `(value, value2)`",
    file: "src/lower/declarations/functions.rs",
    find: "                        self.written_function_names(ty.as_unambig_ty(), named);\n",
    replace: "                        let _ = ty;\n",
    tests: ["test/declarations-function-types.test.ts"],
  },
  {
    name: "subclass-getter-declared-a-method",
    breaks: "a getter of the binding's class a class extends is declared a method, `type(): string`, where its JS is `get type()` (rd679)",
    file: "src/lower/declarations/functions.rs",
    find: "                let declared = match getter {\n",
    replace: "                let declared = match false {\n",
    tests: ["test/binding-subclasses.test.ts", "-t", "is its class's getter"],
  },
  {
    name: "dictionary-parameter-undeclared",
    breaks: "a function's bound's dictionary of a declared trait, `totalArea(shapes, SShape)`'s, isn't declared, so a TypeScript caller gives none and `SShape.area` throws (#550)",
    file: "src/lower/declarations/functions.rs",
    find: "            Form::Dictionary(bound) => (self.dictionary_type(bound), false),\n",
    replace: "            Form::Dictionary(_) => return None,\n",
    tests: ["test/declarations-ungiven.test.ts", "-t", "type of its dictionaries"],
  },
  {
    name: "function-owner-of-its-item",
    breaks: "a method's `&mut F` of its own `F: FnMut` is asked of its type's context, which has no such bound, and declared a box, `f: { value: F }`, where its JS takes the function (#619)",
    file: "src/lower/declarations/functions.rs",
    find: "        let outer = std::mem::replace(&mut self.owner, def_id);\n",
    replace: "        let outer = self.owner;\n",
    tests: ["test/declarations-ungiven.test.ts", "-t", "box is declared"],
  },
  {
    name: "unnamed-filler-names-none",
    breaks: "what's given that names nothing, a host's `js::cast`, a parameter, keeps a function type from being named as the functions given with it name it (#630)",
    file: "src/lower/declarations/functions.rs",
    find: "        for &given in given.iter().flatten() {\n            let own = self.parameter_names(given);\n",
    replace: "        for &given in given {\n            let own = given.map(|given| self.parameter_names(given)).unwrap_or_default();\n",
    tests: ["test/declarations-function-types.test.ts"],
  },
  {
    name: "value-else-unread",
    breaks: "a static's value's `else`, `&schedule_callback`, isn't one of the functions it may be, so it's named as nothing names it (#630)",
    file: "src/lower/declarations/functions.rs",
    find: "            hir::ExprKind::If(_, then, otherwise) => values.extend(std::iter::once(then).chain(otherwise)),\n",
    replace: "            hir::ExprKind::If(_, then, _) => values.push(then),\n",
    tests: ["test/declarations-function-types.test.ts", "-t", "the functions its value may be"],
  },
  {
    name: "thread-local-init-unfound",
    breaks: "a thread-local's value is looked for in its key's own body, std's, not its `init`, so its `dyn Fn` is named by its types (#630)",
    file: "src/lower/declarations/functions.rs",
    find: "    let owner = init.unwrap_or(local);\n",
    replace: "    let owner = init.filter(|_| false).unwrap_or(local);\n",
    tests: ["test/declarations-function-types.test.ts", "-t", "the functions its value may be"],
  },
  {
    name: "fn-param-names-collide",
    breaks:
      "two parameters of one type are both `value`, which TypeScript refuses as a duplicate",
    file: "src/lower/declarations/functions.rs",
    find: "                let name = if taken == 0 {\n",
    replace: "                let name = if true || taken == 0 {\n",
    tests: ["test/declarations.test.ts", "-t", "by what it takes"],
  },
  {
    name: "function-type-trailing-option-required",
    breaks: "a function type's trailing `Option`, `unlink: (link, sub?)`, is declared required, though a caller leaves it out (#544)",
    file: "src/lower/declarations/functions.rs",
    find: "            true => optional_from(self.tcx, inputs, rest.map(|(at, _)| at), |_| false),\n",
    replace: "            true => count,\n",
    tests: ["test/declarations-function-types.test.ts"],
  },
  {
    name: "filled-name-counted-apart",
    breaks: "a parameter named by its type beside one its literals name alike, `(dep, node, value)`, is numbered as if none were named, the same; a given name is never pushed twice",
    file: "src/lower/declarations/functions.rs",
    find: "                    names.push(name.clone());\n                    name\n",
    replace: "                    name\n",
    tests: ["test/declarations-function-types.test.ts"],
  },
];
