import { Icon, Text, View } from "raster-js/components";
import { vitalityColors } from "../data";
import {
  deltaTone,
  entryDelta,
  formatDateLabel,
  formatTimeLabel,
  formatWeight,
  formatWeightDelta,
} from "../model";
import { cardShadow, labelCaps, spaceBetween } from "../styles";
import type { WeightEntry, WeightUnit } from "../types";

interface WeightEntryCardProps {
  entry: WeightEntry;
  previous: WeightEntry | null;
  unit: WeightUnit;
  onClick?: () => void;
}

export function WeightEntryCard({ entry, previous, unit, onClick }: WeightEntryCardProps) {
  const delta = entryDelta(entry, previous, unit);
  const tone = deltaTone(delta);
  const badgeBackground =
    tone === "up"
      ? vitalityColors.errorContainer
      : tone === "down"
        ? vitalityColors.primaryContainer
        : vitalityColors.surfaceContainerHigh;
  const badgeForeground =
    tone === "up"
      ? vitalityColors.onErrorContainer
      : tone === "down"
        ? vitalityColors.onPrimaryContainer
        : vitalityColors.onSurfaceVariant;
  const badgeIcon = tone === "up" ? "arrow-up" : tone === "down" ? "arrow-down" : "dash";

  return (
    <View
      onClick={onClick}
      style={{
        backgroundColor: "#ffffff",
        borderRadius: 24,
        borderWidth: 1,
        borderColor: "rgba(186, 202, 197, 0.2)",
        boxShadow: cardShadow,
        padding: 20,
        gap: 16,
      }}
    >
      <View style={spaceBetween}>
        <View style={{ gap: 2 }}>
          <Text style={{ ...labelCaps, color: vitalityColors.primary }}>{formatDateLabel(entry.date)}</Text>
          <Text style={{ color: vitalityColors.onSurfaceVariant, fontSize: 14 }}>{formatTimeLabel(entry.time)}</Text>
        </View>
        {delta != null ? (
          <View
            style={{
              flexDirection: "row",
              alignItems: "center",
              gap: 4,
              padding: { top: 4, right: 12, bottom: 4, left: 12 },
              borderRadius: 999,
              backgroundColor: badgeBackground,
            }}
          >
            <Icon name={badgeIcon} color={badgeForeground} size="small" />
            <Text style={{ ...labelCaps, color: badgeForeground }}>{formatWeightDelta(delta, unit)}</Text>
          </View>
        ) : null}
      </View>

      <View style={{ flexDirection: "row", gap: 16 }}>
        <View
          style={{
            flex: 1,
            backgroundColor: vitalityColors.surfaceContainerLow,
            borderRadius: 12,
            padding: 12,
            gap: 4,
          }}
        >
          <Text style={{ ...labelCaps, color: vitalityColors.onSurfaceVariant }}>WEIGHT</Text>
          <Text style={{ fontSize: 28, fontWeight: "700", color: vitalityColors.onSurface }}>
            {entry.weight.toFixed(1)}
            <Text style={{ fontSize: 16, fontWeight: "400", color: vitalityColors.onSurfaceVariant }}> {unit}</Text>
          </Text>
        </View>
        <View
          style={{
            flex: 1,
            backgroundColor: vitalityColors.surfaceContainerLow,
            borderRadius: 12,
            padding: 12,
            gap: 4,
          }}
        >
          <Text style={{ ...labelCaps, color: vitalityColors.onSurfaceVariant }}>BODY FAT</Text>
          <Text style={{ fontSize: 28, fontWeight: "700", color: vitalityColors.onSurface }}>
            {entry.bodyFat.toFixed(1)}
            <Text style={{ fontSize: 16, fontWeight: "400", color: vitalityColors.onSurfaceVariant }}> %</Text>
          </Text>
        </View>
      </View>

      {entry.note ? (
        <Text style={{ color: vitalityColors.onSurfaceVariant, fontSize: 13 }}>{entry.note}</Text>
      ) : null}
    </View>
  );
}