import { Image, Text, View } from "raster-js/components";
import type { RasterStyle } from "raster-js/components";

const MOUNTAIN_IMAGE =
  "https://images.unsplash.com/photo-1506905925346-21bda4d32df4?auto=format&fit=crop&w=480&h=320&q=80";

const glassOverlayBase: RasterStyle = {
  width: 88,
  height: 88,
  borderRadius: 6,
  backgroundColor: "rgba(255, 255, 255, 0.3)",
  borderWidth: 1,
  borderColor: "rgba(255, 255, 255, 0.55)",
};

interface BackdropBlurPanelProps {
  label: string;
  blurRadius?: number;
}

function BackdropBlurPanel({ label, blurRadius }: BackdropBlurPanelProps) {
  return (
    <View style={{ flex: 1, minWidth: 104, gap: 8, alignItems: "center" }}>
      <Text style={{ fontSize: 11, fontWeight: "500", color: "#6b7280" }}>{label}</Text>
      <View
        style={{
          width: "100%",
          height: 148,
          position: "relative",
          borderRadius: 12,
          overflow: "hidden",
          boxShadow: "md",
        }}
      >
        <Image
          src={MOUNTAIN_IMAGE}
          objectFit="cover"
          style={{
            position: "absolute",
            top: 0,
            right: 0,
            bottom: 0,
            left: 0,
            width: "100%",
            height: "100%",
          }}
        />
        <View
          style={{
            position: "absolute",
            top: 0,
            right: 0,
            bottom: 0,
            left: 0,
            alignItems: "center",
            justifyContent: "center",
          }}
        >
          <View
            style={{
              ...glassOverlayBase,
              ...(blurRadius != null ? { backdropBlur: blurRadius } : {}),
            }}
          />
        </View>
      </View>
    </View>
  );
}

export function BackdropBlurDemo() {
  return (
    <View style={{ gap: 12 }}>
      <Text style={{ fontSize: 13, fontWeight: "600", color: "#374151" }}>backdrop-filter</Text>
      <View style={{ flexDirection: "row", gap: 12, flexWrap: "wrap" }}>
        <BackdropBlurPanel label="backdrop-blur-sm" blurRadius={8} />
        <BackdropBlurPanel label="bg-white/30" />
        <BackdropBlurPanel label="backdrop-blur-xl" blurRadius={40} />
      </View>
    </View>
  );
}