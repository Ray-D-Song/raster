import { createRoot } from "raster/react";
import {
  Avatar,
  AvatarGroup,
  Alert,
  AreaChart,
  Button,
  ButtonGroup,
  BarChart,
  CandlestickChart,
  Checkbox,
  ColorPicker,
  DatePicker,
  Dialog,
  Field,
  Form,
  Icon,
  Input,
  LineChart,
  PieChart,
  Radio,
  RadioGroup,
  Select,
  Sheet,
  Switch,
  Tab,
  TabBar,
  Text,
  Textarea,
  View,
  VirtualList,
  notification,
  type ChartRef,
} from "raster/components";
import { useRef, useState } from "react";

const virtualRows = Array.from({ length: 1000 }, (_, index) => ({
  id: index,
  value: index,
  label: `Virtual row ${index}`,
  description: `Only visible rows are rendered`,
  badge: index % 2 === 0 ? "even" : "odd",
}));

const chartData = [
  { month: "Jan", value: 42, mobile: 24, desktop: 32, color: "#0f7fd1" },
  { month: "Feb", value: 58, mobile: 32, desktop: 41, color: "#16a34a" },
  { month: "Mar", value: 49, mobile: 29, desktop: 38, color: "#f59e0b" },
  { month: "Apr", value: 68, mobile: 43, desktop: 49, color: "#dc2626" },
];

const candleData = [
  { date: "Jan", open: 42, high: 56, low: 38, close: 52 },
  { date: "Feb", open: 52, high: 61, low: 47, close: 49 },
  { date: "Mar", open: 49, high: 64, low: 45, close: 60 },
  { date: "Apr", open: 60, high: 72, low: 55, close: 66 },
];

const Block = () => {
  const [count, setCount] = useState(0);
  const [text, setText] = useState("Hello");
  const [bio, setBio] = useState("Multiline\nTextarea");
  const [channel, setChannel] = useState("stable");
  const [accentColor, setAccentColor] = useState("#0f7fd1");
  const [mode, setMode] = useState("read");
  const [checked, setChecked] = useState(false);
  const [enabled, setEnabled] = useState(true);
  const [radio, setRadio] = useState(0);
  const [tab, setTab] = useState(0);
  const [date, setDate] = useState<string | null>("2026-05-23");
  const [alertOpen, setAlertOpen] = useState(false);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [sheetOpen, setSheetOpen] = useState(false);
  const lineChartRef = useRef<ChartRef | null>(null);
  return (
    <View
      style={{
        overflow: "auto",
      }}
    >
      <AvatarGroup limit={2} ellipsis>
        <Avatar name="Raster" />
        <Avatar name="GPUI" />
        <Avatar name="JS" placeholder="user" />
      </AvatarGroup>
      <Text
        style={{
          color: "blue",
        }}
      >
        Count: {count}
      </Text>
      <Button onClick={() => setCount(count + 1)}>Click Me</Button>
      <Button onClick={() => setSheetOpen(true)} style={{ margin: { top: 12 } }}>
        Open Sheet
      </Button>
      <Button onClick={() => setDialogOpen(true)} style={{ margin: { top: 12 } }}>
        Open Dialog
      </Button>
      <Button onClick={() => setAlertOpen(true)} style={{ margin: { top: 12 } }}>
        Open Alert
      </Button>
      <ButtonGroup style={{ margin: { top: 12 } }}>
        <Button
          onClick={() =>
            notification.show({
              id: "demo-save",
              type: "success",
              title: "Saved",
              message: "Profile updated",
            })
          }
        >
          Show toast
        </Button>
        <Button onClick={() => notification.dismiss("demo-save")}>Dismiss toast</Button>
        <Button onClick={() => notification.clear()}>Clear toasts</Button>
      </ButtonGroup>
      <ButtonGroup
        value={mode}
        onChange={(value) => {
          if (typeof value === "string") setMode(value);
        }}
        style={{ margin: { top: 12 } }}
      >
        <Button value="read">Read</Button>
        <Button value="write">Write</Button>
        <Button value="preview">Preview</Button>
      </ButtonGroup>
      <Icon
        name="settings"
        size="large"
        color="blue"
        rotate={0.08}
        style={{ margin: { top: 12 } }}
      />
      <Text>Mode: {mode}</Text>
      <Checkbox
        checked={checked}
        onChange={(value) => setChecked(value === true)}
        style={{ margin: { top: 12 } }}
      >
        {checked ? "Checked" : "Unchecked"}
      </Checkbox>
      <Switch
        checked={enabled}
        onChange={(value) => setEnabled(value === true)}
        style={{ margin: { top: 12 } }}
      >
        {enabled ? "Enabled" : "Disabled"}
      </Switch>
      <RadioGroup
        layout="horizontal"
        selectedIndex={radio}
        onChange={(value) => setRadio(Number(value))}
        style={{ margin: { top: 12 } }}
      >
        <Radio label="React" />
        <Radio label="GPUI" />
      </RadioGroup>
      <TabBar
        variant="segmented"
        selectedIndex={tab}
        onClick={(value) => setTab(Number(value))}
        style={{ width: 260, margin: { top: 12 } }}
      >
        <Tab label="Core" />
        <Tab label="React" />
        <Tab label="Component" />
      </TabBar>
      <Text>Tab: {tab}</Text>
      <DatePicker
        value={date}
        placeholder="Pick date"
        cleanable
        disabled={{ dayOfWeek: [0, 6] }}
        onChange={(payload) => {
          if (typeof payload.value === "string" || payload.value === null) {
            setDate(payload.value);
          }
        }}
        style={{ width: 260, margin: { top: 12 } }}
      />
      <Text>Date: {date ?? "none"}</Text>
      <ColorPicker
        value={accentColor}
        label="Accent color"
        icon="palette"
        size="large"
        anchor="bottomLeft"
        featuredColors={["#0f7fd1", "#16a34a", "#dc2626", "#f59e0b", "#7c3aed"]}
        onChange={(payload) => {
          if (payload.value != null) setAccentColor(payload.value);
        }}
        style={{ margin: { top: 12 } }}
      />
      <Text>Color: {accentColor}</Text>
      <Form
        layout="horizontal"
        columns={2}
        labelWidth={120}
        style={{ width: 620, margin: { top: 12 } }}
      >
        <Field
          label="Name"
          value={text}
          required
          description="At least 3 characters."
          validate={(value) => {
            const inputValue = typeof value === "string" ? value : "";
            return {
              error: inputValue.trim().length < 3,
              message: "Name must be at least 3 characters.",
            };
          }}
        >
          <Input
            value={text}
            placeholder="Type here"
            onChangeText={(value) => setText(value)}
          />
        </Field>
        <Field label="Channel" value={channel} description="Release channel.">
          <Select
            value={channel}
            placeholder="Channel"
            options={[
              { id: "stable-form", value: "stable", label: "Stable" },
              { id: "nightly-form", value: "nightly", label: "Nightly" },
            ]}
            onChange={(payload) => {
              if (typeof payload.value === "string") setChannel(payload.value);
            }}
          />
        </Field>
        <Field label="Date" value={date} description="Weekend dates are disabled.">
          <DatePicker
            value={date}
            placeholder="Pick date"
            disabled={{ dayOfWeek: [0, 6] }}
            onChange={(payload) => {
              if (typeof payload.value === "string" || payload.value === null) {
                setDate(payload.value);
              }
            }}
          />
        </Field>
        <Field label="Accent" value={accentColor}>
          <ColorPicker
            value={accentColor}
            featuredColors={["#0f7fd1", "#16a34a", "#dc2626", "#f59e0b", "#7c3aed"]}
            onChange={(payload) => {
              if (payload.value != null) setAccentColor(payload.value);
            }}
          />
        </Field>
        <Field labelIndent={false} colSpan={2}>
          <Switch checked={enabled} onChange={(value) => setEnabled(value === true)}>
            {enabled ? "Enabled" : "Disabled"}
          </Switch>
        </Field>
      </Form>
      <View style={{ width: 760, gap: 12, margin: { top: 16 } }}>
        <Text>Charts</Text>
        <Button
          onClick={() => {
            lineChartRef.current?.appendData({
              month: `T${count + 1}`,
              value: 45 + ((count * 9) % 28),
              mobile: 30 + ((count * 7) % 18),
              desktop: 36 + ((count * 5) % 20),
              color: "#7c3aed",
            });
            setCount((value) => value + 1);
          }}
        >
          Append line point
        </Button>
        <LineChart
          ref={lineChartRef}
          data={chartData}
          maxDataLength={8}
          x="month"
          y="value"
          stroke="#0f7fd1"
          interpolation="linear"
          dot
          style={{ height: 180, borderWidth: 1, borderColor: "#dddddd" }}
        />
        <BarChart
          data={chartData}
          band="month"
          value="value"
          fill="color"
          label="value"
          cornerRadius={4}
          style={{ height: 180, borderWidth: 1, borderColor: "#dddddd" }}
        />
        <AreaChart
          data={chartData}
          x="month"
          series={[
            { y: "mobile", stroke: "#16a34a", fill: "#16a34a", interpolation: "linear" },
            { y: "desktop", stroke: "#7c3aed", fill: "#7c3aed", interpolation: "natural" },
          ]}
          style={{ height: 180, borderWidth: 1, borderColor: "#dddddd" }}
        />
        <PieChart
          data={chartData}
          value="value"
          color="color"
          innerRadius={42}
          padAngle={0.04}
          style={{ width: 260, height: 220, borderWidth: 1, borderColor: "#dddddd" }}
        />
        <CandlestickChart
          data={candleData}
          x="date"
          open="open"
          high="high"
          low="low"
          close="close"
          bodyWidthRatio={0.6}
          style={{ height: 180, borderWidth: 1, borderColor: "#dddddd" }}
        />
      </View>
      <Sheet
        open={sheetOpen}
        title="Raster Sheet"
        placement="right"
        size={420}
        overlay
        overlayClosable
        resizable
        onOpenChange={(event) => setSheetOpen(event.open)}
      >
        <View style={{ gap: 12 }}>
          <Text style={{ color: "blue" }}>Sheet content</Text>
          <Input
            value={text}
            placeholder="Edit from sheet"
            onChangeText={(value) => setText(value)}
            style={{ width: 260 }}
          />
          <Button onClick={() => setSheetOpen(false)}>Close Sheet</Button>
        </View>
      </Sheet>
      <Dialog
        open={dialogOpen}
        title="Raster Dialog"
        confirm
        okText="Save"
        cancelText="Cancel"
        width={420}
        overlay
        overlayClosable
        onOk={() => setCount((value) => value + 1)}
        onOpenChange={(event) => setDialogOpen(event.open)}
      >
        <View style={{ gap: 12 }}>
          <Text style={{ color: "blue" }}>Dialog content</Text>
          <Input
            value={text}
            placeholder="Edit from dialog"
            onChangeText={(value) => setText(value)}
            style={{ width: 260 }}
          />
          <Button onClick={() => setDialogOpen(false)}>Close Dialog</Button>
        </View>
      </Dialog>
      <Alert
        open={alertOpen}
        title="Delete account?"
        description="This action cannot be undone. The account will be permanently removed."
        icon="warning"
        showCancel
        okText="Continue"
        cancelText="Cancel"
        okVariant="danger"
        onOk={() => setCount((value) => value + 1)}
        onOpenChange={(event) => setAlertOpen(event.open)}
      >
        <Text style={{ color: "blue" }}>Alert children render in the body.</Text>
      </Alert>
      <Input
        value={text}
        placeholder="Type here"
        onChangeText={(value) => setText(value)}
        onSubmitEditing={(value) => setCount(value.length)}
        style={{ width: 260, margin: { top: 12 } }}
      />
      <Textarea
        value={bio}
        rows={3}
        onChangeText={(value) => {
          console.log("new value", value);
          setBio(value);
        }}
        style={{ width: 260, margin: { top: 12 } }}
      />
      <Select
        value={channel}
        placeholder="Channel"
        cleanable
        options={[
          { id: "stable", value: "stable", label: "Stable" },
          { id: "nightly", value: "nightly", label: "Nightly" },
          {
            id: "disabled",
            value: "disabled",
            label: "Disabled",
            disabled: true,
          },
        ]}
        onChange={(payload) => {
          if (typeof payload.value === "string") setChannel(payload.value);
        }}
        style={{ width: 260, margin: { top: 12 } }}
      />
      <Text>Input: {text}</Text>
      <Text>Channel: {channel}</Text>
      <VirtualList
        items={virtualRows}
        itemSize={44}
        keyExtractor={(item) => String(item.id)}
        renderItem={({ item }) => (
          <View
            onClick={() => {
              console.log({ item });
            }}
            style={{
              height: 44,
              padding: { top: 4, right: 12, bottom: 4, left: 12 },
            }}
          >
            <Text>{String(item.label ?? "")}</Text>
            <Text style={{ color: "blue", fontSize: 13 }}>
              {String(item.description ?? "")}
            </Text>
          </View>
        )}
        style={{
          width: 360,
          height: 260,
          borderWidth: 1,
          borderColor: "#dddddd",
          borderRadius: 8,
        }}
      />
    </View>
  );
};

const App = () => {
  return (
    <Block />
  );
};
const root = createRoot({
  width: 800,
  height: 600,
  perfdetect: true,
});

root.render(<App />);
