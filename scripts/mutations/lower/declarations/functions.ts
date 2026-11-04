// Mutations of src/lower/declarations/functions.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "receiver-declared-self",
    breaks: "a method's receiver is declared `self`, not its JS name, `counter` (#507)",
    file: "src/lower/declarations/functions.rs",
    find: "        let name = receiver.unwrap_or_else(|| ident.name.to_string());\n",
    replace: "        let name = ident.name.to_string();\n",
    tests: ["test/impl-constants.test.ts"],
  },
  {
    name: "parameter-declared-underscored",
    breaks:
      "a parameter's declaration keeps the `_` its JS name drops, `ref` declared `_ref`",
    file: "src/lower/declarations/functions.rs",
    find: "        let named = super::super::parameter_name(self.tcx, function, &name);\n",
    replace: "        let named = super::super::camel_case(&name);\n",
    tests: ["test/lowering-names.test.ts", "-t", "leading underscore"],
  },
  {
    name: "alias-param-unwritten",
    breaks:
      "a function's parameter written as an alias is its expansion, `(instance: HTMLElement | null) => void`, where it's `RefCallback<HTMLElement>`",
    file: "src/lower/declarations/functions.rs",
    find: "                let written = decl.and_then(|decl| decl.inputs.get(i));\n",
    replace: "                let written = None;\n",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "aliases as the Rust names them",
    ],
  },
  {
    name: "declared-param-rust-name",
    breaks:
      "a parameter named `new` is declared `new`, which TypeScript doesn't read, where the JS names it `new$`",
    file: "src/lower/declarations/functions.rs",
    find: "        crate::names::js_ident(&named)\n    }\n",
    replace: "        named\n    }\n",
    tests: ["test/declarations.test.ts", "-t", "as its JS does"],
  },
  {
    name: "trailing-option-param-required",
    breaks:
      "a Rust function's trailing `Option` parameter is declared required, `shown(name: string | null | undefined)`, so a TypeScript caller must give it",
    file: "src/lower/declarations/functions.rs",
    find: '                json!({ "name": name, "optional": i >= optional_from && !rest, "rest": rest, "type": ty })\n',
    replace:
      '                json!({ "name": name, "optional": false, "rest": rest, "type": ty })\n',
    tests: [
      "test/declarations.test.ts",
      "-t",
      "declarations take JS's null for an Option",
    ],
  },
  {
    name: "written-return-expanded",
    breaks:
      "a function's result written as an alias is what it stands for, `TocItem[][]`, where it's `Toc[]`",
    file: "src/lower/declarations/functions.rs",
    find: "            (false, None) => self.as_written(written, |out| out.ts(output)),",
    replace: "            (false, None) => self.ts(output),",
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
    find: "                let rest = rest == Some(i);\n",
    replace: "                let rest = rest == Some(i) && false;\n",
    tests: [
      "test/rest-parameters.test.ts",
      "-t",
      "rest slice is JS's rest parameter",
    ],
  },
  {
    name: "nullable-result-declared-undefined",
    breaks:
      "a nullable function's result is declared `string | null | undefined`, though its `None` is `null`, never `undefined`",
    file: "src/lower/declarations/functions.rs",
    find: "            .then(|| self.option(output))\n",
    replace:
      "            .then(|| self.option(output))\n            .filter(|_| false)\n",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "a nullable function's result is declared T or null",
    ],
  },
  {
    name: "nullable-param-declared-optional",
    breaks:
      "a `#[rust_js::nullable(..)]` parameter is declared `fiber?: Fiber | null | undefined`, though it's taken as `Fiber | null` (ADR 0861, amended)",
    file: "src/lower/declarations/functions.rs",
    find: "                let ty = match nullable(i).then(|| self.option(ty)).flatten() {\n",
    replace: "                let ty = match None {\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "nullable thread-local's None",
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
    find: "        if names.iter().any(Option::is_some) {\n            let function = self.tcx.erase_and_anonymize_regions(function);\n",
    replace: "        if false {\n            let function = self.tcx.erase_and_anonymize_regions(function);\n",
    tests: ["test/declarations-function-types.test.ts"],
  },
  {
    name: "filled-names-first-literal",
    breaks: "a field filled by two literals that name a parameter apart, `|dep, sub, ..|` and `|dep, target, ..|`, is named by the first's, `sub`",
    file: "src/lower/declarations/functions.rs",
    find: "                                    name.filter(|name| own.get(i).cloned().flatten().as_ref() == Some(name))\n",
    replace: "                                    name.filter(|_| i < usize::MAX)\n",
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
    find: "                let ident = ident.filter(|ident| ident.name != rustc_span::kw::Underscore)?;\n                Some(self.parameter_js_name(function, ident, inputs.map(|inputs| inputs[i])))\n",
    replace: "                let ident = (*ident)?;\n                Some(self.parameter_js_name(function, ident, inputs.map(|inputs| inputs[i])))\n",
    tests: ["test/declarations-function-types.test.ts"],
  },
  {
    name: "filled-parameter-name-unmade",
    breaks: "a filled parameter keeps Rust's spelling, `inner_write` and `_sub`, where its JS's is `innerWrite` and `sub`",
    file: "src/lower/declarations/functions.rs",
    find: "                Some(self.parameter_js_name(function, ident, inputs.map(|inputs| inputs[i])))\n",
    replace: "                Some(ident.name.to_string())\n",
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
    name: "optional-from-of-trailing-all",
    breaks: "a parameter `Option` before one that isn't is declared optional, `(value?: number | null, value2: number)`, which TypeScript refuses",
    file: "src/lower/declarations/functions.rs",
    find: "            .take_while(|&(i, &ty)| is_option(ty) && !nullable(i))\n",
    replace: "            .filter(|&(i, &ty)| is_option(ty) && !nullable(i))\n",
    tests: ["test/declarations-function-types.test.ts"],
  },
  {
    name: "ungiven-fact-declared",
    breaks: "a function whose JS takes a fact of its type parameter, `size(TSize)`, is declared `size<T>(): number`, which gives a JS caller `undefined` (#549)",
    file: "src/lower/declarations/functions.rs",
    find: "        if let Some(&(index, fact)) = self.type_facts.get(&function).and_then(|facts| facts.first()) {\n",
    replace: "        if let Some(&(index, fact)) = self.type_facts.get(&function).and_then(|facts| facts.first()).filter(|_| false) {\n",
    tests: ["test/declarations-ungiven.test.ts"],
  },
  {
    name: "ungiven-dictionary-declared",
    breaks: "a function whose JS takes a bound's dictionary of a trait that isn't declared, `shown(value, TDisplay)`, is declared, which throws for a JS caller that gives none (#549)",
    file: "src/lower/declarations/functions.rs",
    find: "        if let Some(bound) = bounds.iter().find(|bound| !self.declares_dictionary(bound.def_id)) {\n",
    replace: "        if let Some(bound) = bounds.iter().filter(|_| false).find(|bound| !self.declares_dictionary(bound.def_id)) {\n",
    tests: ["test/declarations-ungiven.test.ts"],
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
    find: "            params.push(json!({ \"name\": name, \"optional\": false, \"rest\": false, \"type\": ty }));\n",
    replace: "",
    tests: ["test/declarations-ungiven.test.ts", "-t", "type of its dictionaries"],
  },
  {
    name: "optional-before-dictionary",
    breaks: "a trailing `Option` before a dictionary is declared optional, `prefix?: string | null`, so a TypeScript caller that leaves it out gives its dictionary as `prefix` (#550)",
    file: "src/lower/declarations/functions.rs",
    find: "            false => sig.inputs().len(),\n",
    replace: "            false => optional_from(tcx, sig.inputs(), rest, nullable),\n",
    tests: ["test/declarations-ungiven.test.ts", "-t", "type of its dictionaries"],
  },
  {
    name: "trait-method-declared-its-dictionary",
    breaks: "a declared trait's method is declared taking its own trait's dictionary after its arguments, `area(self: Self, SelfShape: Shape<Self>)`, which its dictionary's entry doesn't (#550)",
    file: "src/lower/declarations/functions.rs",
    find: "        let evidence = match own && !super::super::bindings::is_binding(self.tcx, def_id) {\n",
    replace: "        let evidence = match !super::super::bindings::is_binding(self.tcx, def_id) {\n",
    tests: ["test/declarations-ungiven.test.ts", "-t", "type of its dictionaries"],
  },
];
