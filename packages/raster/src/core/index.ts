/// <reference path="./jsx.d.ts" />

import type { ReactElement } from "react";
import { jsx } from "react/jsx-runtime";

import type { ComponentType } from "react";

import type {
  ConfigProviderProps,
  InputProps,
  LabelProps,
  SlotProps,
  TextareaProps,
  TextProps,
  ViewProps,
  WidgetProps,
} from "./types.js";

type HostPrimitive<Props> = ComponentType<Props>;

export const Label = "Label" as unknown as HostPrimitive<LabelProps>;
export const Input = "Input" as unknown as HostPrimitive<InputProps>;
export const Slot = "Slot" as unknown as HostPrimitive<SlotProps>;
export const Text = "Label" as unknown as HostPrimitive<TextProps>;
export const Textarea = "Textarea" as unknown as HostPrimitive<TextareaProps>;
export const View = "View" as unknown as HostPrimitive<ViewProps>;

export type {
  ConfigProviderProps,
  ComponentAxis,
  DescriptionListItem,
  FieldAlign,
  InputProps,
  JsonObject,
  JsonPrimitive,
  JsonValue,
  LabelProps,
  HandlerSlotId,
  NativeChildSetHandle,
  NativeNodeHandle,
  RasterEventHandler,
  RasterHandlerSlotId,
  RasterHandlerSlotKind,
  RasterNativeBinding,
  RasterNativeChildSet,
  RasterNativeChildSetHandle,
  RasterNativeChildSetId,
  RasterNativeEventBinding,
  RasterNativeJsFunctionRef,
  RasterNativeMaterializeResult,
  RasterNativeMaterializeSpec,
  RasterNativeNode,
  RasterNativeNodeHandle,
  RasterNativeNodeKind,
  RasterNativeQueryBinding,
  RasterNodeTag,
  RasterQueryHandler,
  RasterRoot,
  RasterRootOptions,
  RasterThemeColors,
  RasterThemeConfig,
  RasterThemeMode,
  RasterShadowNodePayload,
  RasterShadowNodeUpdatePayload,
  RasterShadowRevisionId,
  RasterSurfaceGeneration,
  RasterSurfaceId,
  SlotProps,
  TabVariant,
  TextProps,
  TextareaProps,
  TextChangePayload,
  ViewProps,
  WidgetProps,
} from "./types.js";
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

export function ConfigProvider({
  theme = {},
  text = {},
  resources = {},
  children,
  ...events
}: ConfigProviderProps): ReactElement {
  return jsx("ConfigProvider", {
    theme,
    text,
    resources,
    children,
    ...events,
  });
}

export function Widget({ name, props = {}, queries = {}, style, children, ...events }: WidgetProps): ReactElement {
  return jsx("Widget", {
    name,
    props,
    queries,
    style,
    children,
    ...events,
  });
}
