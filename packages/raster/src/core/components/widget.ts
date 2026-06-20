import type { ReactElement } from "react";
import { jsx } from "react/jsx-runtime";
import type { WidgetProps } from "../types/index.js";

export function Widget({ name, props = {}, queries = {}, style, children, ...events }: WidgetProps): ReactElement {
  return jsx("Widget", {
    name,
    props,
    queries,
    style,
    children,
    ...events,
  });
}

