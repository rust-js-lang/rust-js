// The compiler's versioned build result. Shared by native and WASI hosts.

// The library ABI a contract is of: `src/library.rs`' `VERSION`, its
// authority, mirrored here across the compiler's boundary; every library
// a test builds and reads holds the two equal.
export const LIBRARY_ABI = 47;

// The facts a contract's sets hold, by name: `src/library.rs`' `ItemFact`,
// `ParamFact` and `FieldFact`, their authority, mirrored here across the
// compiler's boundary, as `LIBRARY_ABI` is; test/manifest.test.ts reads
// both and holds them equal (#1638).
export const ITEM_FACTS = ["fails", "takes_location", "arguments", "this", "rest", "lazy", "variable", "returns_js", "at_hand"];
export const PARAM_FACTS = ["drop", "copy", "clone", "no_drop", "unboxed", "shown_pretty", "shown_hex", "counted", "written_into"];
export const FIELD_FACTS = ["quiet", "rust_functions"];

function validCompiler(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    && typeof value.version === "string" && value.version.length > 0
    && typeof value.toolchain === "string" && value.toolchain.length > 0
    && value.abi === 1
    // ADR 1003: which build of it, absent of `@rust-js/std`'s.
    && (value.build === undefined || typeof value.build === "string" && /^[0-9a-f]{16}$/.test(value.build));
}

export function parseCompilerIdentity(text) {
  const value = JSON.parse(text);
  if (!validCompiler(value)) throw new Error("Unsupported rust-js compiler identity or ABI; expected ABI 1");
  return value;
}

export function parseManifest(text) {
  let result;
  try { result = JSON.parse(text); }
  catch (error) { throw new Error(`Invalid rust-js manifest JSON: ${error.message}`); }
  const object = value => value !== null && typeof value === "object" && !Array.isArray(value);
  const strings = value => Array.isArray(value) && value.every(item => typeof item === "string");
  const path = value => typeof value === "string" && (/^\//.test(value) || /^[A-Za-z]:[\\/]/.test(value));
  const paths = value => strings(value) && value.every(path);
  // ADR 0852: a type an `Rc` points at, its constructor's key and what
  // it's made of, or any type where it has no `of`.
  const rcShape = value => object(value) && (value.of === undefined || typeof value.of === "string")
    && (value.args === undefined || Array.isArray(value.args) && value.args.every(rcShape));
  // ADR 0100, amended: what a function asks of one of its type parameters,
  // by its index; each part absent where it asks none of it.
  // ADR 0906: a fact set, each by its name, of the names this ABI knows.
  const facts = (value, known) => value === undefined || Array.isArray(value) && value.every(fact => known.includes(fact));
  // #737: a type built of a function's slots, by what it's made of.
  const template = value => object(value) && Object.keys(value).length === 1 && (
    Number.isInteger(value.slot)
    || object(value.adt) && typeof value.adt.key === "string" && Array.isArray(value.adt.args) && value.adt.args.every(template)
    || Array.isArray(value.tuple) && value.tuple.every(template)
    || object(value.ref) && typeof value.ref.mutable === "boolean" && template(value.ref.to)
    || object(value.array) && Number.isInteger(value.array.len) && template(value.array.of)
    || typeof value.primitive === "string");
  const built = value => value === undefined || object(value) && template(value.of);
  const typeParam = value => object(value) && Number.isInteger(value.index) && facts(value.marks, PARAM_FACTS)
    && (value.facts === undefined || strings(value.facts)) && (value.stepped === undefined || strings(value.stepped))
    && built(value.built);
  const fingerprint = value => object(value) && path(value.file)
    && typeof value.hash === "string" && /^[0-9a-f]{16}$/.test(value.hash);
  // Where a file names another's, by the compiler's parse of it (ADR 0101):
  // absent of an older compiler's.
  const links = value => value === undefined || Array.isArray(value) && value.every(link => object(link)
    && Number.isInteger(link.start) && Number.isInteger(link.end) && 0 <= link.start && link.start <= link.end
    && path(link.file));
  if (!object(result) || result.version !== 1) {
    throw new Error(`Unsupported rust-js manifest version ${result?.version}; expected 1`);
  }
  if (result.compiler !== undefined && !validCompiler(result.compiler)) {
    throw new Error("Unsupported rust-js compiler identity or ABI; expected ABI 1");
  }
  if (result.library !== undefined) {
    const library = result.library;
    // ADR 0100: each item another crate can reach, by rustc's key for it.
    if (!object(library) || library.version !== LIBRARY_ABI || typeof library.name !== "string" || !library.name
        // #1130: its crate's identity, which with its name it's known by.
        || typeof library.crate_id !== "string" || !library.crate_id
        || typeof library.crate_hash !== "string" || !library.crate_hash
        || !Array.isArray(library.inputs) || !library.inputs.every(fingerprint)
        || !strings(library.impls) || !strings(library.libraries)
        // ADR 1071: its enums told apart by their keys; none where it's absent.
        || !(library.keyed === undefined || strings(library.keyed))
        // ADR 0852: what the `Rc`s it counts point at; none where it's absent.
        || !(library.counted_rcs === undefined || Array.isArray(library.counted_rcs) && library.counted_rcs.every(rcShape))
        // #2112's change D: what each function and impl leaves copies of
        // out, by key, each a list of shapes; none where it's absent.
        || !(library.elided === undefined || Array.isArray(library.elided) && library.elided.every(elided => object(elided)
          && typeof elided.key === "string" && Array.isArray(elided.shapes) && elided.shapes.every(rcShape)))
        // #2335: the `#[track_caller]` values its bodies don't say the calls
        // of, each as written, where, and its closure; none where it's absent.
        || !(library.untold_locations === undefined || Array.isArray(library.untold_locations)
          && library.untold_locations.every(untold => object(untold)
            && ["value", "at", "closure"].every(part => typeof untold[part] === "string")))
        // ADR 0863: what it does with its `RefCell`s, each a list of shapes.
        || !(library.ref_cells === undefined || object(library.ref_cells)
          && ["boxed", "counted", "unchecked", "handed"].every(kind => library.ref_cells[kind] === undefined
            || Array.isArray(library.ref_cells[kind]) && library.ref_cells[kind].every(rcShape)))
        || !Array.isArray(library.items) || !library.items.every(item => object(item)
          && typeof item.key === "string" && typeof item.rust_path === "string" && strings(item.module)
          && typeof item.export === "string" && (item.member === null || typeof item.member === "string")
          // What it asks of each type parameter it asks anything of, by its
          // index: a drop (ADR 0098), a copy (ADR 0914), a clone (ADR 0914, amended), none (ADR 0190),
          // a `Some` of it unboxed (ADR 0051, amended), type facts (ADR 0145), and the traits it
          // steps it by, each by its key (ADR 0548).
          && (item.params === undefined || Array.isArray(item.params) && item.params.every(typeParam))
          // What else its calls give it or read of it (ADR 0906).
          && facts(item.facts, ITEM_FACTS)
          // Its own nullable parameters each of its crate's calls gives
          // `null` or a value, by index (ADR 0861, amended).
          && (item.nulled_params === undefined || Array.isArray(item.nulled_params)
            && item.nulled_params.every(index => Number.isInteger(index) && index >= 0))
          // The parameters what it returns alone owns, and of those, the
          // ones nothing it runs before it returns may call, by index (ADR
          // 0317, amended).
          && ["owned_by_return", "quiet_by_return"].every(key => item[key] === undefined
            || Array.isArray(item[key]) && item[key].every(index => Number.isInteger(index) && index >= 0)))
        // What it says of each of its structs' fields, by key (ADRs 0863,
        // amended, 0906).
        || !(library.fields === undefined || Array.isArray(library.fields) && library.fields.every(field => object(field)
          && typeof field.key === "string" && facts(field.facts, FIELD_FACTS)))
        // What its traits' dictionaries carry of each associated type, by
        // key (ADR 0145, amended).
        || !(library.associated === undefined || Array.isArray(library.associated)
          && library.associated.every(associated => object(associated)
            && typeof associated.key === "string" && strings(associated.facts)))
        // The structs its code makes values of, by key (ADR 1022, amended).
        || !(library.built === undefined || strings(library.built))
        // Each closure it clones, or gives to clone, by key, and whether
        // it's cloned apart (ADR 0522, amended; #1202).
        || !(library.cloned_closures === undefined || Array.isArray(library.cloned_closures)
          && library.cloned_closures.every(closure => object(closure)
            && typeof closure.key === "string" && typeof closure.apart === "boolean"))) {
      throw new Error(`Invalid rust-js library contract; expected library ABI ${LIBRARY_ABI}`);
    }
  }
  if (!path(result.input) || !path(result.output) || !paths(result.sources)
      || !Array.isArray(result.modules) || !result.modules.every(module => object(module)
        && strings(module.module) && path(module.file) && path(module.map)
        && (module.types === undefined || path(module.types))
        // ADR 1005: of types alone, its `.d.ts` alone, written.
        && (module.types_only === undefined || module.types_only === true && module.types !== undefined)
        && (module.source === null || path(module.source)) && paths(module.imports)
        && (module.in_source === undefined || module.in_source === null || path(module.in_source))
        && (module.exports === undefined || strings(module.exports))
        && links(module.links) && links(module.type_links))
      || !Array.isArray(result.artifacts) || !result.artifacts.every(artifact => object(artifact)
        && path(artifact.file) && typeof artifact.hash === "string" && /^[0-9a-f]{16}$/.test(artifact.hash))
      || (result.stale !== undefined && !(Array.isArray(result.stale) && result.stale.every(fingerprint)))
      // ADR 1076: a test build's test files, and the runner they're for.
      || (result.tests !== undefined && !(object(result.tests) && typeof result.tests.runner === "string" && typeof result.tests.module === "string"
        && paths(result.tests.files)))) {
    throw new Error("Invalid rust-js manifest: expected absolute paths, modules and fingerprinted artifacts");
  }
  const artifacts = new Set(result.artifacts.map(artifact => artifact.file));
  const modules = new Set(result.modules.map(module => module.file));
  if (artifacts.size !== result.artifacts.length
      || modules.size !== result.modules.length
      || result.modules.some(module => !module.types_only && (!artifacts.has(module.file) || !artifacts.has(module.map))
        || (module.types !== undefined && !artifacts.has(module.types))
        || module.imports.some(file => !modules.has(file)))
      || (result.tests !== undefined && !result.tests.files.every(file => artifacts.has(file)))) {
    throw new Error("Invalid rust-js manifest: inconsistent module artifacts or imports");
  }
  // #2481: each path the crate offers JS, by its Rust path and JS's, its
  // kind and its types (rust_js_program::offered); none where it's absent.
  if (result.offered_paths !== undefined && !(Array.isArray(result.offered_paths) && result.offered_paths.every(offeredPath))) {
    throw new Error("Invalid rust-js manifest: expected each offered path's Rust and JS paths, its kind and its types");
  }
  return result;
}

const OFFERED_KINDS = new Set(["function", "const", "static", "namespace", "type"]);

/** One of `offered_paths` (rust_js_program::offered's `OfferedPath`). */
function offeredPath(path) {
  const names = (value) => Array.isArray(value) && value.length > 0 && value.every((name) => typeof name === "string" && name !== "");
  const flag = (value) => value === undefined || typeof value === "boolean";
  return path !== null && typeof path === "object" && names(path.rust) && names(path.js) && path.rust.length === path.js.length
    && OFFERED_KINDS.has(path.kind)
    && (path.params === undefined || Array.isArray(path.params) && path.params.every(offeredType))
    && (path.ty === undefined || offeredType(path.ty))
    && flag(path.generic) && flag(path.located) && flag(path.of_no_value) && flag(path.sealed)
    && (path.unoffered_fields === undefined || path.unoffered_fields === "symbol_keyed")
    // Oracle B's: a result's `None`, a type's definition, a transparent struct's field.
    && (path.result_none === undefined || offeredNones.has(path.result_none))
    && (path.def === undefined || offeredDef(path.def))
    && (path.transparent === undefined || offeredType(path.transparent))
    && (path.variants === undefined || offeredEnum(path.variants))
    && flag(path.class)
    && (path.positional === undefined || Array.isArray(path.positional) && path.positional.every(offeredType))
    && (path.unit === undefined || path.unit !== null && typeof path.unit === "object" && tagKinds.has(path.unit.kind))
    && (path.type_params === undefined || Number.isInteger(path.type_params) && path.type_params >= 0)
    // #2573: a struct's fields, and a generic function's bounds.
    && (path.fields === undefined || Array.isArray(path.fields) && path.fields.every((field) => field !== null && typeof field === "object"
      && typeof field.rust === "string" && typeof field.js === "string" && typeof field.written === "string" && offeredType(field.ty)
      && (field.none === undefined || offeredNones.has(field.none))
      && (field.property === undefined || offeredType(field.property))))
    && (path.bounds === undefined || Array.isArray(path.bounds) && path.bounds.every((bound) => bound !== null && typeof bound === "object"
      && typeof bound.param === "string"
      && (bound.traits === undefined || Array.isArray(bound.traits) && bound.traits.every((name) => typeof name === "string"))
      && (bound.callable === undefined || offeredType(bound.callable))
      && (bound.impls === undefined || Array.isArray(bound.impls) && bound.impls.every((imp) => imp !== null && typeof imp === "object"
        && typeof imp.dictionary === "string" && offeredType(imp.ty)))));
}

/** An offered path's type (rust_js_program::offered's `OfferedType`), its
 * own `kind` and what each kind holds. */
// What an `Option`'s `None` is in a place (`rust_js_program::offered::OfferedNone`).
const offeredNones = new Set(["plain", "present", "null", "null_optional", "nested", "held"]);

// How JS holds an enum's values (`rust_js_program::offered::OfferedEnum`).
const enumForms = new Set(["tagged", "declared", "untagged", "keyed"]);
const tagKinds = new Set(["str", "num", "bool", "null", "undefined"]);
function offeredEnum(e) {
  return e !== null && typeof e === "object" && enumForms.has(e.form) && typeof e.key === "string" && Array.isArray(e.variants)
    && e.variants.every((v) => v !== null && typeof v === "object" && typeof v.rust === "string"
      && v.tag !== null && typeof v.tag === "object" && tagKinds.has(v.tag.kind)
      && (v.fields === undefined || Array.isArray(v.fields) && v.fields.every((field) => field !== null && typeof field === "object"
        && typeof field.js === "string" && offeredType(field.ty) && (field.none === undefined || offeredNones.has(field.none))
        && (field.property === undefined || offeredType(field.property))))
      && (v.otherwise === undefined || typeof v.otherwise === "boolean"));
}

// Where a type is defined, its crate and its path from that crate's root.
function offeredDef(def) {
  return def !== null && typeof def === "object" && (def.crate === undefined || typeof def.crate === "string")
    && Array.isArray(def.rust) && def.rust.every((name) => typeof name === "string");
}

function offeredType(ty) {
  if (ty === null || typeof ty !== "object") return false;
  switch (ty.kind) {
    case "int":
    case "float":
    case "other":
      return typeof ty.rust === "string";
    case "bool":
    case "char":
    case "unit":
      return true;
    case "str":
      return typeof ty.owned === "boolean";
    case "option":
    case "box":
      return offeredType(ty.of);
    case "ref":
      return offeredType(ty.of) && (ty.mutable === undefined || typeof ty.mutable === "boolean");
    case "fn":
      return Array.isArray(ty.inputs) && ty.inputs.every(offeredType) && offeredType(ty.output);
    case "result":
      return offeredType(ty.ok) && offeredType(ty.err);
    case "array":
      return offeredType(ty.of) && Number.isInteger(ty.len);
    case "rc":
      return offeredType(ty.of) && (ty.counted === undefined || typeof ty.counted === "boolean");
    case "cell":
      return offeredType(ty.of) && (ty.itself === undefined || typeof ty.itself === "boolean");
    case "dyn":
      return Array.isArray(ty.traits) && ty.traits.every((name) => typeof name === "string");
    case "weak":
      return offeredType(ty.of) && (ty.counted === undefined || typeof ty.counted === "boolean");
    case "non_zero":
    case "set":
      return offeredType(ty.of);
    case "pointer":
      return offeredType(ty.of) && (ty.nullable === undefined || typeof ty.nullable === "boolean");
    case "map":
      return offeredType(ty.key) && offeredType(ty.value);
    case "iterator":
      return (ty.back === undefined || typeof ty.back === "boolean") && (ty.len === undefined || typeof ty.len === "boolean");
    case "param":
      return typeof ty.name === "string" && (ty.index === undefined || Number.isInteger(ty.index));
    case "named":
      return offeredDef(ty) && (ty.args === undefined || Array.isArray(ty.args) && ty.args.every(offeredType));
    case "vec":
      return offeredType(ty.of) && typeof ty.borrowed === "boolean";
    case "tuple":
      return Array.isArray(ty.items) && ty.items.every(offeredType);
    default:
      return false;
  }
}

// Remap only paths. Names, fingerprints and future unrelated fields stay intact.
export function mapManifestPaths(manifest, map) {
  return {
    ...manifest,
    input: map(manifest.input), output: map(manifest.output),
    sources: manifest.sources.map(map),
    modules: manifest.modules.map(module => ({
      ...module, file: map(module.file), map: map(module.map),
      ...(module.types === undefined ? {} : { types: map(module.types) }),
      source: module.source === null ? null : map(module.source), imports: module.imports.map(map),
      ...(module.in_source == null ? {} : { in_source: map(module.in_source) }),
      ...(module.links === undefined ? {} : { links: module.links.map(link => ({ ...link, file: map(link.file) })) }),
      ...(module.type_links === undefined ? {} : { type_links: module.type_links.map(link => ({ ...link, file: map(link.file) })) }),
    })),
    artifacts: manifest.artifacts.map(artifact => ({ ...artifact, file: map(artifact.file) })),
    ...(manifest.stale === undefined ? {} : { stale: manifest.stale.map(old => ({ ...old, file: map(old.file) })) }),
    ...(manifest.tests === undefined ? {} : { tests: { ...manifest.tests, files: manifest.tests.files.map(map) } }),
    ...(manifest.library === undefined ? {} : { library: {
      ...manifest.library, inputs: manifest.library.inputs.map(input => ({ ...input, file: map(input.file) })),
    } }),
  };
}
