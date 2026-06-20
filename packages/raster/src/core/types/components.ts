import type { ReactElement, ReactNode } from "react";
import type { JsonObject, JsonValue } from "./json.js";
import type { RasterEventHandler, RasterQueryHandler } from "./events.js";
import type { RasterStyleInput } from "./style.js";
import type { RasterThemeConfig } from "./theme.js";

export interface RasterRoot {
  render(element: ReactElement | null): void;
  clear(): void;
}

export interface RasterNodeProps {
  key?: string | number;
  style?: RasterStyleInput;
  children?: ReactNode;
}

export interface ViewProps extends RasterNodeProps {
  onClick?: RasterEventHandler<string>;
}

export type LabelHighlightMode = "full" | "prefix";

export interface LabelHighlightSpec {
  text: string;
  mode?: LabelHighlightMode;
}

export interface LabelProps extends RasterNodeProps {
  secondary?: JsonValue;
  masked?: boolean;
  highlights?: string | LabelHighlightSpec;
  selectable?: boolean;
}

export interface TextProps extends LabelProps {}

export interface SlotProps {
  name: string;
  children?: ReactNode;
}

export interface TextControlAutoGrow {
  minRows: number;
  maxRows: number;
}

export interface TextControlNumberMaskPattern {
  kind: "number";
  separator?: string | null;
}

export type TextControlMaskPattern = string | TextControlNumberMaskPattern;

export interface TextChangePayload {
  value: string;
  eventCount: number;
}

export interface TextControlProps extends RasterNodeProps {
  value?: string | number | null;
  defaultValue?: string | number | null;
  placeholder?: string | number | null;
  editable?: boolean;
  readOnly?: boolean;
  secureTextEntry?: boolean;
  maxLength?: number;
  size?: ComponentSize;
  selected?: boolean;
  appearance?: boolean;
  bordered?: boolean;
  focusBordered?: boolean;
  cleanable?: boolean;
  maskToggle?: boolean;
  tabIndex?: number;
  loading?: boolean;
  multiline?: boolean;
  rows?: number;
  autoGrow?: TextControlAutoGrow;
  codeEditor?: string;
  searchable?: boolean;
  lineNumber?: boolean;
  cleanOnEscape?: boolean;
  softWrap?: boolean;
  pattern?: string;
  maskPattern?: TextControlMaskPattern;
  validate?: RasterQueryHandler<string, boolean>;
  onBlur?: RasterEventHandler<string>;
  onChange?: RasterEventHandler<TextChangePayload>;
  onChangeText?: RasterEventHandler<string>;
  onEndEditing?: RasterEventHandler<string>;
  onFocus?: RasterEventHandler<string>;
  onSubmitEditing?: RasterEventHandler<string>;
}

export interface InputProps extends TextControlProps {}
export interface TextareaProps extends TextControlProps {}


export interface ConfigProviderProps {
  theme?: RasterThemeConfig;
  text?: JsonObject;
  resources?: JsonObject;
  children?: ReactNode;
  [eventName: `on${string}`]: RasterEventHandler<any> | undefined;
}

export interface WidgetProps {
  name: string;
  props?: JsonObject;
  queries?: Record<string, RasterQueryHandler<any, any>>;
  style?: RasterStyleInput;
  children?: ReactNode;
  [eventName: `on${string}`]: RasterEventHandler<any> | undefined;
}

export type ComponentSize = "xs" | "xsmall" | "sm" | "small" | "md" | "medium" | "lg" | "large" | number;
export type ComponentAxis = "horizontal" | "vertical";
export type FieldAlign = "start" | "center" | "end";
export type TabVariant = "tab" | "outline" | "pill" | "segmented" | "underline";
export type DescriptionListItem = { label: JsonValue; value?: JsonValue; span?: number } | "divider";
export type ComponentEventProps = {
  [eventName: `on${string}`]: RasterEventHandler<any> | undefined;
};

export type ComponentQueryProps = {
  [queryName: `get${string}`]: RasterQueryHandler<any, any> | undefined;
};

export interface ComponentBaseProps extends ComponentEventProps, ComponentQueryProps {
  style?: RasterStyleInput;
  children?: ReactNode;
}

export interface GenericComponentProps extends ComponentBaseProps {
  [key: string]: JsonValue | ReactNode | RasterStyleInput | RasterEventHandler<any> | undefined;
}

export type ButtonVariant =
  | "primary"
  | "secondary"
  | "danger"
  | "error"
  | "info"
  | "success"
  | "warning"
  | "ghost"
  | "link"
  | "text"
  | "custom";

export type ButtonRounded = "none" | "small" | "sm" | "medium" | "md" | "large" | "lg" | number | boolean;
export type ToggleVariant = "ghost" | "outline";

export interface ButtonCustomVariant {
  color?: string;
  foreground?: string;
  border?: string;
  hover?: string;
  active?: string;
  shadow?: boolean;
}

export type AlertVariant = "secondary" | "info" | "success" | "warning" | "error" | "danger";
export type GroupBoxVariant = "normal" | "fill" | "outline";
export type TagVariant = "primary" | "secondary" | "danger" | "error" | "success" | "warning" | "info";
export type TagColorName =
  | "gray"
  | "red"
  | "orange"
  | "amber"
  | "yellow"
  | "lime"
  | "green"
  | "emerald"
  | "teal"
  | "cyan"
  | "sky"
  | "blue"
  | "indigo"
  | "violet"
  | "purple"
  | "fuchsia"
  | "pink"
  | "rose";
export interface TagCustomVariant {
  color: string;
  foreground: string;
  border: string;
}
export type DividerOrientation = "horizontal" | "vertical";
export type IconName =
  | "a-large-small"
  | "alert"
  | "triangle-alert"
  | "warning"
  | "arrow-down"
  | "arrow-left"
  | "arrow-right"
  | "arrow-up"
  | "asterisk"
  | "bell"
  | "book-open"
  | "bot"
  | "building"
  | "building-2"
  | "calendar"
  | "case-sensitive"
  | "chart-pie"
  | "check"
  | "chevron-down"
  | "chevron-left"
  | "chevron-right"
  | "chevron-up"
  | "chevrons-up-down"
  | "circle-check"
  | "success"
  | "circle-user"
  | "circle-x"
  | "error"
  | "close"
  | "x"
  | "copy"
  | "dash"
  | "delete"
  | "ellipsis"
  | "ellipsis-vertical"
  | "external-link"
  | "eye"
  | "eye-off"
  | "file"
  | "folder"
  | "folder-closed"
  | "folder-open"
  | "frame"
  | "gallery-vertical-end"
  | "github"
  | "globe"
  | "heart"
  | "heart-off"
  | "inbox"
  | "info"
  | "inspector"
  | "layout-dashboard"
  | "loader"
  | "loader-circle"
  | "map"
  | "maximize"
  | "menu"
  | "minimize"
  | "minus"
  | "moon"
  | "palette"
  | "panel-bottom"
  | "panel-bottom-open"
  | "panel-left"
  | "panel-left-close"
  | "panel-left-open"
  | "panel-right"
  | "panel-right-close"
  | "panel-right-open"
  | "plus"
  | "redo"
  | "redo-2"
  | "replace"
  | "resize-corner"
  | "search"
  | "settings"
  | "settings-2"
  | "sort-ascending"
  | "sort-descending"
  | "square-terminal"
  | "star"
  | "star-off"
  | "sun"
  | "thumbs-down"
  | "thumbs-up"
  | "undo"
  | "undo-2"
  | "user"
  | "window-close"
  | "window-maximize"
  | "window-minimize"
  | "window-restore";

