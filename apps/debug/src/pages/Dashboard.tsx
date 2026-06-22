import { BarChart, Button, Icon, Text, View } from "raster-js/components";
import { Card } from "../components/Card";
import { ProgressRing } from "../components/ProgressRing";
import { activityStreakDays, dailyQuote, startWeight, userProfile, vitalityColors } from "../data";
import {
  bmiCategory,
  computeBmi,
  distanceToGoal,
  formatWeightDelta,
  goalProgress,
  lastSevenDays,
  weekChange,
} from "../model";
import { type AppTheme, labelCaps, pagePadding, row, secondaryText, spaceBetween } from "../styles";
import type { AppTab, UserSettings, WeightEntry } from "../types";

interface DashboardProps {
  entries: WeightEntry[];
  settings: UserSettings;
  theme: AppTheme;
  onNavigate: (tab: AppTab) => void;
}

export function Dashboard({ entries, settings, theme, onNavigate }: DashboardProps) {
  const latest = entries.length > 0 ? entries.reduce((a, b) => (a.date > b.date ? a : b)) : null;
  const current = latest?.weight ?? settings.targetWeight;
  const progress = goalProgress(current, settings.targetWeight, startWeight);
  const gap = distanceToGoal(current, settings.targetWeight);
  const bmi = computeBmi(current, userProfile.heightCm);
  const weekly = weekChange(entries);
  const trend = lastSevenDays(entries);
  const maxWeight = Math.max(...trend.map((point) => point.weight), current);
  const minWeight = Math.min(...trend.map((point) => point.weight), current);
  const chartData = trend.map((point, index) => ({
    label: point.label,
    value: point.weight,
    color: index === trend.length - 1 ? vitalityColors.primary : vitalityColors.surfaceContainerHigh,
    height: maxWeight === minWeight ? 80 : ((maxWeight - point.weight) / (maxWeight - minWeight)) * 60 + 40,
  }));

  return (
    <View style={{ backgroundColor: theme.background }}>
      <View style={[pagePadding, { gap: 32 }]}>
        <View style={{ gap: 4 }}>
          <Text style={{ fontSize: 24, fontWeight: "600", color: vitalityColors.onSurface }}>
            Hello, {userProfile.displayName}
          </Text>
          <Text style={{ fontSize: 16, color: vitalityColors.onSurfaceVariant }}>
            You're {gap.toFixed(1)} {settings.unit} away from your goal!
          </Text>
        </View>

        <View style={{ alignItems: "center" }}>
          <ProgressRing current={current} target={settings.targetWeight} progress={progress} unit={settings.unit} />
        </View>

        <View style={{ flexDirection: "row", flexWrap: "wrap", gap: 16 }}>
          <Card theme={theme} style={{ flex: 1, minWidth: 140, height: 128, justifyContent: "space-between" }}>
            <View style={spaceBetween}>
              <Icon name="chart-pie" color={vitalityColors.secondary} size="medium" />
              <Text style={{ ...labelCaps, color: vitalityColors.onSurfaceVariant }}>BMI</Text>
            </View>
            <View style={{ gap: 2 }}>
              <Text style={{ fontSize: 24, fontWeight: "700", color: vitalityColors.onSurface }}>{bmi.toFixed(1)}</Text>
              <Text style={{ fontSize: 12, fontWeight: "600", color: vitalityColors.primary }}>{bmiCategory(bmi)}</Text>
            </View>
          </Card>

          <Card theme={theme} style={{ flex: 1, minWidth: 140, height: 128, justifyContent: "space-between" }}>
            <View style={spaceBetween}>
              <Icon name="arrow-down" color={vitalityColors.error} size="medium" />
              <Text style={{ ...labelCaps, color: vitalityColors.onSurfaceVariant }}>THIS WEEK</Text>
            </View>
            <View style={{ gap: 2 }}>
              <Text style={{ fontSize: 24, fontWeight: "700", color: vitalityColors.onSurface }}>
                {weekly ? formatWeightDelta(weekly.delta, settings.unit) : "—"}
              </Text>
              <Text style={{ fontSize: 12, color: vitalityColors.onSurfaceVariant }}>
                {weekly ? `Down from ${weekly.from.toFixed(1)} ${settings.unit}` : "Log more entries"}
              </Text>
            </View>
          </Card>

          <Card theme={theme} tinted style={{ width: "100%", flexDirection: "row", alignItems: "center" }}>
            <View style={{ flex: 1, gap: 4 }}>
              <Text style={{ ...labelCaps, color: "rgba(0, 87, 77, 0.7)" }}>ACTIVITY STREAK</Text>
              <Text style={{ fontSize: 36, fontWeight: "700", color: vitalityColors.onPrimaryContainer }}>
                {activityStreakDays} Days
              </Text>
              <Text style={{ fontSize: 14, color: "rgba(0, 87, 77, 0.8)" }}>Keep moving, you're on fire!</Text>
            </View>
            <Icon name="star" color={vitalityColors.onPrimaryContainer} size="large" />
          </Card>
        </View>

        <View style={{ gap: 16 }}>
          <View style={spaceBetween}>
            <Text style={{ fontSize: 20, fontWeight: "600", color: vitalityColors.onSurface }}>Weight Trend</Text>
            <Text style={{ ...labelCaps, color: vitalityColors.primary }}>Last 7 Days</Text>
          </View>
          <Card theme={theme}>
            {chartData.length > 0 ? (
              <BarChart
                data={chartData}
                band="label"
                value="height"
                fill="color"
                height={160}
                cornerRadius={8}
                labelAxis
              />
            ) : (
              <Text style={{ color: secondaryText(theme), fontSize: 13 }}>No trend data yet.</Text>
            )}
          </Card>
        </View>

        <View style={{ flexDirection: "row", gap: 0 }}>
          <Card theme={theme} style={{ flex: 1, justifyContent: "space-between", flexDirection: "row", borderTopLeftRadius: 24, borderBottomLeftRadius: 24, gap: 8 }}>
            <View style={{ width: 4, backgroundColor: vitalityColors.primary, borderTopLeftRadius: 24, borderBottomLeftRadius: 24 }} />
            <View style={{ flex: 1, gap: 4, padding: 16 }}>
              <Text style={{ fontSize: 20, fontWeight: "600", fontStyle: "italic", color: vitalityColors.onSurfaceVariant, margin: { bottom: 8 } }}>
                {dailyQuote.text}
              </Text>
              <Text style={{ ...labelCaps, color: vitalityColors.primary }}>— {dailyQuote.attribution}</Text>
            </View>
          </Card>
        </View>

        <Button
          label="Log weight"
          icon="plus"
          variant="primary"
          onClick={() => onNavigate("entry")}
          style={{ alignSelf: "center" }}
        />
      </View>
    </View>
  );
}