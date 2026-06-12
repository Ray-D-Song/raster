import { createUnplugin } from "unplugin";

import { buildRasterExecutable, startRasterDev } from "./binary.ts";
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
  let viteWatchMode = false;
  let esbuildWatchMode = false;
  let bundleValidated = false;

  return {
    name: "unplugin-raster",
    vite: {
      config(config) {
        options = normalizeRasterOptions(rawOptions);
        bundleValidated = false;
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
      configResolved(config) {
        viteWatchMode = Boolean(config.build.watch);
      },
      generateBundle(outputOptions, bundle) {
        validateRollupBundle(outputOptions, bundle, options);
        bundleValidated = true;
      },
    },
    closeBundle(this: { meta?: { watchMode?: boolean } }) {
      if (!bundleValidated) {
        return;
      }
      if (viteWatchMode || this.meta?.watchMode || options.watch) {
        startRasterDev(options);
        return;
      }
      return buildRasterExecutable(options);
    },
    rollup: {
      options(inputOptions) {
        options = normalizeRasterOptions(rawOptions);
        bundleValidated = false;
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
        bundleValidated = true;
      },
    },
    rolldown: {
      options(inputOptions) {
        options = normalizeRasterOptions(rawOptions);
        bundleValidated = false;
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
        bundleValidated = true;
      },
    },
    esbuild: {
      config(buildOptions) {
        options = normalizeRasterOptions(rawOptions);
        esbuildWatchMode = options.watch || Boolean((buildOptions as { watch?: unknown }).watch);
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
        build.onEnd(async (result) => {
          validateEsbuildMetafile(result, options);
          if (esbuildWatchMode) {
            startRasterDev(options);
            return;
          }
          await buildRasterExecutable(options);
        });
      },
    },
  };
});
