import { Icon, Text, View } from "raster-js/components";
import type { IconifyIcon } from "raster-js/components";
import { vitalityColors } from "../data";
import { appIcons } from "../icons";
import { cardShadow, row } from "../styles";
import type { Mood } from "../types";

interface MoodPickerProps {
  value: Mood | null;
  onChange: (mood: Mood) => void;
}

const moodOptions: Array<{ id: Mood; icon: IconifyIcon }> = [
  { id: "great", icon: appIcons.star },
  { id: "good", icon: appIcons.thumbUp },
  { id: "neutral", icon: appIcons.horizontalRule },
  { id: "bloated", icon: appIcons.info },
  { id: "tired", icon: appIcons.thumbDown },
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
                src={option.icon}
                color={selected ? vitalityColors.primary : vitalityColors.outline}
                size={24}
              />
            </View>
          );
        })}
      </View>
    </View>
  );
}