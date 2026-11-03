// Mutations of src/lower/analysis/validation.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "flattened-variant-field-allowed",
    breaks:
      "an enum variant's flattened field is made nested, without an error",
    file: "src/lower/analysis/validation.rs",
    find: "        if tcx.def_kind(id) == DefKind::Enum {",
    replace: "        if false && tcx.def_kind(id) == DefKind::Enum {",
    tests: ["test/lowering.test.ts", "-t", "flattened field of an enum"],
  },
  {
    name: "flattened-base-refused",
    breaks:
      "a flattened field read whole as an update's base, `..props.html`, is refused, where it's the object its parent is",
    file: "src/lower/analysis/validation.rs",
    find: "                        && !bases.contains(&e)\n",
    replace: "                        && true\n",
    tests: [
      "test/jsx-props.test.ts",
      "-t",
      "flattened props with a field of them set",
    ],
  },
  {
    name: "init-of-nested-function",
    breaks:
      "a function a thread-local's `init` makes is taken for the `init`, its value the thread-local's",
    file: "src/lower/analysis/validation.rs",
    find: "        if matches!(tcx.def_kind(p), DefKind::Fn | DefKind::AssocFn | DefKind::Closure) {\n            return None;\n        }\n",
    replace: "",
    tests: [
      "test/jsx-children.test.ts",
      "-t",
      "a function a block makes and gives",
    ],
  },
  {
    name: "flattened-reference-refused",
    breaks: "a flattened field that's a reference to a struct is refused",
    file: "src/lower/analysis/validation.rs",
    find: "            let object = match ty.peel_refs().kind() {",
    replace: "            let object = match ty.kind() {",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "flattened field made outside JSX",
    ],
  },
  {
    name: "flattened-read-through-deref",
    breaks:
      "reading through a flattened reference, `report.item.line`, is refused as a read of it whole",
    file: "src/lower/analysis/validation.rs",
    find: "                    while let ExprKind::Deref { arg } = thir[lhs].kind {\n",
    replace:
      "                    while let ExprKind::Deref { arg } = thir[lhs].kind && false {\n",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "flattened field made outside JSX",
    ],
  },
  {
    name: "mixed-read-through",
    breaks:
      "a struct of two rests is read through one of them, whose keys JS mixed",
    file: "src/lower/analysis/validation.rs",
    find: "                    if is_mixed(thir[lhs].ty) && bindings::is_flatten_field(tcx, thir[lhs].ty, name.as_usize()) =>",
    replace:
      "                    if false && bindings::is_flatten_field(tcx, thir[lhs].ty, name.as_usize()) =>",
    tests: ["test/jsx-props.test.ts", "-t", "two flattened fields"],
  },
  {
    name: "mixed-taken-apart",
    breaks: "a struct of two rests is taken apart, into two JS rests",
    file: "src/lower/analysis/validation.rs",
    find: "                && is_mixed(pat.ty)",
    replace: "                && false",
    tests: ["test/jsx-props.test.ts", "-t", "two flattened fields"],
  },
  {
    name: "mixed-one-rest",
    breaks: "a struct of one rest is taken as one of two",
    file: "src/lower/analysis/validation.rs",
    find: "            > 1\n        {\n            mixed.insert",
    replace: "            > 0\n        {\n            mixed.insert",
    tests: ["test/jsx-props.test.ts", "-t", "flattened"],
  },
  {
    name: "flatten-js-object-refused",
    breaks: "a flattened `Dict` is refused, whose keys spread",
    file: "src/lower/analysis/validation.rs",
    find: " && !std(adt) || marks_js_object(tcx, adt, args)\n",
    replace: " && !std(adt)\n",
    tests: ["test/jsx-props.test.ts", "-t", "two flattened fields"],
  },
  {
    name: "extern-fn-declared-accepted",
    breaks:
      "a function in an `extern` block, a second form of a binding, is accepted where it's declared (ADR 0373)",
    file: "src/lower/analysis/validation.rs",
    find: "                if tcx.is_foreign_item(def_id)\n                    && tcx.fn_sig(def_id).skip_binder().abi() == rustc_abi::ExternAbi::Rust =>\n",
    replace:
      "                if false && tcx.is_foreign_item(def_id)\n                    && tcx.fn_sig(def_id).skip_binder().abi() == rustc_abi::ExternAbi::Rust =>\n",
    tests: ["test/diagnostics.test.ts", "-t", "a function in an extern block"],
  },
  {
    name: "extern-type-accepted",
    breaks:
      "an `extern` type, a second form of a JS type, is accepted (ADR 0373)",
    file: "src/lower/analysis/validation.rs",
    find: "            DefKind::ForeignTy => {\n",
    replace: "            DefKind::ForeignTy if false => {\n",
    tests: ["test/diagnostics.test.ts", "-t", "an extern type"],
  },
  {
    name: "rest-of-closure-allowed",
    breaks:
      "a closure's parameter marked `#[rust_js::rest]` is gathered, though what calls it gives what its type says",
    file: "src/lower/analysis/validation.rs",
    find: "            let refused = if tcx.is_closure_like(def_id) {\n",
    replace: "            let refused = if false {\n",
    tests: [
      "test/rest-parameters.test.ts",
      "-t",
      "refused: a closure's parameter",
    ],
  },
  {
    name: "rest-of-trait-method-allowed",
    breaks:
      "a trait method's parameter marked `#[rust_js::rest]` is gathered by one impl, though a call through its trait gives the slice whole",
    file: "src/lower/analysis/validation.rs",
    find: "            } else if tcx.trait_impl_of_assoc(def_id).is_some()\n                || tcx\n                    .trait_of_assoc(def_id)\n                    .is_some_and(|t| !crate::lower::classes::is_class_trait(tcx, t))\n            {\n",
    replace: "            } else if false {\n",
    tests: [
      "test/rest-parameters.test.ts",
      "-t",
      "refused: a trait method's parameter",
    ],
  },
  {
    name: "rest-of-component-allowed",
    breaks:
      "a component's props marked `#[rust_js::rest]` are gathered, though React gives it one object",
    file: "src/lower/analysis/validation.rs",
    find: "            } else if bindings::is_component(tcx, def_id) {\n",
    replace: "            } else if false {\n",
    tests: [
      "test/rest-parameters.test.ts",
      "-t",
      "refused: a component's props",
    ],
  },
  {
    name: "rest-not-last-allowed",
    breaks:
      "a parameter marked `#[rust_js::rest]` before another is allowed, which JS's rest parameter can't be",
    file: "src/lower/analysis/validation.rs",
    find: "            } else if index + 1 != count {\n",
    replace: "            } else if false {\n",
    tests: [
      "test/rest-parameters.test.ts",
      "-t",
      "refused: not the last parameter",
    ],
  },
  {
    name: "rest-of-no-slice-allowed",
    breaks:
      "a `Vec` marked `#[rust_js::rest]` is allowed, its call given the `Vec` whole",
    file: "src/lower/analysis/validation.rs",
    find: "            } else if !matches!(ty.kind(), ty::Ref(_, slice, rustc_hir::Mutability::Not) if slice.is_slice()) {\n",
    replace:
      "            } else if !matches!(ty.kind(), ty::Ref(_, slice, _) if slice.is_slice()) {\n",
    tests: ["test/rest-parameters.test.ts", "-t", "refused: a slice of &mut"],
  },
  {
    name: "rest-pattern-allowed",
    breaks:
      "a slice taken apart where it's given and marked `#[rust_js::rest]` is allowed, gathered as no name",
    file: "src/lower/analysis/validation.rs",
    find: "            } else if name.is_none() {\n",
    replace: "            } else if false {\n",
    tests: ["test/rest-parameters.test.ts", "-t", "refused: a pattern"],
  },
  {
    name: "parameter-name-repeat-allowed",
    breaks:
      "a parameter's `#[rust_js::name]` that repeats its camelCase name, a second way to write it, is no error",
    file: "src/lower/analysis/validation.rs",
    find: "    } else if given.as_str() == default {\n",
    replace: "    } else if false {\n",
    tests: [
      "test/bindings.test.ts",
      "-t",
      "a parameter's rust_js::name is its JS name",
    ],
  },
  {
    name: "parameter-name-any-text",
    breaks:
      'a parameter\'s `#[rust_js::name = "my-name"]`, no JS name, is written as it is, a syntax error',
    file: "src/lower/analysis/validation.rs",
    find: "    if !crate::names::is_identifier(given.as_str()) || crate::names::kept_word(given.as_str()) {\n",
    replace: "    if false {\n",
    tests: [
      "test/bindings.test.ts",
      "-t",
      "a parameter's rust_js::name is its JS name",
    ],
  },
  {
    name: "static-mut-pointer-refused",
    breaks:
      "a raw pointer to a `static mut`, `&raw mut TOTAL`, is refused as an address, not a handle on its place (ADR 0096, amended)",
    file: "src/lower/analysis/validation.rs",
    find: "                Some(_) if raw => continue,\n",
    replace:
      '                Some(_) if raw => Refused::Memory("raw addresses of statics"),\n',
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "a static mut a pointer to is given on",
    ],
  },
  {
    name: "static-pointer-unrefused",
    breaks:
      "a raw pointer to an immutable `static`, `&raw const LIMIT`, compiles, which rust-js has no place of (ADR 0096)",
    file: "src/lower/analysis/validation.rs",
    find: '                Some(d) if raw && !tcx.is_mutable_static(d) => Refused::Memory("raw addresses of statics"),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^static_raw_address.rs$"],
  },
  {
    name: "let-name-of-pattern-allowed",
    breaks:
      "a `let (x, y)`'s `#[rust_js::name]`, which names neither, is ignored, not refused (ADR 0855, amended)",
    file: "src/lower/analysis/validation.rs",
    find: "            let rustc_hir::PatKind::Binding(_, _, ident, None) = local.pat.kind else {\n",
    replace:
      "            let rustc_hir::PatKind::Binding(_, _, ident, None) = local.pat.kind else {\n                continue;\n            };\n            if false {\n",
    tests: [
      "test/lowering-symbols.test.ts",
      "-t",
      "a let's rust_js::name is its JS name",
    ],
  },
  {
    name: "let-name-unchecked",
    breaks:
      "a `let`'s `#[rust_js::name]` that's no string, no JS name or the name it has anyway is not refused (ADR 0855, amended)",
    file: "src/lower/analysis/validation.rs",
    find: "        for (local, attr) in bindings::named_lets(tcx, owner) {\n",
    replace:
      "        for (local, attr) in bindings::named_lets(tcx, owner).into_iter().filter(|_| false) {\n",
    tests: [
      "test/lowering-symbols.test.ts",
      "-t",
      "a let's rust_js::name is its JS name",
    ],
  },
  {
    name: "let-name-same-allowed",
    breaks:
      '`#[rust_js::name = "value"] let value`, a second way to write its name, is not refused (ADR 0855, amended)',
    file: "src/lower/analysis/validation.rs",
    find: "            let default = crate::lower::camel_case(ident.name.as_str());\n",
    replace: "            let default = String::new();\n",
    tests: [
      "test/lowering-symbols.test.ts",
      "-t",
      "a let's rust_js::name is its JS name",
    ],
  },
  {
    name: "rest-of-class-trait-refused",
    breaks:
      "a trait of classes' default's `#[rust_js::rest]` is refused, where each class's prototype's method takes rest arguments (rd283)",
    file: "src/lower/analysis/validation.rs",
    find: "                    .is_some_and(|t| !crate::lower::classes::is_class_trait(tcx, t))\n",
    replace: "                    .is_some_and(|_| true)\n",
    tests: ["test/classes-traits.test.ts"],
  },
  {
    name: "subclass-impl-refused",
    breaks: "a class's impl of react's `Component` is refused, as any binding trait's is",
    file: "src/lower/analysis/validation.rs",
    find: "                    || crate::lower::classes::subclass_impl_class(tcx, def_id.to_def_id()).is_some()\n",
    replace: "                    || false\n",
    tests: [
      "test/class-components.test.ts",
      "-t",
      "an ES class React renders"
    ]
  },
  {
    name: "flattened-lent-refused",
    breaks: "a flattened field lent whole, `&state.base`, is refused, so a struct that extends another can't be given to its base's functions",
    file: "src/lower/analysis/validation.rs",
    find: "                        && !lent.contains(&e)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_lent.rs$"],
  },
  {
    name: "flattened-lent-mutably",
    breaks: "a flattened field lent mutably, which may replace it whole, is taken as its parent's object",
    file: "src/lower/analysis/validation.rs",
    find: "                    borrow_kind: rustc_middle::mir::BorrowKind::Shared,\n",
    replace: "                    borrow_kind: _,\n",
    tests: ["test/lowering.test.ts", "-t", "flattened field lent whole"],
  },
  {
    name: "flattened-lent-read-whole",
    breaks: "a flattened field of a type whose clone spreads it is lent whole, so its clone holds the parent's fields too",
    file: "src/lower/analysis/validation.rs",
    find: "                        && reads_whole(expr.ty) =>\n",
    replace: "                        && false =>\n",
    tests: ["test/lowering.test.ts", "-t", "flattened field lent whole"],
  },
  {
    name: "flattened-lent-shadow-unseen",
    breaks: "a flattened field lent whole whose parent has a name it has too is taken as its parent's object, whose name is the parent's",
    file: "src/lower/analysis/validation.rs",
    find: "            return Some(field.name);\n",
    replace: "            return None;\n",
    tests: ["test/lowering.test.ts", "-t", "flattened field lent whole"],
  },
  {
    name: "flattened-std-struct-allowed",
    breaks: "a flattened `RefCell` of a struct, std's struct of named fields, is accepted, read as the parent's own (rd380)",
    file: "src/lower/analysis/validation.rs",
    find: "adt.is_struct() && adt.non_enum_variant().ctor.is_none() && !std(adt)",
    replace: "adt.is_struct() && adt.non_enum_variant().ctor.is_none()",
    tests: ["test/classes-flattened-state.test.ts", "-t", "refused"],
  },
  {
    name: "reads-whole-by-any-impl",
    breaks: "a flattened state's `RefCell` reads it whole by the `Clone` std has of any `RefCell`, where only its state's would, so every borrow of it is refused (rd380)",
    file: "src/lower/analysis/validation.rs",
    find: ".any(|tr| crate::lower::recognition::trait_holds_in(tcx, env, ty, tr) != Some(false))",
    replace: ".any(|_| true)",
    tests: ["test/classes-flattened-state.test.ts", "-t", "made, read and replaced in place"],
  },
  {
    name: "namespace-value-read-allowed",
    breaks: "a Rust read of a namespace thread-local, `NOOP.with(..)`, is lowered as a cell's, which no JS variable is (rd293, ADR 1037, amended)",
    file: "src/lower/analysis/validation.rs",
    find: "                && crate::lower::classes::is_value_namespace(tcx, def_id)\n",
    replace: "                && false\n",
    tests: ["test/namespace-values.test.ts", "-t", "Rust reads is refused"],
  },
  {
    name: "keyed-shadow-unchecked",
    breaks: "a keyed enum flattened into a parent with a field of one of its keys compiles, and the parent's own is overwritten (#504)",
    file: "src/lower/analysis/validation.rs",
    find: "        .find(|key| keyed.keys.contains(key))\n",
    replace: "        .find(|_| false)\n",
    tests: ["test/flattened-keyed.test.ts", "-t", "no key tells"],
  },
  {
    name: "keyed-param-given-anything",
    breaks: "a struct's flattened type parameter given a number compiles, though it has no keys (#504)",
    file: "src/lower/analysis/validation.rs",
    find: "                    _ => \"a flattened type parameter is given an enum each of whose variants has a field no other has, one at most none\".into(),\n",
    replace: "                    _ => continue,\n",
    tests: ["test/flattened-keyed.test.ts", "-t", "no key tells"],
  },
  {
    name: "keyed-param-given-a-struct-refused",
    breaks: "a struct's flattened type parameter given a struct is refused as one given an enum no key tells, so React's DetailedHTMLProps<AnchorHTMLAttributes, ..> is (#504's regression)",
    file: "src/lower/analysis/validation.rs",
    find: "                    ty::Adt(adt, args) if plain(*adt, args) => continue,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^flattened_keyed_param.rs$"],
  },
  {
    name: "keyed-placed-refused",
    breaks: "a flattened keyed enum matched where it's a place, or written whole, is refused as one read whole (#504)",
    file: "src/lower/analysis/validation.rs",
    find: "                        && !(keyed(expr.ty) && placed.contains(&e)) =>\n",
    replace: "                        =>\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_keyed_enum.rs$"],
  },
  {
    name: "keyed-placed-anything",
    breaks: "a flattened field of any type matched or written whole compiles, a type parameter's too (#504)",
    file: "src/lower/analysis/validation.rs",
    find: "                        && !(keyed(expr.ty) && placed.contains(&e)) =>\n",
    replace: "                        && !placed.contains(&e) =>\n",
    tests: ["test/flattened-keyed.test.ts", "-t", "moved out"],
  },
];
