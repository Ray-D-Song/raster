import { Button, Icon, Select, Switch, Text, View } from "raster-js/components";
import { Card } from "../components/Card";
import { SectionHeader } from "../components/SectionHeader";
import { colors, pagePadding, row, secondaryText, spaceBetween, textColor } from "../styles";
import type { CurrencyCode, ThemePreference, UserSettings } from "../types";

interface SettingsProps {
  settings: UserSettings;
  theme: ThemePreference;
  onChange: (settings: UserSettings) => void;
  transactionCount: number;
}

const currencyOptions = [
  { id: "USD", label: "USD - US Dollar", value: "USD" },
  { id: "EUR", label: "EUR - Euro", value: "EUR" },
  { id: "CNY", label: "CNY - Yuan", value: "CNY" },
];

const themeOptions = [
  { id: "light", label: "Light", value: "light" },
  { id: "dark", label: "Dark", value: "dark" },
];

export function Settings({ settings, theme, onChange, transactionCount }: SettingsProps) {
  return (
    <View style={[pagePadding, { gap: 12 }]}>
      <View style={{ gap: 3 }}>
        <Text style={{ color: secondaryText(theme), fontSize: 12 }}>Personal workspace</Text>
        <Text style={{ color: textColor(theme), fontSize: 24, fontWeight: "800" }}>Settings</Text>
      </View>

      <Card theme={theme}>
        <View style={{ flexDirection: "row", gap: 12, alignItems: "center" }}>
          <View
            style={{
              width: 48,
              height: 48,
              borderRadius: 8,
              backgroundColor: "#dbeafe",
              alignItems: "center",
              justifyContent: "center",
            }}
          >
            <Icon name="circle-user" color={colors.blue} />
          </View>
          <View style={{ flex: 1, gap: 3 }}>
            <Text style={{ color: textColor(theme), fontSize: 17, fontWeight: "800" }}>Ray Song</Text>
            <Text style={{ color: secondaryText(theme), fontSize: 12 }}>Individual plan · Offline debug profile</Text>
          </View>
        </View>
      </Card>

      <View style={{ gap: 8 }}>
        <SectionHeader title="Preferences" theme={theme} />
        <Card theme={theme} style={{ gap: 12 }}>
          <View style={{ gap: 6 }}>
            <Text style={{ color: textColor(theme), fontSize: 13, fontWeight: "700" }}>Currency</Text>
            <Select
              value={settings.currency}
              options={currencyOptions}
              onChange={(payload) =>
                onChange({ ...settings, currency: String(payload.value ?? "USD") as CurrencyCode })
              }
            />
          </View>
          <View style={{ gap: 6 }}>
            <Text style={{ color: textColor(theme), fontSize: 13, fontWeight: "700" }}>Theme</Text>
            <Select
              value={settings.theme}
              options={themeOptions}
              onChange={(payload) =>
                onChange({ ...settings, theme: String(payload.value ?? "light") as ThemePreference })
              }
            />
          </View>
        </Card>
      </View>

      <View style={{ gap: 8 }}>
        <SectionHeader title="Notifications" theme={theme} />
        <Card theme={theme} style={{ gap: 14 }}>
          <View style={spaceBetween}>
            <View style={{ gap: 3 }}>
              <Text style={{ color: textColor(theme), fontSize: 14, fontWeight: "700" }}>Budget alerts</Text>
              <Text style={{ color: secondaryText(theme), fontSize: 11 }}>Warn when a category crosses its limit.</Text>
            </View>
            <Switch
              checked={settings.budgetAlerts}
              onChange={(value) => onChange({ ...settings, budgetAlerts: value === true })}
            />
          </View>
          <View style={spaceBetween}>
            <View style={{ gap: 3 }}>
              <Text style={{ color: textColor(theme), fontSize: 14, fontWeight: "700" }}>Monthly report</Text>
              <Text style={{ color: secondaryText(theme), fontSize: 11 }}>Summarize spending at month end.</Text>
            </View>
            <Switch
              checked={settings.monthlyReports}
              onChange={(value) => onChange({ ...settings, monthlyReports: value === true })}
            />
          </View>
        </Card>
      </View>

      <Card theme={theme} style={{ gap: 10 }}>
        <View style={row}>
          <Icon name="info" color={colors.blue} size="small" />
          <Text style={{ color: textColor(theme), fontSize: 14, fontWeight: "700" }}>Debug data</Text>
        </View>
        <Text style={{ color: secondaryText(theme), fontSize: 12 }}>
          This build keeps {transactionCount} transactions in memory. Restarting the app restores the seed ledger.
        </Text>
        <Button label="Export preview" variant="secondary" icon="external-link" size="small" />
      </Card>
    </View>
  );
}
