import type {
  ConfigProviderProps,
  InputProps,
  LabelProps,
  SlotProps,
  TextProps,
  TextareaProps,
  ViewProps,
  WidgetProps,
} from "./types/index.js";

declare module "react" {
  namespace JSX {
    interface IntrinsicElements {
      View: ViewProps;
      Label: LabelProps;
      Text: TextProps;
      Slot: SlotProps;
      Input: InputProps;
      Textarea: TextareaProps;
      ConfigProvider: ConfigProviderProps;
      Widget: WidgetProps;
    }
  }
}