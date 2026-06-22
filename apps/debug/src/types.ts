export type ThemePreference = "light" | "dark";

export type WeightUnit = "kg" | "lb";

export type Mood = "great" | "good" | "neutral" | "bloated" | "tired";

export type SortOrder = "newest" | "oldest";

export type WeeklyGoal = 0.25 | 0.5 | 1.0;

export interface WeightEntry {
  id: string;
  weight: number;
  bodyFat: number;
  date: string;
  time: string;
  mood?: Mood;
  note?: string;
}

export interface UserProfile {
  name: string;
  displayName: string;
  memberSince: string;
  heightCm: number;
  avatarUrl: string;
}

export interface UserSettings {
  theme: ThemePreference;
  unit: WeightUnit;
  targetWeight: number;
  targetDate: string;
  weeklyGoal: WeeklyGoal;
  dailyReminders: boolean;
  darkMode: boolean;
}

export interface NewEntryDraft {
  weight: string;
  bodyFat: string;
  date: string;
  time: string;
  mood: Mood | null;
  note: string;
}

export type AppTab = "dashboard" | "entry" | "history" | "settings";