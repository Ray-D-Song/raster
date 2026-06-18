import { BarChart, Button, Icon, Text, View } from "raster-js/components";
import { AmountText } from "../components/AmountText";
import { BudgetRow } from "../components/BudgetRow";
import { Card } from "../components/Card";
import { SectionHeader } from "../components/SectionHeader";
import { TransactionRow } from "../components/TransactionRow";
import {
  budgetProgress,
  formatMoney,
  monthTransactions,
  spendingByCategory,
  totalExpenses,
  totalIncome,
} from "../model";
import { colors, pagePadding, row, secondaryText, spaceBetween, textColor } from "../styles";
import type { AppTab, Budget, CurrencyCode, ThemePreference, Transaction } from "../types";

interface OverviewProps {
  transactions: Transaction[];
  budgets: Budget[];
  currency: CurrencyCode;
  theme: ThemePreference;
  onAdd: () => void;
  onOpenTransaction: (transaction: Transaction) => void;
  onNavigate: (tab: AppTab) => void;
}

export function Overview({
  transactions,
  budgets,
  currency,
  theme,
  onAdd,
  onOpenTransaction,
  onNavigate,
}: OverviewProps) {
  const current = monthTransactions(transactions);
  const income = totalIncome(current);
  const expenses = totalExpenses(current);
  const balance = income - expenses;
  const spending = spendingByCategory(current);
  const topBudget = budgets
    .map((budget) => ({ budget, progress: budgetProgress(budget, current) }))
    .sort((a, b) => b.progress - a.progress)[0]?.budget;
  const chartData = spending.map((item) => ({
    label: item.category.name,
    value: item.spent,
    color: item.category.color,
  }));

  return (
    <View style={[pagePadding, { gap: 14 }]}>
      <View style={spaceBetween}>
        <View style={{ gap: 3 }}>
          <Text style={{ color: secondaryText(theme), fontSize: 12 }}>Thursday, June 18</Text>
          <Text style={{ color: textColor(theme), fontSize: 24, fontWeight: "800" }}>Pocket Ledger</Text>
        </View>
        <Button label="Add" icon="plus" variant="primary" size="small" onClick={onAdd} />
      </View>

      <Card theme={theme} style={{ gap: 14 }}>
        <View style={spaceBetween}>
          <View style={{ gap: 3 }}>
            <Text style={{ color: secondaryText(theme), fontSize: 12 }}>June balance</Text>
            <Text style={{ color: textColor(theme), fontSize: 32, fontWeight: "800" }}>
              {formatMoney(balance, currency)}
            </Text>
          </View>
          <View
            style={{
              width: 44,
              height: 44,
              borderRadius: 8,
              backgroundColor: "#dbeafe",
              alignItems: "center",
              justifyContent: "center",
            }}
          >
            <Icon name="chart-pie" color={colors.blue} size="medium" />
          </View>
        </View>
        <View style={{ flexDirection: "row", gap: 10 }}>
          <View style={{ flex: 1, gap: 3 }}>
            <Text style={{ color: secondaryText(theme), fontSize: 11 }}>Income</Text>
            <AmountText amount={income} currency={currency} type="income" theme={theme} />
          </View>
          <View style={{ flex: 1, gap: 3 }}>
            <Text style={{ color: secondaryText(theme), fontSize: 11 }}>Spent</Text>
            <AmountText amount={expenses} currency={currency} type="expense" theme={theme} />
          </View>
        </View>
      </Card>

      <Card theme={theme} style={{ gap: 10 }}>
        <SectionHeader title="Spending mix" detail="This month" theme={theme} />
        {chartData.length > 0 ? (
          <BarChart
            data={chartData}
            band="label"
            value="value"
            fill="color"
            height={130}
            cornerRadius={5}
            labelAxis
          />
        ) : (
          <Text style={{ color: secondaryText(theme), fontSize: 13 }}>No spending yet.</Text>
        )}
      </Card>

      {topBudget ? (
        <View style={{ gap: 8 }}>
          <SectionHeader title="Budget pressure" detail="Highest usage" theme={theme} />
          <BudgetRow budget={topBudget} transactions={current} currency={currency} theme={theme} />
        </View>
      ) : null}

      <View style={{ gap: 8 }}>
        <View style={spaceBetween}>
          <SectionHeader title="Recent activity" theme={theme} />
          <Button label="View all" variant="text" size="small" onClick={() => onNavigate("transactions")} />
        </View>
        {current.slice(0, 4).map((transaction) => (
          <TransactionRow
            key={transaction.id}
            transaction={transaction}
            currency={currency}
            theme={theme}
            compact
            onClick={() => onOpenTransaction(transaction)}
          />
        ))}
      </View>

      <View style={[row, { gap: 6, justifyContent: "center" }]}>
        <Icon name="circle-check" color={colors.green} size="small" />
        <Text style={{ color: secondaryText(theme), fontSize: 11 }}>Offline data. No bank connection required.</Text>
      </View>
    </View>
  );
}
