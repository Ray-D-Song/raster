import { Icon, Text, View } from "raster-js/components";
import type { IconName } from "raster-js/components";
import { vitalityColors } from "../data";
import { cardShadow, row } from "../styles";
import type { Mood } from "../types";

interface MoodPickerProps {
  value: Mood | null;
  onChange: (mood: Mood) => void;
}

const moodOptions: Array<{ id: Mood; icon: IconName }> = [
  { id: "great", icon: "star" },
  { id: "good", icon: "thumbs-up" },
  { id: "neutral", icon: "dash" },
  { id: "bloated", icon: "info" },
  { id: "tired", icon: "thumbs-down" },
];

export function MoodPicker({ value, onChange }: MoodPickerProps) {
  return (
    <View style={{ gap: 8 }}>
      <Text style={{ fontSize: 12, fontWeight: "600", color: vitalityColors.outline }}>HOW ARE YOU FEELING?</Text>
      <View
        style={{
          ...row,
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          backgroundColor: "#ffffff",
          borderRadius: 24,
          borderWidth: 1,
          borderColor: vitalityColors.surfaceContainer,
          boxShadow: cardShadow,
          padding: 12,
        }}
      >
        {moodOptions.map((option) => {
          const selected = value === option.id;
          return (
            <View
              key={option.id}
              onClick={() => onChange(option.id)}
              style={{
                padding: 12,
                borderRadius: 16,
                backgroundColor: selected ? vitalityColors.primaryContainer : "transparent",
                alignItems: "center",
              }}
            >
              <Icon
                name={option.icon}
                color={selected ? vitalityColors.primary : vitalityColors.outline}
                size="large"
              />
            </View>
          );
        })}
      </View>
    </View>
  );
}