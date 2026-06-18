import { Button, Icon, Switch, Text, View } from "raster-js/components";
import { BudgetRow } from "../components/BudgetRow";
import { Card } from "../components/Card";
import { SectionHeader } from "../components/SectionHeader";
import { formatMoney, spentForCategory } from "../model";
import { colors, pagePadding, secondaryText, spaceBetween, textColor } from "../styles";
import type { Budget as BudgetModel, CurrencyCode, ThemePreference, Transaction } from "../types";

interface BudgetProps {
  budgets: BudgetModel[];
  transactions: Transaction[];
  currency: CurrencyCode;
  theme: ThemePreference;
  alertsEnabled: boolean;
  onAlertsChange: (value: boolean) => void;
}

export function Budget({
  budgets,
  transactions,
  currency,
  theme,
  alertsEnabled,
  onAlertsChange,
}: BudgetProps) {
  const totalLimit = budgets.reduce((total, budget) => total + budget.limit, 0);
  const totalSpent = budgets.reduce((total, budget) => total + spentForCategory(transactions, budget.category), 0);
  const remaining = totalLimit - totalSpent;
  const overCount = budgets.filter((budget) => spentForCategory(transactions, budget.category) > budget.limit).length;

  return (
    <View style={{ ...pagePadding, gap: 12 }}>
      <View style={{ gap: 3 }}>
        <Text style={{ color: secondaryText(theme), fontSize: 12 }}>Monthly plan</Text>
        <Text style={{ color: textColor(theme), fontSize: 24, fontWeight: "800" }}>Budget</Text>
      </View>

      <Card theme={theme} style={{ gap: 12 }}>
        <View style={spaceBetween}>
          <View style={{ gap: 3 }}>
            <Text style={{ color: secondaryText(theme), fontSize: 12 }}>Remaining this cycle</Text>
            <Text style={{ color: remaining < 0 ? colors.red : textColor(theme), fontSize: 28, fontWeight: "800" }}>
              {formatMoney(remaining, currency)}
            </Text>
          </View>
          <Icon name={remaining < 0 ? "warning" : "circle-check"} color={remaining < 0 ? colors.red : colors.green} />
        </View>
        <Text style={{ color: secondaryText(theme), fontSize: 12 }}>
          {formatMoney(totalSpent, currency)} spent from {formatMoney(totalLimit, currency)} planned.
        </Text>
      </Card>

      <Card theme={theme}>
        <View style={spaceBetween}>
          <View style={{ gap: 3 }}>
            <Text style={{ color: textColor(theme), fontSize: 14, fontWeight: "700" }}>Smart budget alerts</Text>
            <Text style={{ color: secondaryText(theme), fontSize: 11 }}>
              {overCount > 0 ? `${overCount} category needs attention` : "All tracked categories are on plan"}
            </Text>
          </View>
          <Switch checked={alertsEnabled} onChange={(value) => onAlertsChange(value === true)} />
        </View>
      </Card>

      <View style={{ gap: 8 }}>
        <SectionHeader title="Category budgets" detail="June" theme={theme} />
        {budgets.map((budget) => (
          <BudgetRow key={budget.category} budget={budget} transactions={transactions} currency={currency} theme={theme} />
        ))}
      </View>

      <Button label="Review transactions" variant="secondary" icon="file" />
    </View>
  );
}
