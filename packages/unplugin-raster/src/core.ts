import path from "node:path";

export type RasterPluginOptions = {
  entry?: string;
  outfile?: string;
  target?: string;
  sourcemap?: boolean;
  minify?: boolean;
  external?: string[];
};

export type NormalizedRasterPluginOptions = {
  entry: string;
  outfile: string;
  target: string;
  sourcemap: boolean;
  minify: boolean;
  external: string[];
  hostExternal: string[];
  allExternal: string[];
};

export type RollupLikeOutputOptions = {
  file?: string;
  dir?: string;
};

export type RollupLikeOutputChunk = {
  type: "chunk";
  fileName: string;
  imports?: string[];
  dynamicImports?: string[];
};

export type RollupLikeOutputAsset = {
  type: "asset";
  fileName: string;
};

export type RollupLikeOutputBundle = Record<
  string,
  RollupLikeOutputChunk | RollupLikeOutputAsset
>;

export type EsbuildLikeMetafile = {
  outputs: Record<
    string,
    {
      entryPoint?: string;
      imports?: Array<{
        path: string;
        external?: boolean;
      }>;
    }
  >;
};

export type EsbuildLikeBuildResult = {
  metafile?: EsbuildLikeMetafile;
};

export const HOST_EXTERNALS = [
  "react",
  "react/jsx-runtime",
  "raster-js",
  "raster-js/core",
  "raster-js/react",
  "raster-js/component",
  "raster-js/components",
  "react-raster",
] as const;

export const DEFAULT_RASTER_PLUGIN_OPTIONS = {
  entry: "src/main.tsx",
  outfile: "dist/raster/app.js",
  target: "es2022",
  sourcemap: false,
  minify: true,
} as const;

export class RasterPluginError extends Error {
  constructor(message: string) {
    super(`[raster] ${message}`);
    this.name = "RasterPluginError";
  }
}

export function normalizeRasterOptions(
  options: RasterPluginOptions = {}
): NormalizedRasterPluginOptions {
  const entry = nonEmptyString(options.entry, "entry", DEFAULT_RASTER_PLUGIN_OPTIONS.entry);
  const outfile = nonEmptyString(
    options.outfile,
    "outfile",
    DEFAULT_RASTER_PLUGIN_OPTIONS.outfile
  );
  const target = nonEmptyString(
    options.target,
    "target",
    DEFAULT_RASTER_PLUGIN_OPTIONS.target
  );
  const userExternal = uniqueStrings(options.external ?? [], "external");
  const hostExternal = [...HOST_EXTERNALS];

  return {
    entry,
    outfile,
    target,
    sourcemap: options.sourcemap ?? DEFAULT_RASTER_PLUGIN_OPTIONS.sourcemap,
    minify: options.minify ?? DEFAULT_RASTER_PLUGIN_OPTIONS.minify,
    external: userExternal,
    hostExternal,
    allExternal: unique([...hostExternal, ...userExternal]),
  };
}

export function splitOutfile(outfile: string): { outDir: string; fileName: string } {
  const absoluteOutfile = path.resolve(outfile);
  return {
    outDir: path.dirname(absoluteOutfile),
    fileName: path.basename(absoluteOutfile),
  };
}

export function isExternalModule(
  id: string,
  options: NormalizedRasterPluginOptions
): boolean {
  return options.allExternal.includes(id);
}

export function mergeRollupExternal(
  existing: unknown,
  options: NormalizedRasterPluginOptions
): (source: string, importer?: string, isResolved?: boolean) => boolean {
  return (source, importer, isResolved) => {
    if (isExternalModule(source, options)) {
      return true;
    }
    return matchesExistingExternal(existing, source, importer, isResolved);
  };
}

export function validateRollupBundle(
  outputOptions: RollupLikeOutputOptions,
  bundle: RollupLikeOutputBundle,
  options: NormalizedRasterPluginOptions
): void {
  const outputs = Object.values(bundle);
  const assets = outputs.filter(
    (output) => output.type === "asset" && !output.fileName.endsWith(".map")
  );
  if (assets.length > 0) {
    throw new RasterPluginError(
      `asset output is not supported in v1: ${assets
        .map((asset) => asset.fileName)
        .join(", ")}`
    );
  }

  const chunks = outputs.filter((output) => output.type === "chunk");
  if (chunks.length !== 1) {
    throw new RasterPluginError(
      `expected exactly one JS chunk, got ${chunks.length}: ${chunks
        .map((chunk) => chunk.fileName)
        .join(", ")}`
    );
  }

  const [chunk] = chunks;
  assertOutputPathMatches(outputOptions, chunk.fileName, options.outfile);
  const remainingImports = [...(chunk.imports ?? []), ...(chunk.dynamicImports ?? [])].filter(
    (id) => id !== chunk.fileName
  );
  assertOnlyAllowedImports(remainingImports, options);
}

export function validateEsbuildMetafile(
  result: EsbuildLikeBuildResult,
  options: NormalizedRasterPluginOptions
): void {
  const metafile = result.metafile;
  if (!metafile) {
    throw new RasterPluginError("esbuild metafile is required to validate Raster bundle output");
  }

  const outputs = Object.entries(metafile.outputs);
  const jsOutputs = outputs.filter(([file]) => file.endsWith(".js"));
  const assetOutputs = outputs.filter(
    ([file]) => !file.endsWith(".js") && !file.endsWith(".map")
  );

  if (assetOutputs.length > 0) {
    throw new RasterPluginError(
      `asset output is not supported in v1: ${assetOutputs
        .map(([file]) => file)
        .join(", ")}`
    );
  }

  if (jsOutputs.length !== 1) {
    throw new RasterPluginError(
      `expected exactly one JS output, got ${jsOutputs.length}: ${jsOutputs
        .map(([file]) => file)
        .join(", ")}`
    );
  }

  const [actualFile, output] = jsOutputs[0];
  assertSamePath(actualFile, options.outfile, "esbuild output");
  assertOnlyAllowedImports(
    (output.imports ?? [])
      .filter((entry) => entry.external)
      .map((entry) => entry.path),
    options
  );
}

function nonEmptyString(value: string | undefined, name: string, fallback: string): string {
  const resolved = value ?? fallback;
  if (typeof resolved !== "string" || resolved.trim() === "") {
    throw new RasterPluginError(`${name} must be a non-empty string`);
  }
  return resolved;
}

function uniqueStrings(values: string[], name: string): string[] {
  for (const value of values) {
    if (typeof value !== "string" || value.trim() === "") {
      throw new RasterPluginError(`${name} entries must be non-empty strings`);
    }
  }
  return unique(values);
}

function unique(values: string[]): string[] {
  return [...new Set(values)];
}

function matchesExistingExternal(
  existing: unknown,
  source: string,
  importer: string | undefined,
  isResolved: boolean | undefined
): boolean {
  if (typeof existing === "function") {
    return Boolean(
      (existing as (source: string, importer?: string, isResolved?: boolean) => boolean)(
        source,
        importer,
        isResolved
      )
    );
  }
  if (typeof existing === "string") {
    return existing === source;
  }
  if (existing instanceof RegExp) {
    return existing.test(source);
  }
  if (Array.isArray(existing)) {
    return existing.some((entry) => matchesExistingExternal(entry, source, importer, isResolved));
  }
  return false;
}

function assertOutputPathMatches(
  outputOptions: RollupLikeOutputOptions,
  fileName: string,
  outfile: string
): void {
  const actual = outputOptions.file
    ? outputOptions.file
    : path.join(outputOptions.dir ?? process.cwd(), fileName);
  assertSamePath(actual, outfile, "Rollup output");
}

function assertSamePath(actual: string, expected: string, label: string): void {
  if (normalizePath(actual) !== normalizePath(expected)) {
    throw new RasterPluginError(
      `${label} path must match outfile: expected ${path.resolve(expected)}, got ${path.resolve(
        actual
      )}`
    );
  }
}

function assertOnlyAllowedImports(
  imports: string[],
  options: NormalizedRasterPluginOptions
): void {
  const unexpected = imports.filter((id) => !isExternalModule(id, options));
  if (unexpected.length > 0) {
    throw new RasterPluginError(
      `bundle contains imports that are not declared as Raster host or user externals: ${unique(
        unexpected
      ).join(", ")}`
    );
  }
}

function normalizePath(value: string): string {
  return path.resolve(value).replaceAll("\\", "/");
}
