import type { ComponentType, MutableRefObject, ReactElement } from "react";
import { forwardRef, useEffect, useImperativeHandle, useRef, useState } from "react";
import { jsx } from "react/jsx-runtime";

import { ConfigProvider, Input, Label, Text, Textarea, View, Widget } from "../core/index.js";
import type {
  ButtonCustomVariant,
  ButtonRounded,
  ButtonVariant,
  AlertVariant,
  ComponentAxis,
  ComponentBaseProps,
  ComponentSize,
  ConfigProviderProps,
  FieldAlign,
  GenericComponentProps,
  IconName,
  InputProps,
  JsonObject,
  JsonValue,
  LabelProps,
  RasterEventHandler,
  RasterNotificationShowOptions,
  RasterNotificationType,
  RasterQueryHandler,
  RasterStyle,
  RasterStyleInput,
  RasterThemeColors,
  RasterThemeConfig,
  RasterThemeMode,
  StyleDimension,
  TabVariant,
  TextChangePayload,
  TextareaProps,
  TextProps,
  ViewProps,
} from "../core/types.js";

export type {
  ButtonCustomVariant,
  ButtonRounded,
  ButtonVariant,
  AlertVariant,
  ComponentAxis,
  ComponentBaseProps,
  ComponentSize,
  ConfigProviderProps,
  FieldAlign,
  GenericComponentProps,
  IconName,
  InputProps,
  LabelProps,
  RasterNotificationShowOptions,
  RasterNotificationType,
  RasterQueryHandler,
  RasterStyle,
  RasterStyleInput,
  RasterThemeColors,
  RasterThemeConfig,
  RasterThemeMode,
  StyleDimension,
  TabVariant,
  TextChangePayload,
  TextareaProps,
  TextProps,
  ViewProps,
} from "../core/types.js";

export { ConfigProvider, Input, Label, Text, Textarea, View } from "../core/index.js";

type RasterRuntimeGlobal = typeof globalThis & {
  __rasterNative?: {
    notificationShow?(options: RasterNotificationShowOptions): void;
    notificationDismiss?(id: string): void;
    notificationClear?(): void;
    chartAppendData?(handle: { surface_id: number; node_tag: number }, rows: JsonValue[]): void;
    chartReplaceData?(handle: { surface_id: number; node_tag: number }, rows: JsonValue[]): void;
    chartClearData?(handle: { surface_id: number; node_tag: number }): void;
  };
  __rasterFlushSyncWork?: () => void;
};

function nativeBinding() {
  const binding = (globalThis as RasterRuntimeGlobal).__rasterNative;
  if (binding == null) {
    throw new Error("Raster notification API requires globalThis.__rasterNative");
  }
  return binding;
}

export const notification = {
  show(options: RasterNotificationShowOptions): void {
    const binding = nativeBinding();
    if (binding.notificationShow == null) {
      throw new Error("Raster notification.show is not available in this runtime");
    }
    binding.notificationShow(options);
  },
  dismiss(id: string): void {
    const binding = nativeBinding();
    if (binding.notificationDismiss == null) {
      throw new Error("Raster notification.dismiss is not available in this runtime");
    }
    binding.notificationDismiss(id);
  },
  clear(): void {
    const binding = nativeBinding();
    if (binding.notificationClear == null) {
      throw new Error("Raster notification.clear is not available in this runtime");
    }
    binding.notificationClear();
  },
};

export const componentNames = [
  "Avatar",
  "AvatarGroup",
  "Alert",
  "Button",
  "ButtonGroup",
  "Checkbox",
  "ColorPicker",
  "DatePicker",
  "Dialog",
  "Field",
  "Form",
  "Icon",
  "LineChart",
  "BarChart",
  "AreaChart",
  "PieChart",
  "CandlestickChart",
  "Radio",
  "RadioGroup",
  "Select",
  "Sheet",
  "Switch",
  "Tab",
  "TabBar",
  "VirtualList",
] as const;

export type ComponentName = (typeof componentNames)[number];

export interface AvatarProps extends ComponentBaseProps {
  name?: JsonValue;
  src?: string;
  size?: ComponentSize;
  placeholder?: IconName;
}

export interface AvatarSpec {
  name?: JsonValue;
  src?: string;
  placeholder?: IconName;
  icon?: IconName;
}

export interface AvatarGroupProps extends ComponentBaseProps {
  names?: string[];
  avatars?: Array<string | AvatarSpec>;
  items?: Array<string | AvatarSpec>;
  limit?: number;
  ellipsis?: boolean;
  size?: ComponentSize;
}

export interface ButtonProps extends ComponentBaseProps {
  label?: JsonValue;
  value?: JsonValue;
  size?: ComponentSize;
  variant?: ButtonVariant;
  disabled?: boolean;
  selected?: boolean;
  loading?: boolean;
  loadingIcon?: IconName;
  compact?: boolean;
  outline?: boolean;
  rounded?: ButtonRounded;
  dropdownCaret?: boolean;
  tabIndex?: number;
  tabStop?: boolean;
  tooltip?: string;
  icon?: IconName;
  customVariant?: ButtonCustomVariant;
  onClick?: RasterEventHandler<string>;
  onHover?: RasterEventHandler<boolean | string>;
}

export interface ButtonGroupProps extends ComponentBaseProps {
  size?: ComponentSize;
  variant?: ButtonVariant;
  value?: JsonValue;
  disabled?: boolean;
  multiple?: boolean;
  compact?: boolean;
  outline?: boolean;
  layout?: ComponentAxis;
  axis?: ComponentAxis;
  customVariant?: ButtonCustomVariant;
  onClick?: RasterEventHandler<string>;
  onChange?: RasterEventHandler<JsonValue>;
}

export type AlertOpenChangeReason = "ok" | "cancel" | "controlled";

export interface AlertOpenChangePayload {
  open: boolean;
  reason: AlertOpenChangeReason;
}

export interface AlertProps extends ComponentBaseProps {
  open?: boolean;
  title?: JsonValue;
  description?: JsonValue;
  icon?: IconName;
  showCancel?: boolean;
  okText?: JsonValue;
  cancelText?: JsonValue;
  okVariant?: ButtonVariant;
  cancelVariant?: ButtonVariant;
  width?: number;
  overlayClosable?: boolean;
  keyboard?: boolean;
  closeButton?: boolean;
  onOk?: RasterEventHandler<string>;
  onCancel?: RasterEventHandler<string>;
  onClose?: RasterEventHandler<string>;
  onOpenChange?: RasterEventHandler<AlertOpenChangePayload>;
}

export interface CheckboxProps extends ComponentBaseProps {
  label?: JsonValue;
  size?: ComponentSize;
  checked?: boolean;
  selected?: boolean;
  disabled?: boolean;
  tabIndex?: number;
  tabStop?: boolean;
  onChange?: RasterEventHandler<boolean | string>;
  onClick?: RasterEventHandler<boolean | string>;
}

export type ColorPickerAnchor =
  | "topLeft"
  | "topCenter"
  | "topRight"
  | "bottomLeft"
  | "bottomCenter"
  | "bottomRight";

export interface ColorPickerChangePayload {
  value: string | null;
}

export interface ColorPickerProps extends ComponentBaseProps {
  value?: string | null;
  defaultValue?: string;
  featuredColors?: string[];
  label?: JsonValue;
  icon?: IconName;
  size?: ComponentSize;
  anchor?: ColorPickerAnchor;
  onChange?: RasterEventHandler<ColorPickerChangePayload>;
}

export type ChartDatum = Record<string, JsonValue>;
export type ChartInterpolation = "natural" | "linear" | "stepAfter";
export type BarChartAlignment = "top" | "right" | "bottom" | "left";

export interface ChartRef {
  appendData(rowOrRows: ChartDatum | ChartDatum[]): void;
  replaceData(rows: ChartDatum[]): void;
  clearData(): void;
}

interface ChartBaseProps extends ComponentBaseProps {
  data?: ChartDatum[];
  maxDataLength?: number;
  tickMargin?: number;
  grid?: boolean;
  width?: StyleDimension;
  height?: StyleDimension;
  minHeight?: StyleDimension;
  maxHeight?: StyleDimension;
}

export interface LineChartProps extends ChartBaseProps {
  x?: string;
  y?: string;
  stroke?: string;
  interpolation?: ChartInterpolation;
  dot?: boolean;
  xAxis?: boolean;
}

export interface BarChartProps extends ChartBaseProps {
  band?: string;
  value?: string;
  label?: string;
  fill?: string;
  alignment?: BarChartAlignment;
  cornerRadius?: number;
  labelAxis?: boolean;
}

export interface AreaChartSeries {
  y: string;
  stroke?: string;
  fill?: string;
  interpolation?: ChartInterpolation;
}

export interface AreaChartProps extends ChartBaseProps {
  x?: string;
  y?: string;
  series?: AreaChartSeries[];
  stroke?: string;
  fill?: string;
  interpolation?: ChartInterpolation;
  xAxis?: boolean;
}

export interface PieChartProps extends ChartBaseProps {
  value?: string;
  color?: string;
  innerRadius?: number;
  outerRadius?: number;
  padAngle?: number;
}

export interface CandlestickChartProps extends ChartBaseProps {
  x?: string;
  open?: string;
  high?: string;
  low?: string;
  close?: string;
  bodyWidthRatio?: number;
  xAxis?: boolean;
}

export type DateSelectionMode = "single" | "range";
export type DateValue = string | null;
export type DateRangeValue = [DateValue, DateValue];
export type DatePickerValue = DateValue | DateRangeValue;

export type DateDisabledMatcher =
  | string
  | {
      before?: string;
      after?: string;
      from?: string;
      to?: string;
      dayOfWeek?: number[];
    };
export type DateDisabledProp = boolean | DateDisabledMatcher | DateDisabledMatcher[];

export interface DateChangePayload {
  mode: DateSelectionMode;
  value: DatePickerValue;
}

export interface DatePickerProps extends ComponentBaseProps {
  mode?: DateSelectionMode;
  value?: DatePickerValue;
  numberOfMonths?: number;
  size?: ComponentSize;
  disabled?: DateDisabledProp;
  placeholder?: JsonValue;
  cleanable?: boolean;
  appearance?: boolean;
  onChange?: RasterEventHandler<DateChangePayload>;
}

export type DialogOpenChangeReason = "ok" | "cancel" | "controlled";

export interface DialogOpenChangePayload {
  open: boolean;
  reason: DialogOpenChangeReason;
}

export interface DialogProps extends ComponentBaseProps {
  open?: boolean;
  title?: JsonValue;
  confirm?: boolean;
  okText?: JsonValue;
  cancelText?: JsonValue;
  width?: number;
  maxWidth?: number;
  marginTop?: number;
  overlay?: boolean;
  overlayClosable?: boolean;
  keyboard?: boolean;
  closeButton?: boolean;
  onOk?: RasterEventHandler<string>;
  onCancel?: RasterEventHandler<string>;
  onOpenChange?: RasterEventHandler<DialogOpenChangePayload>;
}

export interface FormProps extends ComponentBaseProps {
  layout?: ComponentAxis;
  axis?: ComponentAxis;
  size?: ComponentSize;
  columns?: number;
  labelWidth?: number;
  labelTextSize?: number;
}

export interface FieldValidateResult {
  error: boolean;
  message: string;
}

export type FieldValidateHandler = (value: JsonValue) => FieldValidateResult;

export interface FieldProps extends ComponentBaseProps {
  value?: JsonValue;
  validate?: FieldValidateHandler;
  validateDebounce?: number;
  label?: JsonValue;
  description?: JsonValue;
  required?: boolean;
  visible?: boolean;
  labelIndent?: boolean;
  align?: FieldAlign;
  colSpan?: number;
  colStart?: number;
  colEnd?: number;
}

export interface IconProps extends ComponentBaseProps {
  name?: IconName;
  icon?: IconName;
  path?: string;
  empty?: boolean;
  rotate?: number;
  size?: ComponentSize;
  color?: string;
}

export interface RadioProps extends ComponentBaseProps {
  label?: JsonValue;
  size?: ComponentSize;
  checked?: boolean;
  selected?: boolean;
  disabled?: boolean;
  tabIndex?: number;
  tabStop?: boolean;
  onChange?: RasterEventHandler<boolean | string>;
  onClick?: RasterEventHandler<boolean | string>;
}

export interface RadioGroupProps extends ComponentBaseProps {
  layout?: ComponentAxis;
  axis?: ComponentAxis;
  selectedIndex?: number;
  disabled?: boolean;
  size?: ComponentSize;
  onChange?: RasterEventHandler<string>;
  onClick?: RasterEventHandler<string>;
}

export type CollectionItemId = string | number;

export interface CollectionItem {
  id?: CollectionItemId;
  label?: JsonValue;
  description?: JsonValue;
  icon?: IconName;
  disabled?: boolean;
  value?: JsonValue;
  badge?: JsonValue;
  checked?: boolean;
}

export interface CollectionSection<TItem extends CollectionItem = CollectionItem> {
  id?: CollectionItemId;
  label?: JsonValue;
  items: TItem[];
}

export type MenuAnchor = "topLeft" | "topRight" | "bottomLeft" | "bottomRight";

export interface SelectOption extends CollectionItem {}

export interface SelectOpenChangePayload {
  open: boolean;
  reason: "trigger" | "outside" | "escape" | "select" | "clear";
}

export interface SelectChangePayload {
  value?: JsonValue;
  id?: CollectionItemId;
  label?: JsonValue;
}

export interface SelectProps extends ComponentBaseProps {
  options?: SelectOption[];
  sections?: CollectionSection<SelectOption>[];
  value?: JsonValue;
  placeholder?: JsonValue;
  searchable?: boolean;
  cleanable?: boolean;
  disabled?: boolean;
  size?: ComponentSize;
  anchor?: MenuAnchor;
  onChange?: RasterEventHandler<SelectChangePayload>;
  onOpenChange?: RasterEventHandler<SelectOpenChangePayload>;
  onSearchChange?: RasterEventHandler<string>;
}

export type SheetPlacement = "top" | "right" | "bottom" | "left";
export type SheetOpenChangeReason = "close-button" | "escape" | "overlay" | "controlled";

export interface SheetOpenChangePayload {
  open: boolean;
  reason: SheetOpenChangeReason;
}

export interface SheetProps extends ComponentBaseProps {
  open?: boolean;
  title?: JsonValue;
  placement?: SheetPlacement;
  size?: number;
  overlay?: boolean;
  overlayClosable?: boolean;
  resizable?: boolean;
  onOpenChange?: RasterEventHandler<SheetOpenChangePayload>;
}

export interface SwitchProps extends ComponentBaseProps {
  label?: JsonValue;
  tooltip?: string;
  size?: ComponentSize;
  checked?: boolean;
  disabled?: boolean;
  onChange?: RasterEventHandler<boolean | string>;
  onClick?: RasterEventHandler<boolean | string>;
}

export interface TabProps extends ComponentBaseProps {
  label?: JsonValue;
  icon?: IconName;
  variant?: TabVariant;
  size?: ComponentSize;
  disabled?: boolean;
  selected?: boolean;
  onClick?: RasterEventHandler<string>;
}

export interface TabBarProps extends ComponentBaseProps {
  variant?: TabVariant;
  size?: ComponentSize;
  selectedIndex?: number;
  menu?: boolean;
  onClick?: RasterEventHandler<string>;
}

export interface VirtualListRangePayload {
  start: number;
  end: number;
}

export interface VirtualListRenderItemPayload<TItem extends CollectionItem = CollectionItem> {
  item: TItem;
  index: number;
}

export interface VirtualListProps extends ComponentBaseProps {
  items?: CollectionItem[];
  axis?: ComponentAxis;
  itemSize?: number;
  renderItem?: (payload: VirtualListRenderItemPayload) => ReactElement | null;
  keyExtractor?: (item: CollectionItem, index: number) => string | number;
  onVisibleRangeChange?: RasterEventHandler<VirtualListRangePayload>;
}

function isEventProp(key: string): boolean {
  return /^on[A-Z]/.test(key);
}

function isQueryProp(key: string): boolean {
  return /^get[A-Z]/.test(key);
}

function isComponentName(name: string): name is ComponentName {
  return (componentNames as readonly string[]).includes(name);
}

function splitComponentProps(input: ComponentBaseProps = {}) {
  const typedInput = input as ComponentBaseProps & Record<string, unknown>;
  const { children, style, ...rest } = typedInput;
  const props: JsonObject = {};
  const events: Record<string, RasterEventHandler> = {};
  const queries: Record<string, RasterQueryHandler> = {};

  for (const [key, value] of Object.entries(rest)) {
    if (isEventProp(key) && typeof value === "function") {
      events[key] = value as RasterEventHandler;
    } else if (isQueryProp(key) && typeof value === "function") {
      queries[key] = value as RasterQueryHandler;
    } else {
      props[key] = value as JsonValue;
    }
  }

  return { props, style, children, events, queries };
}

export function createComponent<P extends ComponentBaseProps = GenericComponentProps>(
  name: ComponentName
): ComponentType<P>;
export function createComponent<P extends ComponentBaseProps = GenericComponentProps>(
  name: string
): ComponentType<P>;
export function createComponent<P extends ComponentBaseProps = GenericComponentProps>(
  name: string
): ComponentType<P> {
  if (!isComponentName(name)) {
    throw new Error(`Unknown raster component: ${name}`);
  }

  function RasterComponent(input: P): ReactElement {
    const { props, style, children, events, queries } = splitComponentProps(input);
    return jsx(Widget, {
      name,
      props,
      queries,
      style,
      children,
      ...events,
    });
  }

  RasterComponent.displayName = name;
  return RasterComponent;
}

export const Avatar = createComponent<AvatarProps>("Avatar");
export const AvatarGroup = createComponent<AvatarGroupProps>("AvatarGroup");
export const Alert = createComponent<AlertProps>("Alert");
export const Button = createComponent<ButtonProps>("Button");
export const ButtonGroup = createComponent<ButtonGroupProps>("ButtonGroup");
export const Checkbox = createComponent<CheckboxProps>("Checkbox");
export const ColorPicker = createComponent<ColorPickerProps>("ColorPicker");
export const DatePicker = createComponent<DatePickerProps>("DatePicker");
export const Dialog = createComponent<DialogProps>("Dialog");

type RasterNativeHandleRef = {
  handle?: {
    surface_id: number;
    node_tag: number;
  };
};

function chartNativeHandle(hostRef: MutableRefObject<RasterNativeHandleRef | null>) {
  const handle = hostRef.current?.handle;
  if (handle == null) {
    throw new Error("Chart ref command requires a mounted chart component");
  }
  return handle;
}

function chartRows(rowOrRows: ChartDatum | ChartDatum[]): ChartDatum[] {
  return Array.isArray(rowOrRows) ? rowOrRows : [rowOrRows];
}

function createChartComponent<P extends ChartBaseProps>(name: ComponentName) {
  const ChartComponent = forwardRef<ChartRef, P>((input, ref): ReactElement => {
    const hostRef = useRef<RasterNativeHandleRef | null>(null);

    useImperativeHandle(
      ref,
      () => ({
        appendData(rowOrRows) {
          const binding = nativeBinding();
          if (binding.chartAppendData == null) {
            throw new Error("Raster chart appendData is not available in this runtime");
          }
          binding.chartAppendData(chartNativeHandle(hostRef), chartRows(rowOrRows));
        },
        replaceData(rows) {
          const binding = nativeBinding();
          if (binding.chartReplaceData == null) {
            throw new Error("Raster chart replaceData is not available in this runtime");
          }
          binding.chartReplaceData(chartNativeHandle(hostRef), rows);
        },
        clearData() {
          const binding = nativeBinding();
          if (binding.chartClearData == null) {
            throw new Error("Raster chart clearData is not available in this runtime");
          }
          binding.chartClearData(chartNativeHandle(hostRef));
        },
      }),
      []
    );

    const { props, style, children, events, queries } = splitComponentProps(input);
    return jsx("Widget", {
      name,
      props,
      queries,
      style,
      children,
      ...events,
      ref: hostRef,
    });
  });
  ChartComponent.displayName = name;
  return ChartComponent;
}

export const LineChart = createChartComponent<LineChartProps>("LineChart");
export const BarChart = createChartComponent<BarChartProps>("BarChart");
export const AreaChart = createChartComponent<AreaChartProps>("AreaChart");
export const PieChart = createChartComponent<PieChartProps>("PieChart");
export const CandlestickChart =
  createChartComponent<CandlestickChartProps>("CandlestickChart");
const FieldHost = createComponent<Omit<FieldProps, "validate" | "validateDebounce">>("Field");
export function Field({
  validate,
  validateDebounce = 300,
  value = null,
  description,
  ...input
}: FieldProps): ReactElement {
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const generation = useRef(0);

  useEffect(() => {
    generation.current += 1;
    const currentGeneration = generation.current;
    if (validate == null) {
      setErrorMessage(null);
      return;
    }

    const delay = Math.max(0, validateDebounce);
    const timeout = setTimeout(() => {
      const result = validate(value);
      if (generation.current !== currentGeneration) {
        return;
      }
      setErrorMessage(result.error ? result.message : null);
      (globalThis as RasterRuntimeGlobal).__rasterFlushSyncWork?.();
    }, delay);

    return () => {
      clearTimeout(timeout);
    };
  }, [validate, validateDebounce, value]);

  const resolvedDescription = errorMessage ?? description;
  const hostProps =
    resolvedDescription === undefined
      ? { ...input, value, __validationError: errorMessage != null }
      : {
          ...input,
          value,
          description: resolvedDescription,
          __validationError: errorMessage != null,
        };

  return jsx(FieldHost, {
    ...hostProps,
  });
}
export const Form = createComponent<FormProps>("Form");
export const Icon = createComponent<IconProps>("Icon");
export const Radio = createComponent<RadioProps>("Radio");
export const RadioGroup = createComponent<RadioGroupProps>("RadioGroup");
export const Select = createComponent<SelectProps>("Select");
export const Sheet = createComponent<SheetProps>("Sheet");
export const Switch = createComponent<SwitchProps>("Switch");
export const Tab = createComponent<TabProps>("Tab");
export const TabBar = createComponent<TabBarProps>("TabBar");

const VirtualListHost = createComponent<VirtualListProps>("VirtualList");
export function VirtualList({
  items,
  renderItem,
  keyExtractor,
  children,
  ...input
}: VirtualListProps): ReactElement {
  const renderedChildren =
    renderItem == null || items == null
      ? children
      : items.map((item, index) => {
          const key = keyExtractor?.(item, index) ?? item.id ?? item.value ?? index;
          return jsx(
            View,
            {
              children: renderItem({ item, index }),
            },
            String(key)
          );
        });

  return jsx(VirtualListHost, {
    ...input,
    children: renderedChildren,
  });
}
