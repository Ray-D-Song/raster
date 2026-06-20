import { Text } from "raster-js/components";
import type { AppTheme } from "../styles";
import { formatMoney } from "../model";
import type { CurrencyCode, TransactionType } from "../types";

interface AmountTextProps {
  amount: number;
  currency: CurrencyCode;
  type?: TransactionType;
  size?: number;
  theme: AppTheme;
}

export function AmountText({ amount, currency, type, size = 15, theme }: AmountTextProps) {
  const sign = type === "expense" ? -amount : amount;
  const color = type === "income" ? theme.success : type === "expense" ? theme.danger : null;
  return (
    <Text style={color == null ? { fontSize: size, fontWeight: "700" } : { color, fontSize: size, fontWeight: "700" }}>
      {formatMoney(sign, currency)}
    </Text>
  );
}
