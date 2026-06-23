import { useState } from "react";
import { AppShell, AppShellTab, AppShellTabBar, ConfigProvider, View, useTheme } from "raster-js/components";
import { defaultDraft, defaultSettings, seedEntries } from "./data";
import { makeEntry, nowTime, sortEntries, todayIso } from "./model";
import { Dashboard } from "./pages/Dashboard";
import { Entry } from "./pages/Entry";
import { History } from "./pages/History";
import { Settings } from "./pages/Settings";
import { appIcons } from "./icons";
import { tabBarShadow, vitalityTheme } from "./styles";
import type { AppTab, NewEntryDraft, SortOrder, UserSettings, WeightEntry } from "./types";

function renderPage(
  tab: AppTab,
  appTheme: NonNullable<ReturnType<typeof useTheme>>["colors"],
  props: {
    draft: NewEntryDraft;
    entries: WeightEntry[];
    settings: UserSettings;
    error: string;
    sortOrder: SortOrder;
    calendarMonth: { year: number; month: number };
    selectedDay: number | null;
    setDraft: (draft: NewEntryDraft) => void;
    setError: (error: string) => void;
    setSortOrder: (order: SortOrder) => void;
    setCalendarMonth: (value: { year: number; month: number } | ((current: { year: number; month: number }) => { year: number; month: number })) => void;
    setSelectedDay: (day: number | null) => void;
    setSettings: (settings: UserSettings) => void;
    setTab: (tab: AppTab) => void;
    submitDraft: () => void;
  }
) {
  if (tab === "entry") {
    return (
      <Entry
        draft={props.draft}
        theme={appTheme}
        error={props.error}
        onChange={props.setDraft}
        onSubmit={props.submitDraft}
        onClearError={() => props.setError("")}
      />
    );
  }
  if (tab === "history") {
    return (
      <History
        entries={props.entries}
        unit={props.settings.unit}
        theme={appTheme}
        sortOrder={props.sortOrder}
        calendarMonth={props.calendarMonth}
        selectedDay={props.selectedDay}
        onSortChange={props.setSortOrder}
        onMonthChange={(delta) =>
          props.setCalendarMonth((current) => {
            const next = new Date(current.year, current.month + delta, 1);
            return { year: next.getFullYear(), month: next.getMonth() };
          })
        }
        onSelectDay={props.setSelectedDay}
      />
    );
  }
  if (tab === "settings") {
    return (
      <Settings
        settings={props.settings}
        theme={appTheme}
        entryCount={props.entries.length}
        onChange={props.setSettings}
      />
    );
  }
  return (
    <Dashboard
      entries={props.entries}
      settings={props.settings}
      theme={appTheme}
      onNavigate={props.setTab}
    />
  );
}

export function App() {
  const [tab, setTab] = useState<AppTab>("dashboard");
  const [entries, setEntries] = useState<WeightEntry[]>(() => sortEntries(seedEntries, "newest"));
  const [settings, setSettings] = useState<UserSettings>(defaultSettings);
  const [draft, setDraft] = useState<NewEntryDraft>(defaultDraft);
  const [error, setError] = useState("");
  const [sortOrder, setSortOrder] = useState<SortOrder>("newest");
  const [calendarMonth, setCalendarMonth] = useState(() => {
    const now = new Date();
    return { year: now.getFullYear(), month: now.getMonth() };
  });
  const [selectedDay, setSelectedDay] = useState<number | null>(null);
  const [nextId, setNextId] = useState(1);

  const nativeTheme = useTheme();
  const appTheme = nativeTheme?.colors ?? null;

  function submitDraft() {
    const entry = makeEntry(draft, nextId);
    if (entry == null) {
      setError("Enter a valid weight and body fat percentage.");
      return;
    }
    setEntries((current) => sortEntries([entry, ...current], "newest"));
    setNextId((value) => value + 1);
    setDraft({
      weight: "",
      bodyFat: draft.bodyFat,
      date: todayIso(),
      time: nowTime(),
      mood: null,
      note: "",
    });
    setError("");
    setTab("history");
  }

  return (
    <ConfigProvider theme={{ ...vitalityTheme, mode: settings.theme }}>
      <AppShell
        tabBar={
          <AppShellTabBar
            value={tab}
            onValueChange={(value) => setTab(value as AppTab)}
            style={{ boxShadow: tabBarShadow }}
          >
            <AppShellTab value="dashboard" label="Dashboard" icon={appIcons.dashboard} />
            <AppShellTab value="entry" label="Entry" icon={appIcons.add} />
            <AppShellTab value="history" label="History" icon={appIcons.calendar} />
            <AppShellTab value="settings" label="Settings" icon={appIcons.settings} />
          </AppShellTabBar>
        }
      >
        {appTheme
          ? renderPage(tab, appTheme, {
              draft,
              entries,
              settings,
              error,
              sortOrder,
              calendarMonth,
              selectedDay,
              setDraft,
              setError,
              setSortOrder,
              setCalendarMonth,
              setSelectedDay,
              setSettings,
              setTab,
              submitDraft,
            })
          : <View />}
      </AppShell>
    </ConfigProvider>
  );
}