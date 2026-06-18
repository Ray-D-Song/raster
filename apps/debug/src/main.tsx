import { createRoot } from "raster-js/react";
import { App } from "./App";

const root = createRoot({
  width: 402,
  height: 874,
});

root.render(<App />);
