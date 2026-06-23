import { Button, ButtonGroup, Icon, Text, View } from "raster-js/components";
import { Card } from "../components/Card";
import { WeightEntryCard } from "../components/WeightEntryCard";
import { vitalityColors } from "../data";
import { appIcons } from "../icons";
import { entriesForMonth, previousEntry, sortEntries } from "../model";
import { type AppTheme, labelCaps, pagePadding, spaceBetween } from "../styles";
import type { SortOrder, WeightEntry, WeightUnit } from "../types";

interface HistoryProps {
  entries: WeightEntry[];
  unit: WeightUnit;
  theme: AppTheme;
  sortOrder: SortOrder;
  calendarMonth: { year: number; month: number };
  selectedDay: number | null;
  onSortChange: (order: SortOrder) => void;
  onMonthChange: (delta: number) => void;
  onSelectDay: (day: number | null) => void;
}

const weekdayLabels = ["S", "M", "T", "W", "T", "F", "S"];

function monthLabel(year: number, month: number): string {
  return new Date(year, month, 1).toLocaleDateString("en-US", { month: "long", year: "numeric" });
}

function calendarCells(year: number, month: number): Array<number | null> {
  const firstDay = new Date(year, month, 1).getDay();
  const daysInMonth = new Date(year, month + 1, 0).getDate();
  const cells: Array<number | null> = [];
  for (let index = 0; index < firstDay; index += 1) cells.push(null);
  for (let day = 1; day <= daysInMonth; day += 1) cells.push(day);
  return cells;
}

export function History({
  entries,
  unit,
  theme,
  sortOrder,
  calendarMonth,
  selectedDay,
  onSortChange,
  onMonthChange,
  onSelectDay,
}: HistoryProps) {
  const monthEntries = entriesForMonth(entries, calendarMonth.year, calendarMonth.month);
  const entryDays = new Set(monthEntries.map((entry) => Number(entry.date.split("-")[2])));
  const sorted = sortEntries(entries, sortOrder);
  const cells = calendarCells(calendarMonth.year, calendarMonth.month);

  const filtered =
    selectedDay == null
      ? sorted
      : sorted.filter((entry) => {
          const [year, month, day] = entry.date.split("-").map(Number);
          return year === calendarMonth.year && month === calendarMonth.month + 1 && day === selectedDay;
        });

  return (
    <View style={{ backgroundColor: theme.background }}>
      <View style={[pagePadding, { gap: 32 }]}>
        <View style={{ gap: 8 }}>
          <Text style={{ fontSize: 24, fontWeight: "600", color: vitalityColors.onSurface }}>Weight Journey</Text>
          <Text style={{ fontSize: 14, color: vitalityColors.onSurfaceVariant }}>Your progress over the last 30 days</Text>
        </View>

        <Card theme={theme} style={{ gap: 16 }}>
          <View style={spaceBetween}>
            <Text style={{ fontSize: 20, fontWeight: "600", color: vitalityColors.onSurface }}>
              {monthLabel(calendarMonth.year, calendarMonth.month)}
            </Text>
            <View style={{ flexDirection: "row", gap: 8 }}>
              <View onClick={() => onMonthChange(-1)} style={{ padding: 8, borderRadius: 999 }}>
                <Icon src={appIcons.chevronLeft} color={vitalityColors.primary} size={16} />
              </View>
              <View onClick={() => onMonthChange(1)} style={{ padding: 8, borderRadius: 999 }}>
                <Icon src={appIcons.chevronRight} color={vitalityColors.primary} size={16} />
              </View>
            </View>
          </View>

          <View style={{ flexDirection: "row", flexWrap: "wrap" }}>
            {weekdayLabels.map((label, index) => (
              <View key={`weekday-${index}`} style={{ width: "14.28%", alignItems: "center", margin: { bottom: 8 } }}>
                <Text style={{ ...labelCaps, color: vitalityColors.onSurfaceVariant }}>{label}</Text>
              </View>
            ))}
          </View>

          <View style={{ flexDirection: "row", flexWrap: "wrap" }}>
            {cells.map((day, index) => {
              if (day == null) {
                return <View key={`empty-${index}`} style={{ width: "14.28%", height: 36 }} />;
              }
              const hasEntry = entryDays.has(day);
              const selected = selectedDay === day;
              return (
                <View
                  key={`day-${day}`}
                  onClick={() => onSelectDay(selected ? null : day)}
                  style={{
                    width: "14.28%",
                    height: 36,
                    alignItems: "center",
                    justifyContent: "center",
                    borderRadius: 999,
                    backgroundColor: selected ? vitalityColors.primary : "transparent",
                  }}
                >
                  <Text
                    style={{
                      fontSize: 14,
                      color: selected ? "#ffffff" : hasEntry ? vitalityColors.onSurface : vitalityColors.outline,
                      fontWeight: hasEntry ? "600" : "400",
                    }}
                  >
                    {day}
                  </Text>
                  {hasEntry && !selected ? (
                    <View
                      style={{
                        width: 4,
                        height: 4,
                        borderRadius: 2,
                        backgroundColor: vitalityColors.primary,
                        margin: { top: 2 },
                      }}
                    />
                  ) : null}
                </View>
              );
            })}
          </View>
        </Card>

        <View style={spaceBetween}>
          <Text style={{ fontSize: 20, fontWeight: "600", color: vitalityColors.onSurface }}>Detailed Logs</Text>
          <ButtonGroup
            value={sortOrder}
            variant="primary"
            outline
            size="small"
            onChange={(value) => onSortChange(String(value ?? "newest") as SortOrder)}
          >
            <Button label="Newest" value="newest" />
            <Button label="Oldest" value="oldest" />
          </ButtonGroup>
        </View>

        <View style={{ gap: 16 }}>
          {filtered.map((entry) => {
            const previous = previousEntry(entries, entry);
            return (
              <WeightEntryCard
                key={entry.id}
                entry={entry}
                previous={previous}
                unit={unit}
              />
            );
          })}
          {filtered.length === 0 ? (
            <Text style={{ color: vitalityColors.onSurfaceVariant, fontSize: 14 }}>
              No entries for this selection.
            </Text>
          ) : null}
        </View>
      </View>
    </View>
  );
}