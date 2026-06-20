export { createRoot, createRoot as createRasterRoot } from "./react/index.js";
export { ConfigProvider, Input, Label, Slot, Text, Textarea, View, Widget } from "./core/index.js";
import ReactDefault, * as React from "react";

const ReactRuntime = React as typeof React & Record<string, unknown>;
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
  RasterEventHandler,
  RasterQueryHandler,
  RasterRoot,
  RasterRootOptions,
  RasterThemeColors,
  RasterThemeConfig,
  RasterThemeMode,
  SlotProps,
  TabVariant,
  TextProps,
  TextareaProps,
  TextChangePayload,
  ViewProps,
  WidgetProps,
} from "./core/types/index.js";
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
} from "./core/types/style.js";
export * from "./component/index.js";
export { jsx, jsxs } from "react/jsx-runtime";

export const Activity = React.Activity;
export const Children = React.Children;
export const Component = React.Component;
export const Fragment = React.Fragment;
export const Profiler = React.Profiler;
export const PureComponent = React.PureComponent;
export const StrictMode = React.StrictMode;
export const Suspense = React.Suspense;
export const __CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE =
  ReactRuntime.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE;
export const __COMPILER_RUNTIME = ReactRuntime.__COMPILER_RUNTIME;
export const act = React.act;
export const cache = React.cache;
export const cacheSignal = React.cacheSignal;
export const captureOwnerStack = React.captureOwnerStack;
export const cloneElement = React.cloneElement;
export const createContext = React.createContext;
export const createElement = React.createElement;
export const createRef = React.createRef;
export const forwardRef = React.forwardRef;
export const isValidElement = React.isValidElement;
export const lazy = React.lazy;
export const memo = React.memo;
export const startTransition = React.startTransition;
export const unstable_useCacheRefresh = ReactRuntime.unstable_useCacheRefresh;
export const use = React.use;
export const useActionState = React.useActionState;
export const useCallback = React.useCallback;
export const useContext = React.useContext;
export const useDebugValue = React.useDebugValue;
export const useDeferredValue = React.useDeferredValue;
export const useEffect = React.useEffect;
export const useEffectEvent = React.useEffectEvent;
export const useId = React.useId;
export const useImperativeHandle = React.useImperativeHandle;
export const useInsertionEffect = React.useInsertionEffect;
export const useLayoutEffect = React.useLayoutEffect;
export const useMemo = React.useMemo;
export const useOptimistic = React.useOptimistic;
export const useReducer = React.useReducer;
export const useRef = React.useRef;
export const useState = React.useState;
export const useSyncExternalStore = React.useSyncExternalStore;
export const useTransition = React.useTransition;
export const version = React.version;
export default ReactDefault;
