import type { Budget, Category, NewTransactionDraft, Transaction, UserSettings } from "./types";

export const categories: Category[] = [
  { id: "groceries", name: "Groceries", color: "#2563eb", icon: "inbox" },
  { id: "dining", name: "Dining", color: "#dc2626", icon: "heart" },
  { id: "transport", name: "Transport", color: "#0891b2", icon: "map" },
  { id: "home", name: "Home", color: "#7c3aed", icon: "building" },
  { id: "wellness", name: "Wellness", color: "#16a34a", icon: "circle-check" },
  { id: "income", name: "Income", color: "#15803d", icon: "check" },
];

export const seedTransactions: Transaction[] = [
  {
    id: "tx-001",
    title: "Paycheck",
    merchant: "Acme Studio",
    amount: 4820,
    type: "income",
    category: "income",
    date: "2026-06-15",
    note: "June payroll",
  },
  {
    id: "tx-002",
    title: "Weekly groceries",
    merchant: "Market Lane",
    amount: 94.32,
    type: "expense",
    category: "groceries",
    date: "2026-06-16",
  },
  {
    id: "tx-003",
    title: "Dinner with team",
    merchant: "Juniper Table",
    amount: 58.4,
    type: "expense",
    category: "dining",
    date: "2026-06-14",
  },
  {
    id: "tx-004",
    title: "Metro card",
    merchant: "City Transit",
    amount: 36,
    type: "expense",
    category: "transport",
    date: "2026-06-12",
  },
  {
    id: "tx-005",
    title: "Rent",
    merchant: "Northline Homes",
    amount: 1480,
    type: "expense",
    category: "home",
    date: "2026-06-03",
  },
  {
    id: "tx-006",
    title: "Yoga membership",
    merchant: "Breathe Club",
    amount: 72,
    type: "expense",
    category: "wellness",
    date: "2026-06-02",
  },
  {
    id: "tx-007",
    title: "Cafe",
    merchant: "Little Owl",
    amount: 12.8,
    type: "expense",
    category: "dining",
    date: "2026-06-01",
  },
];

export const seedBudgets: Budget[] = [
  { category: "groceries", limit: 520, color: "#2563eb" },
  { category: "dining", limit: 360, color: "#dc2626" },
  { category: "transport", limit: 180, color: "#0891b2" },
  { category: "home", limit: 1600, color: "#7c3aed" },
  { category: "wellness", limit: 220, color: "#16a34a" },
];

export const defaultSettings: UserSettings = {
  currency: "USD",
  theme: "light",
  budgetAlerts: true,
  monthlyReports: false,
  budgetCycle: "monthly",
};

export const defaultDraft: NewTransactionDraft = {
  title: "",
  merchant: "",
  amount: "",
  type: "expense",
  category: "groceries",
  date: "2026-06-18",
  note: "",
};
