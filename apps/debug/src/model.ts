import type { Mood, NewEntryDraft, SortOrder, WeightEntry, WeightUnit } from "./types";

const moodLabels: Record<Mood, string> = {
  great: "Great",
  good: "Good",
  neutral: "Neutral",
  bloated: "Bloated",
  tired: "Tired",
};

export function moodLabel(mood: Mood): string {
  return moodLabels[mood];
}

export function formatWeight(value: number, unit: WeightUnit = "kg"): string {
  const formatted = value.toFixed(1);
  return unit === "kg" ? `${formatted} kg` : `${(value * 2.20462).toFixed(1)} lb`;
}

export function formatWeightDelta(delta: number, unit: WeightUnit = "kg"): string {
  const value = unit === "kg" ? delta : delta * 2.20462;
  const sign = value > 0 ? "+" : value < 0 ? "" : "";
  const suffix = unit === "kg" ? "kg" : "lb";
  return `${sign}${value.toFixed(1)} ${suffix}`;
}

export function formatDateLabel(date: string): string {
  const parsed = new Date(`${date}T12:00:00`);
  return parsed.toLocaleDateString("en-US", { month: "long", day: "numeric", year: "numeric" }).toUpperCase();
}

export function formatTimeLabel(time: string): string {
  const [hours, minutes] = time.split(":").map(Number);
  const period = hours >= 12 ? "PM" : "AM";
  const hour12 = hours % 12 || 12;
  return `${hour12}:${String(minutes).padStart(2, "0")} ${period}`;
}

export function computeBmi(weightKg: number, heightCm: number): number {
  const heightM = heightCm / 100;
  return weightKg / (heightM * heightM);
}

export function bmiCategory(bmi: number): string {
  if (bmi < 18.5) return "Underweight";
  if (bmi < 25) return "Healthy Range";
  if (bmi < 30) return "Overweight";
  return "High";
}

export function sortEntries(entries: WeightEntry[], order: SortOrder): WeightEntry[] {
  const sorted = [...entries].sort((a, b) => {
    const aKey = `${a.date}T${a.time}`;
    const bKey = `${b.date}T${b.time}`;
    return aKey < bKey ? -1 : aKey > bKey ? 1 : 0;
  });
  return order === "newest" ? sorted.reverse() : sorted;
}

export function previousEntry(entries: WeightEntry[], entry: WeightEntry): WeightEntry | null {
  const sorted = sortEntries(entries, "oldest");
  const index = sorted.findIndex((item) => item.id === entry.id);
  return index > 0 ? sorted[index - 1] : null;
}

export function entryDelta(entry: WeightEntry, previous: WeightEntry | null, unit: WeightUnit = "kg"): number | null {
  if (previous == null) return null;
  return entry.weight - previous.weight;
}

export function deltaTone(delta: number | null): "up" | "down" | "flat" {
  if (delta == null || Math.abs(delta) < 0.05) return "flat";
  return delta > 0 ? "up" : "down";
}

export function weekChange(entries: WeightEntry[]): { delta: number; from: number } | null {
  if (entries.length < 2) return null;
  const sorted = sortEntries(entries, "newest");
  const latest = sorted[0];
  const weekAgo = sorted.find((entry) => {
    const latestDate = new Date(`${latest.date}T12:00:00`);
    const entryDate = new Date(`${entry.date}T12:00:00`);
    const diffDays = (latestDate.getTime() - entryDate.getTime()) / (1000 * 60 * 60 * 24);
    return diffDays >= 6 && diffDays <= 8;
  });
  if (weekAgo == null) return null;
  return { delta: latest.weight - weekAgo.weight, from: weekAgo.weight };
}

export function goalProgress(current: number, target: number, start: number): number {
  const total = start - target;
  if (total <= 0) return 1;
  const done = start - current;
  return Math.min(1, Math.max(0, done / total));
}

export function distanceToGoal(current: number, target: number): number {
  return Math.max(0, current - target);
}

export function lastSevenDays(entries: WeightEntry[]): Array<{ label: string; weight: number }> {
  const sorted = sortEntries(entries, "newest").slice(0, 7).reverse();
  return sorted.map((entry) => {
    const day = new Date(`${entry.date}T12:00:00`).toLocaleDateString("en-US", { weekday: "short" });
    return { label: day.slice(0, 3).toUpperCase(), weight: entry.weight };
  });
}

export function entriesForMonth(entries: WeightEntry[], year: number, month: number): WeightEntry[] {
  const monthKey = `${year}-${String(month + 1).padStart(2, "0")}`;
  return entries.filter((entry) => entry.date.startsWith(monthKey));
}

export function makeEntry(draft: NewEntryDraft, nextId: number): WeightEntry | null {
  const weight = Number(draft.weight);
  const bodyFat = Number(draft.bodyFat);
  if (!Number.isFinite(weight) || weight <= 0) return null;
  if (!Number.isFinite(bodyFat) || bodyFat < 0) return null;
  return {
    id: `w-new-${nextId}`,
    weight,
    bodyFat,
    date: draft.date,
    time: draft.time,
    mood: draft.mood ?? undefined,
    note: draft.note.trim() || undefined,
  };
}

export function todayIso(): string {
  const now = new Date();
  return now.toISOString().split("T")[0];
}

export function nowTime(): string {
  const now = new Date();
  return `${String(now.getHours()).padStart(2, "0")}:${String(now.getMinutes()).padStart(2, "0")}`;
}