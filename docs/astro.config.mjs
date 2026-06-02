// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

// https://astro.build/config
export default defineConfig({
  integrations: [
    starlight({
      title: "Raster Docs",
      defaultLocale: "root",
      locales: {
        root: {
          label: "English",
          lang: "en",
        },
      },
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/ray-d-song/raster",
        },
      ],
      sidebar: [
        {
          label: "Start Here",
          items: ["getting-started"],
        },
        {
          label: "Guides",
          items: [
            "guides/react-renderer",
            "guides/config-provider",
            "guides/styling-and-layout",
            "guides/events-and-state",
          ],
        },
        {
          label: "Components",
          items: [
            "components",
            "components/actions",
            "components/forms",
            "components/display",
            "components/layout",
            "components/overlays",
            "components/data-collections",
          ],
        },
        {
          label: "API Reference",
          items: [
            "reference/react",
            "reference/core",
            "reference/components",
            "reference/unplugin-raster",
          ],
        },
        {
          label: "Concepts",
          items: ["concepts/architecture", "concepts/runtime-contracts"],
        },
      ],
    }),
  ],
});
