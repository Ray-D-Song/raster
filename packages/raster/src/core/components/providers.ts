import type { ReactElement } from "react";
import { jsx } from "react/jsx-runtime";
import type { ConfigProviderProps } from "../types/index.js";

export function ConfigProvider({
  theme = {},
  text = {},
  resources = {},
  children,
  ...events
}: ConfigProviderProps): ReactElement {
  return jsx("ConfigProvider", {
    theme,
    text,
    resources,
    children,
    ...events,
  });
}

