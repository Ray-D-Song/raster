import { Icon, Text, View } from "raster-js/components";
import type { IconifyIcon } from "raster-js/components";
import { vitalityColors } from "../data";

interface SectionTitleProps {
  src: IconifyIcon;
  title: string;
}

const titleText = {
  fontSize: 20,
  fontWeight: "600" as const,
  color: vitalityColors.onSurface,
};

export function SectionTitle({ src, title }: SectionTitleProps) {
  return (
    <View style={{ flexDirection: "row", alignItems: "center", gap: 8, display: "flex" }}>
      <Icon src={src} color={vitalityColors.primary} size={20} />
      <View style={{ height: 20, justifyContent: "center" }}>
        <Text style={titleText}>{title}</Text>
      </View>
    </View>
  );
}