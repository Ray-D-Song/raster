---
title: unplugin-raster
description: Bundler plugin API.
---

`unplugin-raster` builds a Raster app entrypoint into one JavaScript output
file.

## Options

```ts
type RasterPluginOptions = {
  entry?: string;
  outfile?: string;
  target?: string;
  sourcemap?: boolean;
  minify?: boolean;
  external?: string[];
};
```

## Defaults

| Option | Default |
| --- | --- |
| `entry` | `src/main.tsx` |
| `outfile` | `target/raster/app.js` |
| `target` | `es2022` |
| `sourcemap` | `true` |
| `minify` | `false` |

## Entry Points

The package exposes plugin entrypoints for Vite, Rollup, Rolldown, and esbuild.

```ts
import raster from "unplugin-raster/vite";
```

The plugin keeps Raster host modules, including `react`, `react/jsx-runtime`,
and `raster-js/*`, external so the Rust runtime can provide them.
