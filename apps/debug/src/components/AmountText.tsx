import { Text } from "raster-js/components";
import { colors, textColor } from "../styles";
import { formatMoney } from "../model";
import type { CurrencyCode, ThemePreference, TransactionType } from "../types";

interface AmountTextProps {
  amount: number;
  currency: CurrencyCode;
  type?: TransactionType;
  size?: number;
  theme: ThemePreference;
}

export function AmountText({ amount, currency, type, size = 15, theme }: AmountTextProps) {
  const sign = type === "expense" ? -amount : amount;
  const color = type === "income" ? colors.green : type === "expense" ? colors.red : textColor(theme);
  return (
    <Text style={{ color, fontSize: size, fontWeight: "700" }}>
      {formatMoney(sign, currency)}
    </Text>
  );
}
