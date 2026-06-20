import type { ComponentType } from "react";
import type { InputProps, LabelProps, SlotProps, TextareaProps, TextProps, ViewProps } from "../types/index.js";

type HostPrimitive<Props> = ComponentType<Props>;

export const Label = "Label" as unknown as HostPrimitive<LabelProps>;
export const Input = "Input" as unknown as HostPrimitive<InputProps>;
export const Slot = "Slot" as unknown as HostPrimitive<SlotProps>;
export const Text = "Label" as unknown as HostPrimitive<TextProps>;
export const Textarea = "Textarea" as unknown as HostPrimitive<TextareaProps>;
export const View = "View" as unknown as HostPrimitive<ViewProps>;

