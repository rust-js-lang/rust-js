// Mutations of src/lower/analysis/naming.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "held-read-imported-too",
    breaks:
      'a module that reads a binding through the module that holds it imports what the binding reads as well, `import * as react from "react"`, unused',
    file: "src/lower/analysis/naming.rs",
    find: "            .filter(|(_, expr)| !held.reads.contains_key(&expr.span))\n",
    replace: "",
    tests: ["test/lowering-bindings.test.ts", "-t", "named through the module"],
  },
  {
    name: "held-import-named-by-module",
    breaks:
      "a library's module's `const` is imported by its module's name, `internals`, not the `const`'s, `ReactSharedInternals`",
    file: "src/lower/analysis/naming.rs",
    find: "                _ if uses.named.contains_key(export_key) => uses.named[export_key].clone(),\n",
    replace: "",
    tests: ["test/cargo-link.test.ts", "-t", "holds a binding"],
  },
  {
    name: "reexport-renames-own",
    breaks:
      "a `pub use` by another name is taken for the module's own name of what it re-exports, so `use priorities::IMMEDIATE` beside `pub use priorities::IMMEDIATE as UNSTABLE_IMMEDIATE` imports it as `IMMEDIATE as unstable_ImmediatePriority`",
    file: "src/lower/analysis/naming.rs",
    find: "UseKind::Single(ident)) if !super::is_reexport(tcx, item) =>",
    replace: "UseKind::Single(ident)) =>",
    tests: ["test/modules.test.ts", "-t", "another name of the module's own"],
  },
  {
    name: "import-named-over-export",
    breaks:
      "an import takes the name of the crate's own function, which is renamed, so JS calling it by its Rust name finds none",
    file: "src/lower/analysis/naming.rs",
    find: "            let mut reserved: HashSet<String> = uses.globals.iter().chain(items).cloned().collect();",
    replace:
      "            let mut reserved: HashSet<String> = uses.globals.iter().cloned().collect();",
    tests: ["test/crates.test.ts", "-t", "two crates: same_name"],
  },
  {
    name: "component-default-import-unnamed",
    breaks:
      "\`next/image#default\` of \`fn Image\` is \`image\`, which JSX takes for an element",
    file: "src/lower/analysis/naming.rs",
    find: "                [only] if tcx.def_kind(**only) == DefKind::Fn => Some(bindings::fn_name(tcx, **only)),",
    replace:
      "                [only] if false && tcx.def_kind(**only) == DefKind::Fn => Some(bindings::fn_name(tcx, **only)),",
    tests: ["test/next.test.ts", "-t", "build builds"],
  },
  {
    name: "default-member-names-import",
    breaks:
      "`next/app#default.origGetInitialProps`, used alone, names the import after its function, `orig_get_initial_props`",
    file: "src/lower/analysis/naming.rs",
    find: "                .map(|held| held.iter().filter(itself).collect::<Vec<_>>())",
    replace: "                .map(|held| held.iter().collect::<Vec<_>>())",
    tests: ["test/next.test.ts", "-t", "build builds"],
  },
  {
    name: "default-statics-named-by-file",
    breaks:
      "a default class used for its statics, in `mod greeter`, is `greeter`, not `Greeter`",
    file: "src/lower/analysis/naming.rs",
    find: '                ("default", None, Some(module)) if statics => {\n',
    replace:
      '                ("default", None, Some(module)) if false && statics => {\n',
    tests: ["test/modules.test.ts", "-t", "default import of a class"],
  },
  {
    name: "default-function-named-as-class",
    breaks:
      "a default function a module holds two bindings of, in `mod sizes`, is named as a class whose statics it holds, `Sizes`, not for its file",
    file: "src/lower/analysis/naming.rs",
    find: "                .is_some_and(|held| !held.iter().any(|id| itself(&id)));",
    replace: "                .is_some_and(|held| !held.is_empty());",
    tests: ["test/modules.test.ts", "-t", "default function bound twice"],
  },
  {
    name: "import-named-around-every-module",
    breaks:
      "an import is renamed for another module's item, `import { join as join$1 }` for a `fn join` elsewhere",
    file: "src/lower/analysis/naming.rs",
    find: "uses.globals.iter().chain(items).cloned().collect();",
    replace:
      "uses.globals.iter().chain(taken.values().flatten()).cloned().collect();",
    tests: ["test/snapshots.test.ts", "-t", "imports"],
    snapshots: true,
  },
  {
    name: "imports-named-alike",
    breaks:
      "a module's imports aren't named around each other: `./greet.js`'s default and `./inner/wave.js`'s `greet` are both `greet`",
    file: "src/lower/analysis/naming.rs",
    find: "fresh_in(&mut reserved, alias.as_ref().unwrap_or(base)),",
    replace: "fresh_in(&mut reserved.clone(), alias.as_ref().unwrap_or(base)),",
    tests: ["test/snapshots.test.ts", "-t", "imports"],
    snapshots: true,
  },
  {
    name: "default-import-not-renamed",
    breaks:
      '`use …::Link as NextLink` beside the module\'s own `Link` is `import Link$1 from "next/link"`',
    file: "src/lower/analysis/naming.rs",
    find: ".filter(|&&id| default || binds_itself(tcx, id))",
    replace: ".filter(|&&id| !default && binds_itself(tcx, id))",
    tests: ["test/jsx.test.ts", "-t", "use renames it"],
  },
  {
    name: "named-binding-import-not-renamed",
    breaks:
      "`use bound::twice as double` is `import { twice }`, not the name the module's `use` gives it",
    file: "src/lower/analysis/naming.rs",
    find: ".filter(|&&id| default || binds_itself(tcx, id))",
    replace: ".filter(|&&id| default)",
    tests: ["test/modules.test.ts", "-t", "renames a binding"],
  },
  {
    name: "member-binding-renames-import",
    breaks:
      "`use bound::scale as scaled`, of `tools.scale`, names the import of `tools` `scaled`, `scaled.scale(x)`",
    file: "src/lower/analysis/naming.rs",
    find: ".filter(|&&id| default || binds_itself(tcx, id))",
    replace: ".filter(|&&id| default || binds_itself(tcx, id) || true)",
    tests: ["test/modules.test.ts", "-t", "renames a binding"],
  },
  {
    name: "two-import-names-kept",
    breaks:
      "`use inner::client as first; use inner::client as second;` compiles, and one of the names is left unsaid",
    file: "src/lower/analysis/naming.rs",
    find: "                Some(named) if *named != alias => {",
    replace: "                Some(named) if false && *named != alias => {",
    tests: ["test/modules.test.ts", "-t", "naming one item apart"],
  },
  {
    name: "library-import-not-renamed",
    breaks:
      "`use shared::enqueue_task::enqueue_task as queue_macrotask` is `import enqueueTask`, not the name ReactAct's `use` gives it",
    file: "src/lower/analysis/naming.rs",
    find: "                        .or_else(|| library_renamed.get(*export).map(|alias| (*alias).clone()));",
    replace: "                        ;",
    tests: ["test/cross-crate-items.test.ts", "-t", "renames it to"],
  },
  {
    name: "renamed-static-not-camel-case",
    breaks:
      '`use …::hero_img as banner_img` is `import banner_img from "./hero.png"`, not an asset\'s camel case',
    file: "src/lower/analysis/naming.rs",
    find: "alias.unwrap_or_else(|| bindings::alias_cased(tcx, def_id, ident.name.as_str())),",
    replace: "alias.unwrap_or_else(|| ident.name.to_string()),",
    tests: ["test/jsx.test.ts", "-t", "use renames it"],
  },
  {
    name: "renamed-thread-local-as-written",
    breaks:
      "`use shared::object_is::OBJECT_IS as IS` is `import IS`, called `IS(..)`, not the thread-local's camelCase `is` (ADR 0202, amended)",
    file: "src/lower/analysis/naming.rs",
    find: "alias.unwrap_or_else(|| bindings::alias_cased(tcx, def_id, ident.name.as_str())),",
    replace: "alias.unwrap_or_else(|| camel_case(ident.name.as_str())),",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "cased as the thread-local",
    ],
  },
  {
    name: "tested-variant-unimported",
    breaks:
      "a `match` of an element calls `isValidElement`, which the module doesn't import",
    file: "src/lower/analysis/naming.rs",
    find: "        for arm in body.thir.arms.iter() {\n            tested(&arm.pattern);\n        }\n",
    replace: "",
    tests: ["test/jsx-types.test.ts", "-t", "look inside their children"],
  },
  {
    name: "namespace-named-by-file",
    breaks:
      "a namespace whose bindings `mod ContextMenu` holds is named by its file, `import * as Menu`",
    file: "src/lower/analysis/naming.rs",
    find: "        Some(module) if holds_only(tcx, module, &export.0) => {\n",
    replace:
      "        Some(module) if false && holds_only(tcx, module, &export.0) => {\n",
    tests: ["test/modules.test.ts", "-t", "namespace import is named"],
  },
  {
    name: "named-expression-module-name",
    breaks:
      "a function a block makes and gives takes a module name, so a function of the module named alike is `whisper$1`",
    file: "src/lower/analysis/naming.rs",
    find: "            None if named_expressions.contains(&def_id) => (js_name, None),\n",
    replace: "",
    tests: [
      "test/jsx-children.test.ts",
      "-t",
      "a function a block makes and gives",
    ],
  },
  {
    name: "local-consts-none",
    breaks:
      "a `const` written in a function's body is its module's, beside the function",
    file: "src/lower/analysis/naming.rs",
    find: "            within.then_some((def_id, (block.span, readers)))\n",
    replace:
      "            (within && false).then_some((def_id, (block.span, readers)))\n",
    tests: ["test/corpus.test.ts", "-t", "^local_consts.rs$"],
    snapshots: true,
  },
  {
    name: "local-const-read-outside",
    breaks:
      "a `const` written in a function's body that a method of the module's reads is the function's, which the method can't see",
    file: "src/lower/analysis/naming.rs",
    find: "            within.then_some((def_id, (block.span, readers)))\n",
    replace: "            Some((def_id, (block.span, readers)))\n",
    tests: ["test/corpus.test.ts", "-t", "^local_consts.rs$"],
  },
  {
    name: "named-expression-any-nested",
    breaks:
      "a function a block makes but doesn't give is put where it's called, `(function loud(text) { .. })(text)`",
    file: "src/lower/analysis/naming.rs",
    find: "    matches!(block.expr, Some(hir::Expr { kind: hir::ExprKind::Path(hir::QPath::Resolved(None, path)), .. })\n        if path.res == Res::Def(DefKind::Fn, def_id.to_def_id()))",
    replace: "    true",
    tests: [
      "test/jsx-children.test.ts",
      "-t",
      "a function a block makes and gives",
    ],
  },
  {
    name: "named-expression-recursive",
    breaks:
      "a function that calls itself is put where it's named, leaving its own call a hole",
    file: "src/lower/analysis/naming.rs",
    find: "uses == 1 && given_by_its_block",
    replace: "uses >= 1 && given_by_its_block",
    tests: [
      "test/jsx-children.test.ts",
      "-t",
      "a function a block makes and gives",
    ],
  },
  {
    name: "named-expression-generic",
    breaks:
      "a generic function a block gives is put where it's named, without the dictionaries its caller gives",
    file: "src/lower/analysis/naming.rs",
    find: " || tcx.generics_of(def_id).count() != 0",
    replace: "",
    tests: [
      "test/jsx-children.test.ts",
      "-t",
      "a function a block makes and gives",
    ],
  },
  {
    name: "dynamic-import-static-too",
    breaks:
      "a binding `js::import!` loads is imported statically too, and its local renamed",
    file: "src/lower/analysis/naming.rs",
    find: "            .filter(|(id, _)| !dynamic.contains(id))\n",
    replace: "            .filter(|(id, _)| true || !dynamic.contains(id))\n",
    tests: ["test/bindings.test.ts", "-t", "js::import! loads a module"],
  },
  {
    name: "required-also-imported",
    breaks: "a `#[rust_js::require]` binding is imported statically too",
    file: "src/lower/analysis/naming.rs",
    find: "                Some((_, Some((export, _)))) if bindings::requires(tcx, def_id) => {\n",
    replace:
      "                Some((_, Some((export, _)))) if false && bindings::requires(tcx, def_id) => {\n",
    tests: ["test/bindings.test.ts", "-t", "a required binding"],
  },
  {
    name: "local-functions-none",
    breaks:
      "a function written in a body is the module's, not its body's where it's written",
    file: "src/lower/analysis/naming.rs",
    find: "                        hir::Node::Stmt(_)\n",
    replace: "                        hir::Node::Stmt(_) if false\n",
    tests: ["test/lowering-functions.test.ts", "-t", "local function there"],
  },
  {
    name: "local-function-static-user",
    breaks:
      "a function a static's initializer calls is written in its body, where the module's initializer can't call it",
    file: "src/lower/analysis/naming.rs",
    find: "                    .any(|&user| user != def_id && user != parent(def_id) && !local.contains(&user))\n",
    replace:
      "                    .any(|&user| false && user != def_id && user != parent(def_id) && !local.contains(&user))\n",
    tests: ["test/lowering-functions.test.ts", "-t", "local function there"],
  },
  {
    name: "anon-const-unnamed",
    breaks: "a `const { .. }` block, which has no name, is named by its item's",
    file: "src/lower/analysis/naming.rs",
    find: "    if tcx.def_kind(def_id) == DefKind::AnonConst {",
    replace: "    if false {",
    tests: ["test/corpus.test.ts", "-t", "slice_uninit"],
  },
  {
    name: "default-local-function-local",
    breaks:
      "a function written in a trait's default is declared in each impl's copy, where what it calls isn't imported, and the copy calls the trait module's, which isn't there",
    file: "src/lower/analysis/naming.rs",
    find: "                    && tcx\n                        .trait_of_assoc(tcx.typeck_root_def_id(tcx.local_parent(local).to_def_id()))\n                        .is_none()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^default_inner_fn.rs$"],
  },
  {
    name: "namespace-module-as-written",
    breaks:
      "a namespace import a snake_case module holds is named `side_menu`, not PascalCase `SideMenu`",
    file: "src/lower/analysis/naming.rs",
    find: "            crate::lower::namespace_name(tcx.item_name(module).as_str())\n",
    replace: "            tcx.item_name(module).to_string()\n",
    tests: [
      "test/modules.test.ts",
      "-t",
      "a namespace import is named as the module of its bindings",
    ],
  },
  {
    name: "own-alias-use-camel-case",
    breaks:
      "`use scheduler::unstable_now;` imports it as `unstableNow`, not by the name it's exported as (ADR 0596, amended)",
    file: "src/lower/analysis/naming.rs",
    find: "                    let alias = given.or_else(|| crate::lower::default_exports::alias_named(tcx, def_id, ident.name));\n",
    replace: "                    let alias = given;\n",
    tests: [
      "test/modules.test.ts",
      "-t",
      "a module's own item a pub use renames is imported by that name",
    ],
  },
  {
    name: "use-name-attribute-ignored",
    breaks:
      "a private `use .. as`'s `#[rust_js::name]` is ignored, `_areaDev` where it says `_areaDEV` (ADR 0596, amended)",
    file: "src/lower/analysis/naming.rs",
    find: "                    let given = bindings::given_name(tcx, item.owner_id.to_def_id());\n",
    replace: "                    let given: Option<String> = None;\n",
    tests: ["test/modules.test.ts", "-t", "re-exported as written"],
  },
  {
    name: "namespace-of-any-holding-module",
    breaks:
      "a namespace whose bindings a module of other code holds, `mod cache` calling `scheduler#*.unstable_now`, is named after that module, `import * as Cache`, not `Scheduler`",
    file: "src/lower/analysis/naming.rs",
    find: "        Some(module) if holds_only(tcx, module, &export.0) => {\n",
    replace: "        Some(module) => {\n",
    tests: [
      "test/modules.test.ts",
      "-t",
      "a namespace import is named as the module of its bindings",
    ],
  },
  {
    name: "namespace-module-of-other-items",
    breaks:
      "a module is its namespace's though it holds a function of its own too, so `mod cache`'s `scheduler#*` is `import * as cache`",
    file: "src/lower/analysis/naming.rs",
    find: "            _ => a_type(tcx.def_kind(def_id)) || of_it(def_id),\n",
    replace:
      "            _ => a_type(tcx.def_kind(def_id)) || of_it(def_id) || matches!(item.kind, ItemKind::Fn { .. }),\n",
    tests: [
      "test/modules.test.ts",
      "-t",
      "a namespace import is named as the module of its bindings",
    ],
  },
  {
    name: "namespace-module-of-props-unread",
    breaks:
      "a module of a namespace's parts and the props they take isn't the namespace's, so `mod ContextMenu` is `import * as menu`",
    file: "src/lower/analysis/naming.rs",
    find: "            _ => a_type(tcx.def_kind(def_id)) || of_it(def_id),\n",
    replace: "            _ => of_it(def_id),\n",
    tests: [
      "test/modules.test.ts",
      "-t",
      "a namespace import is named as the module of its bindings",
    ],
  },
  {
    name: "namespace-module-written-impls-unread",
    breaks:
      "a module of a namespace's parts whose props derive `Clone`, or `jsx!` writes an impl of, isn't the namespace's, so `mod ContextMenu` is `import * as menu`",
    file: "src/lower/analysis/naming.rs",
    find: "                item.span.from_expansion()\n                    || (tcx",
    replace: "                (tcx",
    tests: [
      "test/modules.test.ts",
      "-t",
      "a namespace import is named as the module of its bindings",
    ],
  },
  {
    name: "namespace-module-jsx-macro-unread",
    breaks:
      "a module of components whose props `jsx!` writes a macro of isn't the namespace's, so `<Menu.Item>` is `<Parts.Item>`, named after its file",
    file: "src/lower/analysis/naming.rs",
    find: " | DefKind::Trait | DefKind::Macro(_)\n",
    replace: " | DefKind::Trait\n",
    tests: [
      "test/jsx.test.ts",
      "-t",
      "a namespace import's component is a tag where it's written",
    ],
  },
  {
    name: "namespace-module-of-another-namespace",
    breaks:
      "a module holding another module's bindings too is the namespace's, named after the module",
    file: "src/lower/analysis/naming.rs",
    find: '            .is_some_and(|((f, export), _)| f == from && export == "*")\n',
    replace: "            .is_some()\n",
    tests: [
      "test/modules.test.ts",
      "-t",
      "a namespace import is named as the module of its bindings",
    ],
  },
  {
    name: "exports-named-in-order",
    breaks:
      "a module's items are named in the order they're found, so a private function takes `emptyContextObject` and the export it reads is `emptyContextObject$1` (ADR 0922)",
    file: "src/lower/analysis/naming.rs",
    find: ".partition(|&def_id| exported(def_id));\n",
    replace: ".partition(|_| false);\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "an export keeps its name beside a private item of it",
    ],
  },
  {
    name: "two-exports-one-name-unrefused",
    breaks:
      "two exports of one JS name compile, one named apart, which its importers don't read (ADR 0922)",
    file: "src/lower/analysis/naming.rs",
    find: "        if exported(def_id) {\n            match exports.entry(",
    replace:
      "        if exported(def_id) && false {\n            match exports.entry(",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "two exports of one JS name are refused",
    ],
  },
  {
    name: "class-display-owned-apart",
    breaks:
      "a class's `Display`'s `fmt` is a function of its own, not its class's prototype's `toString` (#358)",
    file: "src/lower/analysis/naming.rs",
    find: "        let owner_type = owner_type.or_else(|| crate::lower::classes::shown_class(tcx, def_id));\n",
    replace: "",
    tests: ["test/classes.test.ts", "-t", "Display is its prototype"],
  },
  {
    name: "class-display-named-fmt",
    breaks:
      "a class's `Display`'s `fmt` is named `queenPositionDisplay_fmt` on its prototype, which JS's `String` never calls (#358)",
    file: "src/lower/analysis/naming.rs",
    find: '        return "toString".to_owned();\n',
    replace: '        return "show".to_owned();\n',
    tests: ["test/classes.test.ts", "-t", "Display is its prototype"],
  },
  {
    name: "class-named-by-its-struct",
    breaks:
      'a class\'s `#[rust_js::name = "FiberNode"]` is ignored, its constructor function `Fiber`',
    file: "src/lower/analysis/naming.rs",
    find: "                let named = crate::lower::classes::name(tcx, ty).unwrap_or_else(|| tcx.item_name(ty).to_string());\n",
    replace: "                let named = tcx.item_name(ty).to_string();\n",
    tests: ["test/classes.test.ts", "-t", "constructor's statements set this"],
  },
  {
    name: "object-method-its-types",
    breaks:
      "an object's method is named as a property of its type's object, `ScopeInstance.containsNode`, where it's a function of its module its literal holds (ADR 0943)",
    file: "src/lower/analysis/naming.rs",
    find: "                Some(crate::lower::classes::Member::Own | crate::lower::classes::Member::Namespace)\n",
    replace: "                Some(crate::lower::classes::Member::Namespace)\n",
    tests: [
      "test/classes.test.ts",
      "-t",
      "an object's methods are its literal's own",
    ],
  },
  {
    name: "namespace-names-unchecked",
    breaks:
      "a module's `*` named `Scheduler` by one `link_name` and otherwise, or not at all, by another is one of them, not refused",
    file: "src/lower/analysis/naming.rs",
    find: "            [Some(name)] => {\n",
    replace: "            [.., Some(name)] => {\n",
    tests: [
      "test/modules.test.ts",
      "-t",
      "a namespace import is named as its link_name",
    ],
  },
  {
    name: "use-group-named-each-the-same",
    breaks:
      "a `#[rust_js::name]` on a group `use` names each of its imports the same, `ImmediatePriority as SchedulerPriority, NormalPriority as SchedulerPriority$1`, not an error",
    file: "src/lower/analysis/naming.rs",
    find: "                && bindings::given_name(tcx, item.owner_id.to_def_id()).is_some()\n",
    replace: "                && false\n",
    tests: [
      "test/modules.test.ts",
      "-t",
      "a use of one name is imported by its rust_js::name",
    ],
  },
  {
    name: "reexported-default-unexported",
    breaks:
      "another module's `pub(crate) fn twice` a module's default is, which no other module calls, isn't exported by its module, so the default's `export { twice as default } from \"./helpers.js\"` fails to load (ADR 0577, amended)",
    file: "src/lower/analysis/naming.rs",
    find: "        if fns.get(&item).is_some_and(|target| target.module != module) {\n",
    replace: "        if false {\n",
    tests: [
      "test/modules.test.ts",
      "-t",
      "a module's default is called with whatever JS gives it",
    ],
  },
  {
    name: "use-as-underscore-renames",
    breaks:
      "`js::export_default!`'s `use fourfold as _;` renames `fourfold` in its module, which its `use super::helpers::fourfold;` names by its name, `import { fourfold as _ }` and `_(2)` (ADR 0577, amended)",
    file: "src/lower/analysis/naming.rs",
    find: "                        && (ident.name == tcx.item_name(def_id) || ident.name == rustc_span::kw::Underscore)\n",
    replace:
      "                        && (ident.name == tcx.item_name(def_id))\n",
    tests: [
      "test/modules.test.ts",
      "-t",
      "a module's default is called with whatever JS gives it",
    ],
  },
  {
    name: "binding-given-name-unread",
    breaks:
      "a binding's `#[rust_js::name]` names nothing, `import { NotPendingTransition }` read by that name, where React's ReactFiberHooks imports it `as NoPendingHostTransition` (ADR 0028, amended)",
    file: "src/lower/analysis/naming.rs",
    find: "                        .or_else(|| given_in(tcx, uses, export, module))\n",
    replace: "",
    tests: [
      "test/modules.test.ts",
      "-t",
      "a binding's rust_js::name names its import",
    ],
  },
  {
    name: "global-given-name-ignored",
    breaks:
      "a global's binding's `#[rust_js::name]`, which no import names, is ignored, not refused (ADR 0028, amended)",
    file: "src/lower/analysis/naming.rs",
    find: "                    if let Some(name) = bindings::given_name(tcx, def_id)\n",
    replace:
      "                    if let Some(name) = bindings::given_name(tcx, def_id).filter(|_| false)\n",
    tests: [
      "test/modules.test.ts",
      "-t",
      "a binding's rust_js::name names its import",
    ],
  },
  {
    name: "member-given-name-ignored",
    breaks:
      "a binding of a member's or a namespace's `#[rust_js::name]`, which no import of its name holds, is ignored, not refused (ADR 0028, amended)",
    file: "src/lower/analysis/naming.rs",
    find: '            if export.1 == "*" || !binds_itself(tcx, id) {\n',
    replace: "            if false {\n",
    tests: [
      "test/modules.test.ts",
      "-t",
      "a binding's rust_js::name names its import",
    ],
  },
  {
    name: "two-given-import-names",
    breaks:
      "two bindings of one import naming it two names compile, one of them unread (ADR 0028, amended)",
    file: "src/lower/analysis/naming.rs",
    find: "        if let Some((name, id)) = names.iter().find(|(name, _)| *name != names[0].0) {\n",
    replace:
      "        if let Some((name, id)) = names.iter().find(|(name, _)| *name != names[0].0 && false) {\n",
    tests: [
      "test/modules.test.ts",
      "-t",
      "a binding's rust_js::name names its import",
    ],
  },
  {
    name: "held-binding-name-refused",
    breaks:
      'a binding a module holds as its `const`, `#[rust_js::name = "ReactSharedInternals"]` of `react#*.__CLIENT_INTERNALS..`, whose name names that `const` (ADR 0633), is refused as naming no import (ADR 0028, amended)',
    file: "src/lower/analysis/naming.rs",
    find: "        for &id in held.iter().filter(|id| !uses.held.contains(*id)) {\n",
    replace: "        for &id in held.iter() {\n",
    tests: [
      "test/lowering-bindings.test.ts",
      "-t",
      "a binding named through the module that holds it is read through it",
    ],
  },
  {
    name: "given-import-name-of-another-module",
    breaks:
      "a binding's `#[rust_js::name]` names its export's import in every module, so ReactFiberHostContext's own binding of `NotPendingTransition` is imported `as NoPendingHostTransition`, ReactFiberHooks' name (ADR 0028, amended)",
    file: "src/lower/analysis/naming.rs",
    find: "        .filter(|&&id| declared_in(tcx, id) == Some(module))\n",
    replace: "",
    tests: [
      "test/modules.test.ts",
      "-t",
      "a binding's rust_js::name names its import",
    ],
  },
  {
    name: "own-item-renames-refused",
    breaks:
      "a module's `use`s that give its own item two names, `pub(super) use self::Foo as Bar;`, are refused as two names of one import, which its file has none of (ADR 0202, amended; rustc's privacy/pub-restricted-path-usage-55376.rs)",
    file: "src/lower/analysis/naming.rs",
    find: ".filter(|&(def_id, ..)| !own(def_id))",
    replace: ".filter(|&(def_id, ..)| !own(def_id) || true)",
    tests: ["test/corpus.test.ts", "-t", "^use_renames_own_item.rs$"],
  },
  {
    name: "use-tree-at-its-leaf",
    breaks:
      "a comment above a `use` tree, `use crate::{a::x, b::y};`, is lost: each import it makes is where its leaf's path is (rd92)",
    file: "src/lower/analysis/naming.rs",
    find: "                Some((statement(item.span), named))\n",
    replace: "                Some((item.span, named))\n",
    tests: ["test/lowering-comments-items.test.ts", "-t", "above a use"],
  },
  {
    name: "binding-use-makes-no-import",
    breaks:
      "a comment above the `use` of a binding is lost, not above its JS file's import (rd92)",
    file: "src/lower/analysis/naming.rs",
    find: "            export.is_some_and(|(of, _)| of == from)\n",
    replace: "            export.is_some_and(|(of, _)| of == from && false)\n",
    tests: ["test/lowering-comments-items.test.ts", "-t", "above a use"],
  },
  {
    name: "module-use-makes-no-import",
    breaks:
      "a comment above the `use` of a module is above the next item, not above the import of what's read through it (rd92)",
    file: "src/lower/analysis/naming.rs",
    find: "            DefKind::Mod => tcx.is_descendant_of(target.to_def_id(), id),\n",
    replace: "            DefKind::Mod => false,\n",
    tests: ["test/lowering-comments-items.test.ts", "-t", "above a use"],
  },
  {
    name: "namespace-named-camel-case",
    breaks:
      "a binding's namespace imported from its file is `import * as scheduler`, camelCase, where every namespace is PascalCase, `import * as Scheduler`, as React writes it (rd114b, ADR 1018)",
    file: "src/lower/analysis/naming.rs",
    find: "        _ => crate::lower::namespace_name(bindings::file_stem(&export.0)),\n",
    replace: "        _ => module_binding(&export.0),\n",
    tests: [
      "test/lowering-bindings.test.ts",
      "-t",
      "named through the module that holds it",
    ],
  },
  {
    name: "namespace-name-repeat-accepted",
    breaks:
      "`scheduler#* as Scheduler.unstable_now` names the namespace as it's named anyway, which is accepted, two spellings of one import (rd114b, ADR 1018)",
    file: "src/lower/analysis/naming.rs",
    find: "            [Some(name)] if *name == namespace_default(tcx, &uses, &export) => {\n",
    replace:
      "            [Some(name)] if false && *name == namespace_default(tcx, &uses, &export) => {\n",
    tests: [
      "test/modules.test.ts",
      "-t",
      "a namespace import is named as its link_name",
    ],
  },
  {
    name: "export-named-apart-exported-apart",
    breaks:
      "`pub fn delete()` is exported as its module names it, `export function delete$()`, which a JS importer of `delete` reads as `undefined` (#338)",
    file: "src/lower/analysis/naming.rs",
    find: "        let export = (exported(def_id) && owner.is_none() && name != own_name).then_some(own_name);\n",
    replace:
      "        let export = (false && exported(def_id) && owner.is_none() && name != own_name).then_some(own_name);\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "an export its module names apart is exported by its own name",
    ],
  },
  {
    name: "global-name-of-quoted-path",
    breaks:
      "`window['$RT']`'s global is taken to be that whole text, so a local `window` isn't renamed and `window[\"$RT\"] = window + 1` writes to the number",
    file: "src/lower/analysis/naming.rs",
    find: ".insert(bindings::first_name(path).to_string());",
    replace: ".insert(path.split('.').next().unwrap_or_default().to_string());",
    tests: ["test/lowering-binding-paths.test.ts"],
  },
  {
    name: "namespace-methods-in-objects",
    breaks:
      "a namespace's methods are its type's object's members, not its module's functions (ADR 1037)",
    file: "src/lower/analysis/naming.rs",
    find: "                Some(crate::lower::classes::Member::Own | crate::lower::classes::Member::Namespace)\n",
    replace: "                Some(crate::lower::classes::Member::Own)\n",
    tests: [
      "test/namespaces.test.ts",
      "-t",
      "a namespace's methods are its module's functions",
    ],
  },
];
