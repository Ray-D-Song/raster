import type { ReactElement, ReactNode } from "react";
import type { RasterStyleInput } from "./style.js";

export type {
  AlignContentValue,
  AlignValue,
  DisplayValue,
  EdgeInsets,
  FlexDirectionValue,
  FlexWrapValue,
  FontWeight,
  JustifyContentValue,
  OverflowValue,
  PositionValue,
  RasterStyle,
  RasterStyleInput,
  StyleDimension,
} from "./style.js";

export type JsonPrimitive = null | boolean | number | string;
export type JsonValue = JsonPrimitive | JsonValue[] | JsonObject;
export type JsonObject = { [key: string]: JsonValue };

export type RasterEventHandler<T = unknown> = (payload: T) => void;
export type RasterQueryHandler<TPayload = unknown, TResult = unknown> = (payload: TPayload) => TResult;

export interface RasterRootOptions {
  width?: number | null;
  height?: number | null;
  perfdetect?: boolean | null;
}

export type RasterSurfaceId = number;
export type RasterNodeTag = number;
export type RasterShadowRevisionId = number;
export type RasterSurfaceGeneration = number;
export type RasterHandlerSlotId = number;
export type RasterNativeChildSetId = number;

export type RasterNativeNodeKind =
  | "host"
  | "text"
  | "input"
  | "textarea"
  | "widget"
  | "fragment"
  | "slot"
  | "config_provider";

export type RasterHandlerSlotKind = "event" | "query";

export interface RasterNativeNodeHandle {
  surface_id: RasterSurfaceId;
  node_tag: RasterNodeTag;
  revision_id: RasterShadowRevisionId;
  generation: RasterSurfaceGeneration;
}

export interface RasterNativeChildSetHandle {
  surface_id: RasterSurfaceId;
  child_set_id: RasterNativeChildSetId;
  generation: RasterSurfaceGeneration;
}

export interface RasterNativeNode {
  $$typeof: "raster.native-node";
  kind: RasterNativeNodeKind;
  tag: RasterNodeTag;
  handle: RasterNativeNodeHandle;
  debug?: {
    name?: string;
    key?: string;
    componentStack?: string;
  };
}

export interface RasterNativeChildSet {
  $$typeof: "raster.native-child-set";
  surfaceId: RasterSurfaceId;
  handle: RasterNativeChildSetHandle;
}

export interface RasterNativeEventBinding {
  property: string;
  event_type: string | null;
  options?: JsonObject;
  handler_slot_id: RasterHandlerSlotId;
}

export interface RasterNativeQueryBinding {
  property: string;
  query_type: string | null;
  options?: JsonObject;
  handler_slot_id: RasterHandlerSlotId;
}

export interface RasterShadowNodePayload {
  props?: JsonObject;
  style?: JsonValue;
  text?: string;
  hidden?: boolean;
  context?: JsonObject;
  event_bindings?: RasterNativeEventBinding[];
  query_bindings?: RasterNativeQueryBinding[];
}

export type RasterShadowNodeUpdatePayload = RasterShadowNodePayload;

export interface RasterNativeMaterializeSpec {
  handle?: RasterNativeNodeHandle;
  kind?: RasterNativeNodeKind;
  name?: string;
  key?: string | null;
  text?: string;
  payload?: RasterShadowNodeUpdatePayload;
  children?: RasterNativeMaterializeSpec[];
  childHandles?: RasterNativeNodeHandle[];
  childUpdates?: RasterNativeMaterializeChildUpdate[];
}

export interface RasterNativeMaterializeChildUpdate {
  index: number;
  child: RasterNativeMaterializeSpec;
}

export interface RasterNativeMaterializeResult {
  handle: RasterNativeNodeHandle;
  children: RasterNativeMaterializeResult[];
  childUpdates?: RasterNativeMaterializeChildResult[];
}

export interface RasterNativeMaterializeChildResult {
  index: number;
  child: RasterNativeMaterializeResult;
}

export type RasterNativeJsFunctionRef = (payload: unknown) => unknown;

export interface RasterNativeBinding {
  createSurface(options?: RasterRootOptions): RasterSurfaceId;
  createNode(
    surfaceId: RasterSurfaceId,
    kind: RasterNativeNodeKind,
    name: string,
    key: string | null,
    payload: RasterShadowNodePayload
  ): RasterNativeNodeHandle;
  createTextNode(
    surfaceId: RasterSurfaceId,
    text: string,
    payload: RasterShadowNodePayload
  ): RasterNativeNodeHandle;
  appendInitialChild(parent: RasterNativeNodeHandle, child: RasterNativeNodeHandle): void;
  prepareForCommit(surfaceId: RasterSurfaceId): void;
  resetAfterCommit(surfaceId: RasterSurfaceId): void;
  clearSurface?(surfaceId: RasterSurfaceId): void;
  appendChild(parent: RasterNativeNodeHandle, child: RasterNativeNodeHandle): void;
  appendChildToContainer(surfaceId: RasterSurfaceId, child: RasterNativeNodeHandle): void;
  insertBefore(
    parent: RasterNativeNodeHandle,
    child: RasterNativeNodeHandle,
    before: RasterNativeNodeHandle
  ): void;
  insertInContainerBefore(
    surfaceId: RasterSurfaceId,
    child: RasterNativeNodeHandle,
    before: RasterNativeNodeHandle
  ): void;
  removeChild(parent: RasterNativeNodeHandle, child: RasterNativeNodeHandle): void;
  removeChildFromContainer(surfaceId: RasterSurfaceId, child: RasterNativeNodeHandle): void;
  updateNode(handle: RasterNativeNodeHandle, payload: RasterShadowNodeUpdatePayload): void;
  updateTextNode(handle: RasterNativeNodeHandle, text: string): void;
  cloneNode(
    handle: RasterNativeNodeHandle,
    payload: RasterShadowNodeUpdatePayload
  ): RasterNativeNodeHandle;
  cloneNodeWithChildren(
    handle: RasterNativeNodeHandle,
    payload: RasterShadowNodeUpdatePayload,
    children: RasterNativeNodeHandle[]
  ): RasterNativeNodeHandle;
  createChildSet(surfaceId: RasterSurfaceId): RasterNativeChildSetHandle;
  appendChildToSet(childSet: RasterNativeChildSetHandle, childHandle: RasterNativeNodeHandle): void;
  finalizeChildSet(childSet: RasterNativeChildSetHandle): void;
  commitChildSet(surfaceId: RasterSurfaceId, childSet: RasterNativeChildSetHandle): void;
  commitSurfaceTree(
    surfaceId: RasterSurfaceId,
    roots: RasterNativeMaterializeSpec[]
  ): RasterNativeMaterializeResult[];
  deleteNode(handle: RasterNativeNodeHandle): void;
  registerHandlerSlot(
    surfaceId: RasterSurfaceId,
    nodeTag: RasterNodeTag,
    kind: RasterHandlerSlotKind,
    property: string,
    eventOrQueryType: string | null
  ): RasterHandlerSlotId;
  updateHandlerSlot(handlerSlotId: RasterHandlerSlotId, jsFunctionRef: RasterNativeJsFunctionRef): void;
  dropHandlerSlotsForNode(surfaceId: RasterSurfaceId, nodeTag: RasterNodeTag): void;
  notificationShow?(options: RasterNotificationShowOptions): void;
  notificationDismiss?(id: string): void;
  notificationClear?(): void;
  chartAppendData?(handle: RasterNativeNodeHandle, rows: JsonValue[]): void;
  chartReplaceData?(handle: RasterNativeNodeHandle, rows: JsonValue[]): void;
  chartClearData?(handle: RasterNativeNodeHandle): void;
}

export type RasterNotificationType = "info" | "success" | "warning" | "error";

export interface RasterNotificationShowOptions {
  id?: string;
  type?: RasterNotificationType;
  title?: string;
  message: string;
  autohide?: boolean;
}

export type NativeNodeHandle = RasterNativeNodeHandle;
export type NativeChildSetHandle = RasterNativeChildSetHandle;
export type HandlerSlotId = RasterHandlerSlotId;

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

export type RasterThemeMode = "light" | "dark" | "system";

export interface RasterThemeColors {
  background?: string;
  foreground?: string;
  border?: string;
  input?: string;
  primary?: string;
  primaryForeground?: string;
  secondary?: string;
  secondaryForeground?: string;
  accent?: string;
  accentForeground?: string;
  muted?: string;
  mutedForeground?: string;
  popover?: string;
  popoverForeground?: string;
  ring?: string;
  danger?: string;
  success?: string;
  warning?: string;
  info?: string;
}

export interface RasterThemeConfig {
  mode?: RasterThemeMode;
  radius?: number;
  radiusLg?: number;
  fontSize?: number;
  fontFamily?: string;
  monoFontSize?: number;
  monoFontFamily?: string;
  colors?: RasterThemeColors;
}

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
