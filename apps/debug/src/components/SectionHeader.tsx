import { Text, View } from "raster-js/components";
import { secondaryText, spaceBetween, textColor } from "../styles";
import type { ThemePreference } from "../types";

interface SectionHeaderProps {
  title: string;
  detail?: string;
  theme: ThemePreference;
}

export function SectionHeader({ title, detail, theme }: SectionHeaderProps) {
  return (
    <View style={spaceBetween}>
      <Text style={{ color: textColor(theme), fontSize: 16, fontWeight: "700" }}>{title}</Text>
      {detail ? <Text style={{ color: secondaryText(theme), fontSize: 12 }}>{detail}</Text> : null}
    </View>
  );
}
