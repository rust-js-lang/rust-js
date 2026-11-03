// Mutations of src/lower/declarations/aliases.rs (ADR 0093): a type alias in
// a module's declarations.
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "alias-expanded",
    breaks:
      "a field written as @types/react's alias is its expansion, `(event: MouseEvent<..>) => void`, where it's `MouseEventHandler<HTMLButtonElement>`",
    file: "src/lower/declarations/aliases.rs",
    find: "        let declared = written_types(self.tcx, did)?;\n        let given: Vec<&hir::Ty<'tcx>>",
    replace:
      "        let declared = written_types(self.tcx, did).filter(|_| false)?;\n        let given: Vec<&hir::Ty<'tcx>>",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "aliases as the Rust names them",
    ],
  },
  {
    name: "alias-default-written",
    breaks:
      "an alias's argument left to its default is written out, `MouseEventHandler<Element>`, where it's `MouseEventHandler`",
    file: "src/lower/declarations/aliases.rs",
    find: "                    && at(name).is_some_and(|at| at >= given.len())\n                {\n                    break;\n",
    replace:
      '                    && at(name).is_some_and(|at| at >= given.len())\n                {\n                    let param = (self.tcx.generics_of(did).own_params.iter()).find(|p| p.name.as_str() == name.as_str()).expect("a parameter");\n                    let ty = self.tcx.type_of(param.def_id).instantiate_identity().skip_normalization();\n                    args.push(self.ts(ty));\n                    continue;\n',
    tests: [
      "test/declarations.test.ts",
      "-t",
      "aliases as the Rust names them",
    ],
  },
  {
    name: "written-alias-not-nested",
    breaks:
      "an alias written as another type's argument, `Context<Toc>`, is what it stands for, `Context<TocItem[]>`",
    file: "src/lower/declarations/aliases.rs",
    find: "                        self.aliases_in(ty.as_unambig_ty(), named);",
    replace: "                        let _ = ty;",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "declarations name a type alias",
    ],
  },
  {
    name: "written-alias-args-expanded",
    breaks:
      "an alias's argument written as an alias, `Pair<Toc>`, is what it stands for",
    file: "src/lower/declarations/aliases.rs",
    find: "                self.as_written(Some(given), |out| out.ts(ty))\n",
    replace: "                self.ts(ty)\n",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "declarations name a type alias",
    ],
  },
  {
    name: "written-alias-in-ref",
    breaks:
      "an alias written behind a reference, `&Toc`, is what it stands for",
    file: "src/lower/declarations/aliases.rs",
    find: "            hir::TyKind::Ref(_, inner) | hir::TyKind::Ptr(inner) => self.aliases_in(inner.ty, named),",
    replace:
      "            hir::TyKind::Ptr(inner) => self.aliases_in(inner.ty, named),",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "declarations name a type alias",
    ],
  },
  {
    name: "written-alias-unimported",
    breaks:
      "another module's alias is named but not imported, and TypeScript finds no `Toc`",
    file: "src/lower/declarations/aliases.rs",
    find: "            self.imported(file, did, name.clone());",
    replace: "            let _ = file;",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "declarations name a type alias",
    ],
  },
  {
    name: "written-alias-of-its-own-expanded",
    breaks:
      "an alias whose `#[rust_js::types]` writes a union, `ReactServerValue`, is that union written out at each use, not named (rd538)",
    file: "src/lower/declarations/aliases.rs",
    find: "        let own = declares_written_alias(self.tcx, did);",
    replace: "        let own = false;",
    tests: ["test/declarations-libraries.test.ts", "-t", "declared as written"],
  },
  {
    name: "written-alias-declared-as-rust",
    breaks:
      "an alias whose `#[rust_js::types]` writes a union is declared as its Rust type, `export type ReactServerValue = unknown;` (rd538)",
    file: "src/lower/declarations/aliases.rs",
    find: ".filter(|_| declares_written_alias(self.tcx, def_id))",
    replace: ".filter(|_| false)",
    tests: ["test/declarations-libraries.test.ts", "-t", "declared as written"],
  },
  {
    name: "written-private-alias-named",
    breaks:
      "a private alias, which isn't declared, is named, and TypeScript finds no `Depth`",
    file: "src/lower/declarations/aliases.rs",
    find: "                if !self.tcx.visibility(did).is_public() {\n                    return None;\n                }\n                let home",
    replace:
      "                if false {\n                    return None;\n                }\n                let home",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "declarations name a type alias",
    ],
  },
  {
    name: "library-plain-alias-expanded",
    breaks: "a library's plain alias is written out where it's used, `Map<string, unknown>`, not named, `TemporaryReferenceSet` (rd642)",
    file: "src/lower/declarations/aliases.rs",
    find: "            None => match self.foreign.declared_type(did) {\n",
    replace: "            None => match self.foreign.declared_type(did).filter(|_| own) {\n",
    tests: ["test/declarations-libraries.test.ts", "-t", "type-only re-export"],
  },
];
