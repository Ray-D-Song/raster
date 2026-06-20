export type TransactionType = "expense" | "income";

export type ThemePreference = "light" | "dark";

export type ThemePresetChoice = "macos-classic" | "ayu" | "catppuccin" | "everforest" | "gruvbox" | "solarized";

export type CurrencyCode = "USD" | "EUR" | "CNY";

export interface Category {
  id: string;
  name: string;
  color: string;
  icon: string;
}

export interface Transaction {
  id: string;
  title: string;
  merchant: string;
  amount: number;
  type: TransactionType;
  category: string;
  date: string;
  note?: string;
}

export interface Budget {
  category: string;
  limit: number;
  color: string;
}

export interface UserSettings {
  currency: CurrencyCode;
  theme: ThemePreference;
  themePreset: ThemePresetChoice;
  budgetAlerts: boolean;
  monthlyReports: boolean;
  budgetCycle: "monthly";
}

export interface NewTransactionDraft {
  title: string;
  merchant: string;
  amount: string;
  type: TransactionType;
  category: string;
  date: string;
  note: string;
}

export type AppTab = "overview" | "transactions" | "budget" | "settings";
