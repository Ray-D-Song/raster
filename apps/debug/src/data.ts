import type { NewEntryDraft, UserProfile, UserSettings, WeightEntry } from "./types";

export const vitalityColors = {
  primary: "#006b5f",
  primaryContainer: "#2dd4bf",
  onPrimaryContainer: "#00574d",
  secondary: "#0058be",
  background: "#f8f9ff",
  surface: "#f8f9ff",
  surfaceContainer: "#e5eeff",
  surfaceContainerLow: "#eff4ff",
  surfaceContainerHigh: "#dce9ff",
  onSurface: "#0b1c30",
  onSurfaceVariant: "#3c4a46",
  outline: "#6b7a76",
  outlineVariant: "#bacac5",
  error: "#ba1a1a",
  errorContainer: "#ffdad6",
  onErrorContainer: "#93000a",
} as const;

export const userProfile: UserProfile = {
  name: "Alex Johnson",
  displayName: "Jordan",
  memberSince: "Jan 2024",
  heightCm: 180,
  avatarUrl:
    "https://lh3.googleusercontent.com/aida-public/AB6AXuDP1Kbq-jDU1ghpl-sGpfNOxfm2pTYzv71vQoDi8UOQj_LcbyUr2gb-uu3x8fdTDoLHsXOyQo2XDKKd_4iSHpDFFVGPGLnXahFDfl5wTdkk6e_sej8lkKH-qHd2mTU-OqDUzrJZD_dfLmfdDVsQbOwHbz0aXzhINolXaOqXi-zAiwMq5L7GkavHc-VkfFxOnY8N6fIYzv5BmITFcWFvZmCCENvfhbzW9mkUGYlIdc3cQJs1EJjVemfl",
};

export const defaultSettings: UserSettings = {
  theme: "light",
  unit: "kg",
  targetWeight: 71.6,
  targetDate: "2026-12-31",
  weeklyGoal: 0.25,
  dailyReminders: true,
  darkMode: false,
};

export const seedEntries: WeightEntry[] = [
  { id: "w-001", weight: 75.8, bodyFat: 18.2, date: "2026-06-22", time: "08:15", mood: "good", note: "Feeling lighter after morning walk." },
  { id: "w-002", weight: 76.1, bodyFat: 18.4, date: "2026-06-21", time: "07:50", mood: "great" },
  { id: "w-003", weight: 76.3, bodyFat: 18.5, date: "2026-06-20", time: "08:00", mood: "neutral" },
  { id: "w-004", weight: 76.5, bodyFat: 18.6, date: "2026-06-19", time: "07:45", mood: "good" },
  { id: "w-005", weight: 76.4, bodyFat: 18.7, date: "2026-06-18", time: "08:10", mood: "tired", note: "Late night, slightly bloated." },
  { id: "w-006", weight: 76.6, bodyFat: 18.8, date: "2026-06-17", time: "07:55", mood: "good" },
  { id: "w-007", weight: 76.6, bodyFat: 18.9, date: "2026-06-16", time: "08:05", mood: "neutral" },
  { id: "w-008", weight: 82.4, bodyFat: 21.2, date: "2023-10-22", time: "07:45", mood: "bloated" },
  { id: "w-009", weight: 82.0, bodyFat: 21.4, date: "2023-10-18", time: "08:12", mood: "good" },
  { id: "w-010", weight: 82.8, bodyFat: 21.8, date: "2023-10-15", time: "07:50", mood: "neutral" },
];

export const defaultDraft: NewEntryDraft = {
  weight: "72.5",
  bodyFat: "18.5",
  date: "2026-06-22",
  time: "08:30",
  mood: null,
  note: "",
};

export const dailyQuote = {
  text: "It's not about being perfect, it's about effort. And when you bring that effort every single day, that's where transformation happens.",
  attribution: "VitalTrack Daily Motivation",
};

export const activityStreakDays = 12;

export const startWeight = 80.0;