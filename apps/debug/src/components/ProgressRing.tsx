import { PieChart, Text, View } from "raster-js/components";
import { vitalityColors } from "../data";
import { formatWeight } from "../model";
import { labelCaps } from "../styles";
import type { WeightUnit } from "../types";

interface ProgressRingProps {
  current: number;
  target: number;
  progress: number;
  unit: WeightUnit;
}

export function ProgressRing({ current, target, progress, unit }: ProgressRingProps) {
  const remaining = Math.round(progress * 100);

  return (
    <View
      style={{
        alignItems: "center",
        justifyContent: "center",
        width: 288,
        height: 288,
        borderRadius: 24,
        backgroundColor: "#ffffff",
        borderWidth: 1,
        borderColor: "rgba(0, 107, 95, 0.08)",
        overflow: "hidden",
      }}
    >
      <View
        style={{
          position: "absolute",
          top: 0,
          right: 0,
          bottom: 0,
          left: 0,
          backgroundColor: vitalityColors.primaryContainer,
          borderRadius: 24,
          opacity: 0.1,
        }}
      />
      <PieChart
        data={[
          { label: "progress", value: remaining, color: vitalityColors.primary },
          { label: "track", value: 100 - remaining, color: "rgba(0, 107, 95, 0.1)" },
        ]}
        value="value"
        color="color"
        innerRadius={84}
        outerRadius={110}
        width={256}
        height={256}
      />
      <View
        style={{
          position: "absolute",
          alignItems: "center",
          gap: 4,
        }}
      >
        <Text style={{ ...labelCaps, color: vitalityColors.onSurfaceVariant }}>CURRENT</Text>
        <Text style={{ fontSize: 36, fontWeight: "700", color: vitalityColors.onSurface }}>
          {current.toFixed(1)}
          <Text style={{ fontSize: 20, fontWeight: "600" }}> {unit}</Text>
        </Text>
        <View
          style={{
            margin: { top: 4 },
            padding: { top: 4, right: 12, bottom: 4, left: 12 },
            borderRadius: 999,
            backgroundColor: "rgba(45, 212, 191, 0.2)",
          }}
        >
          <Text style={{ ...labelCaps, color: vitalityColors.onPrimaryContainer }}>
            Target: {formatWeight(target, unit)}
          </Text>
        </View>
      </View>
    </View>
  );
}