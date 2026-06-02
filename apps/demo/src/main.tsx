import { useState } from "react";
import { createRoot } from "raster/react";
import { Button, Text, View } from "raster/components";

const App = () => {
  const [count, setCount] = useState(0);

  return (
    <View
      style={{
        width: "100%",
        height: "100%",
        alignItems: "center",
        justifyContent: "center",
        gap: 12,
      }}
    >
      <Text style={{ fontSize: 24 }}>Count: {count}</Text>
      <Button onClick={() => setCount((value) => value + 1)}>Increment</Button>
    </View>
  );
};

const root = createRoot({
  width: 800,
  height: 600,
  perfdetect: true,
});

root.render(<App />);
