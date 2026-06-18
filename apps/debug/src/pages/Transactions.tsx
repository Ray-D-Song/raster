import { Button, Input, Select, Text, View } from "raster-js/components";
import { categories } from "../data";
import { Card } from "../components/Card";
import { SectionHeader } from "../components/SectionHeader";
import { TransactionRow } from "../components/TransactionRow";
import { categoryById, totalExpenses, totalIncome } from "../model";
import { colors, pagePadding, secondaryText, spaceBetween, textColor } from "../styles";
import type { CurrencyCode, ThemePreference, Transaction } from "../types";

interface TransactionsProps {
  transactions: Transaction[];
  currency: CurrencyCode;
  theme: ThemePreference;
  search: string;
  categoryFilter: string;
  onSearchChange: (value: string) => void;
  onCategoryChange: (value: string) => void;
  onAdd: () => void;
  onOpenTransaction: (transaction: Transaction) => void;
}

const categoryOptions = [
  { id: "all", label: "All categories", value: "all" },
  ...categories.map((category) => ({ id: category.id, label: category.name, value: category.id })),
];

export function Transactions({
  transactions,
  currency,
  theme,
  search,
  categoryFilter,
  onSearchChange,
  onCategoryChange,
  onAdd,
  onOpenTransaction,
}: TransactionsProps) {
  const normalizedSearch = search.trim().toLowerCase();
  const filtered = transactions.filter((transaction) => {
    const category = categoryById(transaction.category);
    const matchesCategory = categoryFilter === "all" || transaction.category === categoryFilter;
    const haystack = `${transaction.title} ${transaction.merchant} ${category.name}`.toLowerCase();
    return matchesCategory && (normalizedSearch.length === 0 || haystack.includes(normalizedSearch));
  });
  const income = totalIncome(filtered);
  const expenses = totalExpenses(filtered);

  return (
    <View style={{ ...pagePadding, gap: 12 }}>
      <View style={spaceBetween}>
        <View style={{ gap: 3 }}>
          <Text style={{ color: secondaryText(theme), fontSize: 12 }}>Activity</Text>
          <Text style={{ color: textColor(theme), fontSize: 24, fontWeight: "800" }}>Transactions</Text>
        </View>
        <Button label="Add" icon="plus" variant="primary" size="small" onClick={onAdd} />
      </View>

      <Card theme={theme} style={{ gap: 10 }}>
        <Input value={search} placeholder="Search merchant, title, category" searchable onChangeText={onSearchChange} />
        <Select
          value={categoryFilter}
          options={categoryOptions}
          onChange={(payload) => onCategoryChange(String(payload.value ?? "all"))}
        />
      </Card>

      <View style={{ flexDirection: "row", gap: 8 }}>
        <Card theme={theme} style={{ flex: 1, gap: 3 }}>
          <Text style={{ color: secondaryText(theme), fontSize: 11 }}>Income</Text>
          <Text style={{ color: colors.green, fontSize: 16, fontWeight: "800" }}>
            {currency === "USD" ? "$" : currency} {Math.round(income).toLocaleString("en-US")}
          </Text>
        </Card>
        <Card theme={theme} style={{ flex: 1, gap: 3 }}>
          <Text style={{ color: secondaryText(theme), fontSize: 11 }}>Expenses</Text>
          <Text style={{ color: colors.red, fontSize: 16, fontWeight: "800" }}>
            {currency === "USD" ? "$" : currency} {Math.round(expenses).toLocaleString("en-US")}
          </Text>
        </Card>
      </View>

      <SectionHeader title="Ledger" detail={`${filtered.length} items`} theme={theme} />
      <View style={{ gap: 8 }}>
        {filtered.map((transaction) => (
          <TransactionRow
            key={transaction.id}
            transaction={transaction}
            currency={currency}
            theme={theme}
            onClick={() => onOpenTransaction(transaction)}
          />
        ))}
        {filtered.length === 0 ? (
          <Card theme={theme}>
            <Text style={{ color: secondaryText(theme), fontSize: 13 }}>No transactions match this filter.</Text>
          </Card>
        ) : null}
      </View>
    </View>
  );
}
