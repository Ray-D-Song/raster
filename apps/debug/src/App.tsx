import { useMemo, useState } from "react";
import { Alert, AppShell, AppShellTab, AppShellTabBar, ConfigProvider, Dialog, Text, View, useTheme } from "raster-js/components";
import { AddTransactionDialog } from "./components/AddTransactionDialog";
import { defaultDraft, defaultSettings, seedBudgets, seedTransactions } from "./data";
import { categoryById, formatMoney, makeTransaction } from "./model";
import { Budget } from "./pages/Budget";
import { Overview } from "./pages/Overview";
import { Settings } from "./pages/Settings";
import { Transactions } from "./pages/Transactions";
import { borderColor, panelBackground, secondaryText } from "./styles";
import type { AppTab, NewTransactionDraft, Transaction, UserSettings } from "./types";

export function App() {
  const [tab, setTab] = useState<AppTab>("overview");
  const [transactions, setTransactions] = useState<Transaction[]>(seedTransactions);
  const [settings, setSettings] = useState<UserSettings>(defaultSettings);
  const [search, setSearch] = useState("");
  const [categoryFilter, setCategoryFilter] = useState("all");
  const [draft, setDraft] = useState<NewTransactionDraft>(defaultDraft);
  const [addOpen, setAddOpen] = useState(false);
  const [error, setError] = useState("");
  const [selectedTransaction, setSelectedTransaction] = useState<Transaction | null>(null);
  const [nextId, setNextId] = useState(1);

  const currency = settings.currency;
  const theme = settings.theme;
  const nativeTheme = useTheme();
  const appTheme = nativeTheme?.colors ?? null;
  const selectedCategory = selectedTransaction ? categoryById(selectedTransaction.category) : null;

  const activePage = useMemo(() => {
    if (appTheme == null) return null;
    if (tab === "transactions") {
      return (
        <Transactions
          transactions={transactions}
          currency={currency}
          theme={appTheme}
          search={search}
          categoryFilter={categoryFilter}
          onSearchChange={setSearch}
          onCategoryChange={setCategoryFilter}
          onAdd={() => setAddOpen(true)}
          onOpenTransaction={setSelectedTransaction}
        />
      );
    }
    if (tab === "budget") {
      return (
        <Budget
          budgets={seedBudgets}
          transactions={transactions}
          currency={currency}
          theme={appTheme}
          alertsEnabled={settings.budgetAlerts}
          onAlertsChange={(budgetAlerts) => setSettings((current) => ({ ...current, budgetAlerts }))}
        />
      );
    }
    if (tab === "settings") {
      return (
        <Settings
          settings={settings}
          theme={appTheme}
          onChange={setSettings}
          transactionCount={transactions.length}
        />
      );
    }
    return (
      <Overview
        transactions={transactions}
        budgets={seedBudgets}
        currency={currency}
        theme={appTheme}
        onAdd={() => setAddOpen(true)}
        onOpenTransaction={setSelectedTransaction}
        onNavigate={setTab}
      />
    );
  }, [appTheme, categoryFilter, currency, search, settings, tab, transactions]);

  const closeAdd = () => {
    setAddOpen(false);
    setDraft(defaultDraft);
  };

  const submitDraft = () => {
    const title = draft.title.trim();
    const amount = Number(draft.amount);
    if (title.length === 0) {
      setError("Transaction title is required.");
      return;
    }
    if (!Number.isFinite(amount) || amount <= 0) {
      setError("Enter a positive transaction amount.");
      return;
    }
    const transaction = makeTransaction(draft, nextId);
    setTransactions((current) => [transaction, ...current]);
    setNextId((value) => value + 1);
    closeAdd();
    setTab("transactions");
  };

  return (
    <ConfigProvider theme={{ mode: theme }}>
      <AppShell
        theme={theme}
        tabBar={
          <AppShellTabBar value={tab} theme={theme} onValueChange={(value) => setTab(value as AppTab)}>
            <AppShellTab value="overview" label="Overview" icon="layout-dashboard" />
            <AppShellTab value="transactions" label="Activity" icon="file" />
            <AppShellTab value="budget" label="Budget" icon="chart-pie" />
            <AppShellTab value="settings" label="Settings" icon="circle-user" />
          </AppShellTabBar>
        }
      >
        {activePage}
        {appTheme ? (
          <AddTransactionDialog
            open={addOpen}
            draft={draft}
            theme={appTheme}
            onChange={setDraft}
            onCancel={closeAdd}
            onSubmit={submitDraft}
          />
        ) : null}

        <Alert
          open={error.length > 0}
          title="Check transaction"
          description={error}
          icon="warning"
          okText="Got it"
          onOk={() => setError("")}
          onOpenChange={(event) => {
            if (!event.open) setError("");
          }}
        />

        <Dialog
          open={selectedTransaction != null}
          title={selectedTransaction?.title ?? "Transaction"}
          width={340}
          closeButton
          onCancel={() => setSelectedTransaction(null)}
          onOpenChange={(event) => {
            if (!event.open) setSelectedTransaction(null);
          }}
        >
          {selectedTransaction && selectedCategory && appTheme ? (
            <View style={{ gap: 12 }}>
              <View
                style={{
                  backgroundColor: panelBackground(appTheme),
                  borderColor: borderColor(appTheme),
                  borderWidth: 1,
                  borderRadius: 8,
                  padding: 12,
                  gap: 8,
                }}
              >
                <Text style={{ color: secondaryText(appTheme), fontSize: 12 }}>{selectedTransaction.merchant}</Text>
                <Text
                  style={{
                    color: selectedTransaction.type === "income" ? appTheme.success : appTheme.danger,
                    fontSize: 28,
                    fontWeight: "800",
                  }}
                >
                  {formatMoney(
                    selectedTransaction.type === "expense" ? -selectedTransaction.amount : selectedTransaction.amount,
                    currency
                  )}
                </Text>
              </View>
              <View style={{ gap: 6 }}>
                <Text style={{ fontSize: 13, fontWeight: "700" }}>Category</Text>
                <Text style={{ color: secondaryText(appTheme), fontSize: 13 }}>{selectedCategory.name}</Text>
              </View>
              <View style={{ gap: 6 }}>
                <Text style={{ fontSize: 13, fontWeight: "700" }}>Date</Text>
                <Text style={{ color: secondaryText(appTheme), fontSize: 13 }}>{selectedTransaction.date}</Text>
              </View>
              {selectedTransaction.note ? (
                <View style={{ gap: 6 }}>
                  <Text style={{ fontSize: 13, fontWeight: "700" }}>Note</Text>
                  <Text style={{ color: secondaryText(appTheme), fontSize: 13 }}>{selectedTransaction.note}</Text>
                </View>
              ) : null}
            </View>
          ) : null}
        </Dialog>
      </AppShell>
    </ConfigProvider>
  );
}
