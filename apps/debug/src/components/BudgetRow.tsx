import { Icon, Text, View } from "raster-js/components";
import { Card } from "./Card";
import { ProgressBar } from "./ProgressBar";
import { categoryById, formatMoney, spentForCategory } from "../model";
import { colors, secondaryText, spaceBetween, textColor } from "../styles";
import type { Budget, CurrencyCode, ThemePreference, Transaction } from "../types";

interface BudgetRowProps {
  budget: Budget;
  transactions: Transaction[];
  currency: CurrencyCode;
  theme: ThemePreference;
}

export function BudgetRow({ budget, transactions, currency, theme }: BudgetRowProps) {
  const category = categoryById(budget.category);
  const spent = spentForCategory(transactions, budget.category);
  const remaining = budget.limit - spent;
  const progress = Math.min(1, spent / budget.limit);
  const over = remaining < 0;

  return (
    <Card theme={theme}>
      <View style={{ gap: 10 }}>
        <View style={spaceBetween}>
          <View style={{ flexDirection: "row", alignItems: "center", gap: 8 }}>
            <View style={{ width: 10, height: 10, borderRadius: 5, backgroundColor: budget.color }} />
            <Text style={{ color: textColor(theme), fontSize: 14, fontWeight: "700" }}>{category.name}</Text>
          </View>
          {over ? <Icon name="warning" color={colors.red} size="small" /> : null}
        </View>
        <ProgressBar value={progress} color={over ? colors.red : budget.color} theme={theme} />
        <View style={spaceBetween}>
          <Text style={{ color: secondaryText(theme), fontSize: 12 }}>
            {formatMoney(spent, currency)} of {formatMoney(budget.limit, currency)}
          </Text>
          <Text style={{ color: over ? colors.red : colors.green, fontSize: 12, fontWeight: "700" }}>
            {over ? `${formatMoney(Math.abs(remaining), currency)} over` : `${formatMoney(remaining, currency)} left`}
          </Text>
        </View>
      </View>
    </Card>
  );
}
