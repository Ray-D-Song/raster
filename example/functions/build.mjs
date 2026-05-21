import esbuild from "esbuild";
import fs from "node:fs/promises";
import path from "node:path";

const OUTDIR = "build";

await fs.rm(OUTDIR, { recursive: true, force: true });

async function buildReact() {
  const outbase = path.join(OUTDIR, "react");
  const outfile = path.join(outbase, "index.mjs");

  await fs.mkdir(outbase, { recursive: true });
  await fs.copyFile("src/react/index.html", path.join(outbase, "index.html"));

  const devMode = process.argv.slice(2)[0] == "--dev";

  await esbuild.build({
    entryPoints: {
      index: "src/ssr.ts",
      app: "src/react/index.tsx",
    },
    logLevel: "info",
    ...(!devMode && {
      platform: "node",
    }),
    target: "es2023",
    format: devMode ? "cjs" : "esm",
    define: {
      "process.env.NODE_ENV": JSON.stringify("production"),
    },
    loader: {
      ".svg": "file",
    },
    bundle: true,
    outdir: outbase,
  });

  await fs.rename(path.join(outbase, "index.js"), outfile);
  await fs.readFile(outfile).then((data) => {
    const indexSource = `import { createRequire } from "node:module";\nconst require = createRequire(import.meta.url);\n${data.toString()}`;
    return fs.writeFile(outfile, indexSource);
  });
}

await buildReact();
