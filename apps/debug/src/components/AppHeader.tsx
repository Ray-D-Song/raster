import { Avatar, Icon, Text, View } from "raster-js/components";
import { vitalityColors } from "../data";
import { type AppTheme, row, spaceBetween } from "../styles";

interface AppHeaderProps {
  theme: AppTheme;
  avatarUrl: string;
  displayName?: string;
  compact?: boolean;
}

export function AppHeader({ theme, avatarUrl, displayName, compact = false }: AppHeaderProps) {
  return (
    <View
      style={{
        ...spaceBetween,
        height: 64,
        padding: { top: 0, right: 20, bottom: 0, left: 20 },
        backgroundColor: theme.background,
        borderBottomWidth: 1,
        borderColor: theme.border,
      }}
    >
      <View style={{ ...row, gap: 12 }}>
        <Avatar src={avatarUrl} size={compact ? "small" : "medium"} />
        <Text
          style={{
            fontSize: compact ? 20 : 24,
            fontWeight: "700",
            color: vitalityColors.primary,
          }}
        >
          VitalTrack
        </Text>
        {displayName ? (
          <Text style={{ color: theme.mutedForeground, fontSize: 12 }}>{displayName}</Text>
        ) : null}
      </View>
      <View
        style={{
          width: 40,
          height: 40,
          borderRadius: 20,
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        <Icon name="bell" color={vitalityColors.primary} size="medium" />
      </View>
    </View>
  );
}