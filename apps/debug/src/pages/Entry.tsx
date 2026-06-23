import { Alert, Button, DatePicker, Icon, Input, Slider, Text, Textarea, View } from "raster-js/components";
import { Card } from "../components/Card";
import { MoodPicker } from "../components/MoodPicker";
import { vitalityColors } from "../data";
import { appIcons } from "../icons";
import { type AppTheme, elevatedShadow, labelCaps, pagePadding } from "../styles";
import type { NewEntryDraft } from "../types";

interface EntryProps {
  draft: NewEntryDraft;
  theme: AppTheme;
  error: string;
  onChange: (draft: NewEntryDraft) => void;
  onSubmit: () => void;
  onClearError: () => void;
}

export function Entry({ draft, theme, error, onChange, onSubmit, onClearError }: EntryProps) {
  const bodyFat = Number(draft.bodyFat);

  return (
    <View style={{ backgroundColor: theme.background }}>
      <View style={[pagePadding, { gap: 16 }]}>
        <View style={{ gap: 8 }}>
          <Text style={{ fontSize: 24, fontWeight: "600", color: vitalityColors.onSurface }}>New Measurement</Text>
          <Text style={{ fontSize: 14, color: vitalityColors.onSurfaceVariant }}>
            Consistency is the key to progress. Let's record your stats for today.
          </Text>
        </View>

        <Card theme={theme} style={{ gap: 16 }}>
          <Text style={{ ...labelCaps, color: vitalityColors.outline }}>WEIGHT (KG)</Text>
          <View style={{ flexDirection: "row", alignItems: "center", justifyContent: "center", gap: 8 }}>
            <Input
              value={draft.weight}
              placeholder="0.0"
              style={{ fontWeight: "700", color: vitalityColors.primary, width: 160 }}
              onChange={(event) => onChange({ ...draft, weight: event.value ?? "" })}
            />
            <Text style={{ fontSize: 20, fontWeight: "600", color: vitalityColors.onSurfaceVariant }}>kg</Text>
          </View>
        </Card>

        <Card theme={theme} style={{ gap: 12 }}>
          <View style={{ flexDirection: "row", justifyContent: "space-between", alignItems: "center" }}>
            <Text style={{ ...labelCaps, color: vitalityColors.outline }}>BODY FAT (%)</Text>
            <Text style={{ fontSize: 20, fontWeight: "600", color: vitalityColors.primary }}>
              {Number.isFinite(bodyFat) ? `${bodyFat.toFixed(1)}%` : "—"}
            </Text>
          </View>
          <Slider
            min={5}
            max={50}
            step={0.5}
            value={Number.isFinite(bodyFat) ? bodyFat : 18.5}
            onChange={(event) => onChange({ ...draft, bodyFat: String(event.value) })}
          />
          <View style={{ flexDirection: "row", justifyContent: "space-between" }}>
            <Text style={{ fontSize: 12, color: vitalityColors.onSurfaceVariant }}>5%</Text>
            <Text style={{ fontSize: 12, color: vitalityColors.onSurfaceVariant }}>25%</Text>
            <Text style={{ fontSize: 12, color: vitalityColors.onSurfaceVariant }}>50%</Text>
          </View>
        </Card>

        <View style={{ flexDirection: "row", gap: 16 }}>
          <Card theme={theme} style={{ flex: 1, gap: 8, padding: 16 }}>
            <Text style={{ ...labelCaps, color: vitalityColors.outline }}>DATE</Text>
            <DatePicker
              value={draft.date}
              placeholder="Select date"
              appearance={false}
              onChange={(event) => {
                const value = event.value;
                if (typeof value === "string" && value.length > 0) {
                  onChange({ ...draft, date: value });
                }
              }}
            />
          </Card>
          <Card theme={theme} style={{ flex: 1, gap: 8, padding: 16 }}>
            <Text style={{ ...labelCaps, color: vitalityColors.outline }}>TIME</Text>
            <View style={{ flexDirection: "row", alignItems: "center", gap: 8 }}>
              <Icon src={appIcons.info} color={vitalityColors.primary} size={14} />
              <Input
                value={draft.time}
                onChange={(event) => onChange({ ...draft, time: event.value ?? "" })}
              />
            </View>
          </Card>
        </View>

        <MoodPicker
          value={draft.mood}
          onChange={(mood) => onChange({ ...draft, mood })}
        />

        <Card theme={theme} style={{ padding: 16 }}>
          <Textarea
            value={draft.note}
            placeholder="Add a private note about your day..."
            rows={3}
            onChange={(event) => onChange({ ...draft, note: event.value ?? "" })}
          />
        </Card>

        <Button
          label="Save Record"
          icon={appIcons.check}
          variant="primary"
          onClick={onSubmit}
          style={{ height: 56, borderRadius: 999, boxShadow: elevatedShadow }}
        />
      </View>

      <Alert
        open={error.length > 0}
        title="Check entry"
        description={error}
        icon={appIcons.warning}
        okText="Got it"
        onOk={onClearError}
        onOpenChange={(event) => {
          if (!event.open) onClearError();
        }}
      />
    </View>
  );
}