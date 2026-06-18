import { Button, Dialog, Input, Select, Text, View } from "raster-js/components";
import { categories, defaultDraft } from "../data";
import { colors, secondaryText, textColor } from "../styles";
import type { NewTransactionDraft, ThemePreference, TransactionType } from "../types";

interface AddTransactionDialogProps {
  open: boolean;
  draft: NewTransactionDraft;
  theme: ThemePreference;
  onChange: (draft: NewTransactionDraft) => void;
  onCancel: () => void;
  onSubmit: () => void;
}

const typeOptions = [
  { id: "expense", label: "Expense", value: "expense" },
  { id: "income", label: "Income", value: "income" },
];

const categoryOptions = categories
  .filter((category) => category.id !== "income")
  .map((category) => ({ id: category.id, label: category.name, value: category.id }));

export function AddTransactionDialog({
  open,
  draft,
  theme,
  onChange,
  onCancel,
  onSubmit,
}: AddTransactionDialogProps) {
  return (
    <Dialog open={open} title="Add transaction" width={360} closeButton onCancel={onCancel} onOpenChange={(event) => {
      if (!event.open) onCancel();
    }}>
      <View style={{ gap: 12 }}>
        <Text style={{ color: secondaryText(theme), fontSize: 12 }}>
          Add an offline transaction to test list updates, budgets, and dashboard totals.
        </Text>
        <Input
          value={draft.title}
          placeholder="Title"
          onChangeText={(title) => onChange({ ...draft, title })}
        />
        <Input
          value={draft.merchant}
          placeholder="Merchant"
          onChangeText={(merchant) => onChange({ ...draft, merchant })}
        />
        <Input
          value={draft.amount}
          placeholder="Amount"
          maskPattern={{ kind: "number", separator: "," }}
          onChangeText={(amount) => onChange({ ...draft, amount })}
        />
        <Select
          value={draft.type}
          options={typeOptions}
          onChange={(payload) => onChange({ ...draft, type: String(payload.value ?? "expense") as TransactionType })}
        />
        {draft.type === "expense" ? (
          <Select
            value={draft.category}
            options={categoryOptions}
            onChange={(payload) => onChange({ ...draft, category: String(payload.value ?? defaultDraft.category) })}
          />
        ) : null}
        <Input value={draft.date} placeholder="YYYY-MM-DD" onChangeText={(date) => onChange({ ...draft, date })} />
        <Input value={draft.note} placeholder="Note" onChangeText={(note) => onChange({ ...draft, note })} />
        <View style={{ flexDirection: "row", gap: 8, justifyContent: "flex-end" }}>
          <Button label="Cancel" variant="secondary" onClick={onCancel} />
          <Button label="Add" variant="primary" onClick={onSubmit} />
        </View>
        <Text style={{ color: textColor(theme), fontSize: 11 }}>
          Expenses affect category budgets. Income updates the balance summary.
        </Text>
        <Text style={{ color: colors.faint, fontSize: 10 }}>Data resets when the debug app restarts.</Text>
      </View>
    </Dialog>
  );
}
