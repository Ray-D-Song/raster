import { createUnplugin } from "unplugin";

import {
  mergeRollupExternal,
  normalizeRasterOptions,
  splitOutfile,
  validateEsbuildMetafile,
  validateRollupBundle,
  type RasterPluginOptions,
} from "./core.ts";

export const rasterUnplugin = createUnplugin<RasterPluginOptions>((rawOptions = {}) => {
  let options = normalizeRasterOptions(rawOptions);

  return {
    name: "unplugin-raster",
    vite: {
      config(config) {
        options = normalizeRasterOptions(rawOptions);
        const output = splitOutfile(options.outfile);

        return {
          build: {
            target: options.target,
            sourcemap: options.sourcemap,
            minify: options.minify,
            outDir: output.outDir,
            emptyOutDir: true,
            rollupOptions: {
              input: options.entry,
              external: mergeRollupExternal(config.build?.rollupOptions?.external, options),
              output: {
                format: "esm",
                entryFileNames: output.fileName,
                codeSplitting: false,
              },
            },
          },
        };
      },
      generateBundle(outputOptions, bundle) {
        validateRollupBundle(outputOptions, bundle, options);
      },
    },
    rollup: {
      options(inputOptions) {
        options = normalizeRasterOptions(rawOptions);
        return {
          ...inputOptions,
          input: options.entry,
          external: mergeRollupExternal(inputOptions.external, options),
        };
      },
      outputOptions(outputOptions) {
        return {
          ...outputOptions,
          file: options.outfile,
          format: "esm",
          inlineDynamicImports: true,
        };
      },
      generateBundle(outputOptions, bundle) {
        validateRollupBundle(outputOptions, bundle, options);
      },
    },
    rolldown: {
      options(inputOptions) {
        options = normalizeRasterOptions(rawOptions);
        return {
          ...inputOptions,
          input: options.entry,
          external: mergeRollupExternal(inputOptions.external, options),
        };
      },
      outputOptions(outputOptions) {
        return {
          ...outputOptions,
          file: options.outfile,
          format: "esm",
          codeSplitting: false,
          minify: options.minify,
        };
      },
      generateBundle(outputOptions, bundle) {
        validateRollupBundle(outputOptions, bundle, options);
      },
    },
    esbuild: {
      config(buildOptions) {
        options = normalizeRasterOptions(rawOptions);
        buildOptions.entryPoints = [options.entry];
        buildOptions.outfile = options.outfile;
        buildOptions.bundle = true;
        buildOptions.platform = "neutral";
        buildOptions.format = "esm";
        buildOptions.splitting = false;
        buildOptions.external = [
          ...new Set([...(buildOptions.external ?? []), ...options.allExternal]),
        ];
        buildOptions.target = options.target;
        buildOptions.minify = options.minify;
        buildOptions.sourcemap = options.sourcemap;
        buildOptions.metafile = true;
      },
      setup(build) {
        build.onEnd((result) => {
          validateEsbuildMetafile(result, options);
        });
      },
    },
  };
});
