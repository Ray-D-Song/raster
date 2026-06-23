import type { ReactNode } from "react";
import { Avatar, Button, ButtonGroup, Icon, Input, Switch, Text, View } from "raster-js/components";
import type { IconName } from "raster-js/components";
import { Card } from "../components/Card";
import { userProfile, vitalityColors } from "../data";
import { type AppTheme, labelCaps, pagePadding, spaceBetween } from "../styles";
import type { UserSettings, WeeklyGoal, WeightUnit } from "../types";

interface SettingsProps {
  settings: UserSettings;
  theme: AppTheme;
  entryCount: number;
  onChange: (settings: UserSettings) => void;
}

const weeklyGoalOptions: WeeklyGoal[] = [0.25, 0.5, 1.0];

const preferenceDivider = "rgba(186, 202, 197, 0.2)";

interface PreferenceRowProps {
  icon: IconName;
  label: string;
  control: ReactNode;
  bordered?: boolean;
}

function PreferenceRow({ icon, label, control, bordered = false }: PreferenceRowProps) {
  return (
    <View
      style={{
        ...spaceBetween,
        padding: { top: 16, bottom: 16 },
        ...(bordered ? { borderTopWidth: 1, borderColor: preferenceDivider } : {}),
      }}
    >
      <View style={{ flexDirection: "row", alignItems: "center", gap: 16, flex: 1, minWidth: 0 }}>
        <View
          style={{
            width: 24,
            height: 24,
            alignItems: "center",
            justifyContent: "center",
            flexShrink: 0,
          }}
        >
          <Icon name={icon} color={vitalityColors.outline} size="medium" />
        </View>
        <Text style={{ fontSize: 16 }}>{label}</Text>
      </View>
      <View style={{ flexShrink: 0, margin: { left: 16 } }}>{control}</View>
    </View>
  );
}

export function Settings({ settings, theme, entryCount, onChange }: SettingsProps) {
  return (
    <View style={{ backgroundColor: theme.background }}>
      <View style={[pagePadding, { gap: 32 }]}>
        <View style={{ flexDirection: "row", flexWrap: "wrap", gap: 16 }}>
          <Card theme={theme} style={{ flex: 1, minWidth: 160, alignItems: "center", gap: 12 }}>
            <Avatar src={userProfile.avatarUrl} size="large" />
            <Text style={{ fontSize: 20, fontWeight: "600", color: vitalityColors.onSurface }}>{userProfile.name}</Text>
            <Text style={{ fontSize: 14, color: vitalityColors.outline }}>Member since {userProfile.memberSince}</Text>
          </Card>

          <Card theme={theme} tinted style={{ flex: 1, minWidth: 160, justifyContent: "space-between" }}>
            <View style={{ gap: 8 }}>
              <Text style={{ ...labelCaps, color: vitalityColors.primary }}>CURRENT HEIGHT</Text>
              <View style={{ flexDirection: "row", alignItems: "baseline", gap: 4 }}>
                <Text style={{ fontSize: 36, fontWeight: "700", color: vitalityColors.primary }}>
                  {userProfile.heightCm}
                </Text>
                <Text style={{ fontSize: 20, color: "rgba(0, 107, 95, 0.7)" }}>cm</Text>
              </View>
            </View>
            <View style={{ flexDirection: "row", alignItems: "center", gap: 4, margin: { top: 12 } }}>
              <Text style={{ fontSize: 16, fontWeight: "600", color: vitalityColors.primary }}>Edit Profile</Text>
              <Icon name="settings" color={vitalityColors.primary} size="small" />
            </View>
          </Card>
        </View>

        <View style={{ gap: 16 }}>
          <View style={{ flexDirection: "row", alignItems: "center", gap: 8 }}>
            <Icon name="chart-pie" color={vitalityColors.primary} size="medium" />
            <Text style={{ fontSize: 20, fontWeight: "600", color: vitalityColors.onSurface }}>Goal Settings</Text>
          </View>
          <Card theme={theme} style={{ gap: 24 }}>
            <View
              style={{
                ...spaceBetween,
                padding: 16,
                borderRadius: 12,
                backgroundColor: vitalityColors.surfaceContainerLow,
              }}
            >
              <View style={{ gap: 4 }}>
                <Text style={{ fontSize: 20, fontWeight: "600", color: vitalityColors.onSurface }}>Target Weight</Text>
                <Text style={{ fontSize: 14, color: vitalityColors.outline }}>Your ideal body mass</Text>
              </View>
              <View style={{ flexDirection: "row", alignItems: "center", gap: 8 }}>
                <Input
                  value={String(settings.targetWeight)}
                  style={{ width: 64, fontWeight: "600", color: vitalityColors.primary }}
                  onChange={(event) =>
                    onChange({ ...settings, targetWeight: Number(event.value ?? settings.targetWeight) })
                  }
                />
                <Text style={{ fontSize: 16, color: vitalityColors.outline }}>kg</Text>
              </View>
            </View>

            <View
              style={{
                ...spaceBetween,
                padding: 16,
                borderRadius: 12,
                backgroundColor: vitalityColors.surfaceContainerLow,
              }}
            >
              <View style={{ gap: 4 }}>
                <Text style={{ fontSize: 20, fontWeight: "600", color: vitalityColors.onSurface }}>Target Date</Text>
                <Text style={{ fontSize: 14, color: vitalityColors.outline }}>When to reach your goal</Text>
              </View>
              <Input
                value={settings.targetDate}
                onChange={(event) => onChange({ ...settings, targetDate: event.value ?? settings.targetDate })}
              />
            </View>

            <View style={{ gap: 12 }}>
              <Text style={{ fontSize: 20, fontWeight: "600", color: vitalityColors.onSurface }}>Weekly Goal</Text>
              <View style={{ flexDirection: "row", gap: 8 }}>
                {weeklyGoalOptions.map((goal) => {
                  const selected = settings.weeklyGoal === goal;
                  return (
                    <View
                      key={goal}
                      onClick={() => onChange({ ...settings, weeklyGoal: goal })}
                      style={{
                        flex: 1,
                        padding: 12,
                        borderRadius: 12,
                        borderWidth: 2,
                        borderColor: selected ? vitalityColors.primaryContainer : vitalityColors.outlineVariant,
                        backgroundColor: selected ? "rgba(45, 212, 191, 0.1)" : "transparent",
                        alignItems: "center",
                      }}
                    >
                      <Text
                        style={{
                          ...labelCaps,
                          color: selected ? vitalityColors.primary : vitalityColors.outline,
                        }}
                      >
                        {goal} kg/wk
                      </Text>
                    </View>
                  );
                })}
              </View>
            </View>
          </Card>
        </View>

        <View style={{ gap: 16 }}>
          <View style={{ flexDirection: "row", alignItems: "center", gap: 8 }}>
            <Icon name="settings" color={vitalityColors.primary} size="medium" />
            <Text style={{ fontSize: 20, fontWeight: "600", color: vitalityColors.onSurface }}>Preferences</Text>
          </View>
          <Card theme={theme} style={{ gap: 0, padding: {
            top: 8, right: 20, bottom: 8, left: 20
          } }}>
            <PreferenceRow
              icon="bell"
              label="Daily Reminders"
              control={
                <Switch
                  checked={settings.dailyReminders}
                  onChange={(value) => onChange({ ...settings, dailyReminders: value === true })}
                />
              }
            />
            <PreferenceRow
              icon="info"
              label="Units of Measure"
              bordered
              control={
                <ButtonGroup
                  value={settings.unit}
                  variant="primary"
                  outline
                  size="small"
                  onChange={(value) => onChange({ ...settings, unit: String(value ?? "kg") as WeightUnit })}
                >
                  <Button label="KG" value="kg" />
                  <Button label="LB" value="lb" />
                </ButtonGroup>
              }
            />
            <PreferenceRow
              icon="moon"
              label="Dark Mode"
              bordered
              control={
                <Switch
                  checked={settings.darkMode}
                  onChange={(value) =>
                    onChange({ ...settings, darkMode: value === true, theme: value === true ? "dark" : "light" })
                  }
                />
              }
            />
          </Card>
        </View>

        <View style={{ gap: 12 }}>
          <Button label="Export CSV Data" icon="external-link" variant="primary" outline />
          <Button label="Delete All Data" icon="delete" variant="danger" />
          <Text style={{ fontSize: 12, color: vitalityColors.onSurfaceVariant }}>
            Debug build keeps {entryCount} weight entries in memory.
          </Text>
        </View>
      </View>
    </View>
  );
}