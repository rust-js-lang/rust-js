// The compiler's versioned build result. Shared by native and WASI hosts.
function validCompiler(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    && typeof value.version === "string" && value.version.length > 0
    && typeof value.toolchain === "string" && value.toolchain.length > 0
    && value.abi === 1;
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
  const typeParam = value => object(value) && Number.isInteger(value.index) && facts(value.marks, ["drop", "copy", "clone", "no_drop", "unboxed"])
    && (value.facts === undefined || strings(value.facts)) && (value.stepped === undefined || strings(value.stepped));
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
    if (!object(library) || library.version !== 10 || typeof library.name !== "string" || !library.name
        || typeof library.crate_hash !== "string" || !library.crate_hash
        || !Array.isArray(library.inputs) || !library.inputs.every(fingerprint)
        || !strings(library.impls) || !strings(library.libraries)
        // ADR 0852: what the `Rc`s it counts point at; none where it's absent.
        || !(library.counted_rcs === undefined || Array.isArray(library.counted_rcs) && library.counted_rcs.every(rcShape))
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
          && facts(item.facts, ["fails", "takes_location", "rest", "lazy", "variable"]))
        // What it says of each of its structs' fields, by key (ADRs 0863,
        // amended, 0906).
        || !(library.fields === undefined || Array.isArray(library.fields) && library.fields.every(field => object(field)
          && typeof field.key === "string" && facts(field.facts, ["quiet"])))) {
      throw new Error("Invalid rust-js library contract; expected library ABI 10");
    }
  }
  if (!path(result.input) || !path(result.output) || !paths(result.sources)
      || !Array.isArray(result.modules) || !result.modules.every(module => object(module)
        && strings(module.module) && path(module.file) && path(module.map)
        && (module.types === undefined || path(module.types))
        && (module.source === null || path(module.source)) && paths(module.imports)
        && (module.in_source === undefined || module.in_source === null || path(module.in_source))
        && (module.exports === undefined || strings(module.exports))
        && links(module.links) && links(module.type_links))
      || !Array.isArray(result.artifacts) || !result.artifacts.every(artifact => object(artifact)
        && path(artifact.file) && typeof artifact.hash === "string" && /^[0-9a-f]{16}$/.test(artifact.hash))
      || (result.stale !== undefined && !(Array.isArray(result.stale) && result.stale.every(fingerprint)))
      || (result.tests !== undefined && !path(result.tests))) {
    throw new Error("Invalid rust-js manifest: expected absolute paths, modules and fingerprinted artifacts");
  }
  const artifacts = new Set(result.artifacts.map(artifact => artifact.file));
  const modules = new Set(result.modules.map(module => module.file));
  if (artifacts.size !== result.artifacts.length
      || modules.size !== result.modules.length
      || result.modules.some(module => !artifacts.has(module.file) || !artifacts.has(module.map)
        || (module.types !== undefined && !artifacts.has(module.types))
        || module.imports.some(file => !modules.has(file)))
      || (result.tests !== undefined && !artifacts.has(result.tests))) {
    throw new Error("Invalid rust-js manifest: inconsistent module artifacts or imports");
  }
  return result;
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
    ...(manifest.library === undefined ? {} : { library: {
      ...manifest.library, inputs: manifest.library.inputs.map(input => ({ ...input, file: map(input.file) })),
    } }),
  };
}
