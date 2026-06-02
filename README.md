# Raster

Build native GPUI apps with React and TypeScript.

Raster lets you write UI with familiar React components while rendering through
GPUI instead of a browser. You describe your interface in TSX, Raster bundles it
for the native host runtime, and the Rust app opens a GPUI window.

```tsx
import { createRoot } from "raster/react";
import { Button, Text, View } from "raster/components";

function App() {
  return (
    <View style={{ padding: 16, gap: 8 }}>
      <Text>Hello from GPUI</Text>
      <Button onClick={() => console.log("clicked")}>Click me</Button>
    </View>
  );
}

createRoot({ width: 800, height: 600 }).render(<App />);
```