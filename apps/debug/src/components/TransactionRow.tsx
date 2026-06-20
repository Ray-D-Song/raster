import { Text, View } from "raster-js/components";
import { AmountText } from "./AmountText";
import { Card } from "./Card";
import { categoryById } from "../model";
import { type AppTheme, row, secondaryText } from "../styles";
import type { CurrencyCode, Transaction } from "../types";

interface TransactionRowProps {
  transaction: Transaction;
  currency: CurrencyCode;
  theme: AppTheme;
  compact?: boolean;
  onClick?: () => void;
}

export function TransactionRow({ transaction, currency, theme, compact = false, onClick }: TransactionRowProps) {
  const category = categoryById(transaction.category);
  return (
    <Card theme={theme} style={{ padding: compact ? 10 : 12 }} >
      <View onClick={onClick} style={{ flexDirection: "row", alignItems: "center", gap: 10 }}>
        <View
          style={{
            width: 34,
            height: 34,
            borderRadius: 8,
            backgroundColor: category.color,
            alignItems: "center",
            justifyContent: "center",
          }}
        >
          <Text style={{ color: theme.primaryForeground, fontWeight: "700", fontSize: 13 }}>
            {category.name.slice(0, 1)}
          </Text>
        </View>
        <View style={{ flex: 1, gap: 2 }}>
          <Text style={{ fontWeight: "700", fontSize: compact ? 13 : 14 }}>
            {transaction.title}
          </Text>
          <View style={row}>
            <Text style={{ color: secondaryText(theme), fontSize: 11 }}>
              {transaction.merchant}
            </Text>
            <Text style={{ color: secondaryText(theme), fontSize: 11 }}> · {category.name}</Text>
          </View>
        </View>
        <View style={{ alignItems: "flex-end", gap: 2 }}>
          <AmountText
            amount={transaction.amount}
            currency={currency}
            type={transaction.type}
            theme={theme}
            size={13}
          />
          <Text style={{ color: secondaryText(theme), fontSize: 11 }}>{transaction.date.slice(5)}</Text>
        </View>
      </View>
    </Card>
  );
}
