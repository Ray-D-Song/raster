import { categories } from "./data";
import type { Budget, Category, CurrencyCode, NewTransactionDraft, Transaction } from "./types";

const currencySymbols: Record<CurrencyCode, string> = {
  USD: "$",
  EUR: "€",
  CNY: "¥",
};

export function categoryById(categoryId: string): Category {
  return categories.find((category) => category.id === categoryId) ?? categories[0];
}

export function formatMoney(amount: number, currency: CurrencyCode): string {
  const symbol = currencySymbols[currency];
  const absolute = Math.abs(amount);
  const formatted =
    absolute >= 1000
      ? absolute.toLocaleString("en-US", { maximumFractionDigits: 0 })
      : absolute.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  return `${amount < 0 ? "-" : ""}${symbol}${formatted}`;
}

export function monthTransactions(transactions: Transaction[]): Transaction[] {
  return transactions.filter((transaction) => transaction.date.startsWith("2026-06"));
}

export function totalIncome(transactions: Transaction[]): number {
  return transactions
    .filter((transaction) => transaction.type === "income")
    .reduce((total, transaction) => total + transaction.amount, 0);
}

export function totalExpenses(transactions: Transaction[]): number {
  return transactions
    .filter((transaction) => transaction.type === "expense")
    .reduce((total, transaction) => total + transaction.amount, 0);
}

export function spentForCategory(transactions: Transaction[], categoryId: string): number {
  return transactions
    .filter((transaction) => transaction.type === "expense" && transaction.category === categoryId)
    .reduce((total, transaction) => total + transaction.amount, 0);
}

export function budgetProgress(budget: Budget, transactions: Transaction[]): number {
  return Math.min(1, spentForCategory(transactions, budget.category) / budget.limit);
}

export function spendingByCategory(transactions: Transaction[]) {
  return categories
    .filter((category) => category.id !== "income")
    .map((category) => ({
      category,
      spent: spentForCategory(transactions, category.id),
    }))
    .filter((row) => row.spent > 0)
    .sort((a, b) => b.spent - a.spent);
}

export function makeTransaction(draft: NewTransactionDraft, nextId: number): Transaction {
  const amount = Number(draft.amount);
  return {
    id: `tx-new-${nextId}`,
    title: draft.title.trim(),
    merchant: draft.merchant.trim() || draft.title.trim(),
    amount,
    type: draft.type,
    category: draft.type === "income" ? "income" : draft.category,
    date: draft.date,
    note: draft.note.trim() || undefined,
  };
}
