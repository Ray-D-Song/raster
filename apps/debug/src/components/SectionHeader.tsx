import { Text, View } from "raster-js/components";
import { type AppTheme, secondaryText, spaceBetween } from "../styles";


interface SectionHeaderProps {
  title: string;
  detail?: string;
  theme: AppTheme;
}

export function SectionHeader({ title, detail, theme }: SectionHeaderProps) {
  return (
    <View style={spaceBetween}>
      <Text style={{ fontSize: 16, fontWeight: "700" }}>{title}</Text>
      {detail ? <Text style={{ color: secondaryText(theme), fontSize: 12 }}>{detail}</Text> : null}
    </View>
  );
}
