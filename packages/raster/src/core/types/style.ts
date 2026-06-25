export interface EdgeInsets {
  top?: number;
  right?: number;
  bottom?: number;
  left?: number;
}

export type StyleDimension = number | `${number}%`;
export type FontWeight = "normal" | "bold" | number | `${number}`;
export type RasterStyleInput = RasterStyle | ReadonlyArray<RasterStyle | null | undefined> | null;
export type DisplayValue = "flex" | "block" | "none";
export type FlexDirectionValue = "row" | "column" | "row-reverse" | "column-reverse";
export type FlexWrapValue = "nowrap" | "wrap" | "wrap-reverse";
export type JustifyContentValue =
  | "flex-start"
  | "flex-end"
  | "start"
  | "end"
  | "center"
  | "space-between"
  | "space-around"
  | "space-evenly";
export type AlignValue =
  | "stretch"
  | "flex-start"
  | "flex-end"
  | "start"
  | "end"
  | "center"
  | "baseline";
export type AlignContentValue =
  | "stretch"
  | "flex-start"
  | "flex-end"
  | "start"
  | "end"
  | "center"
  | "space-between"
  | "space-around"
  | "space-evenly";
export type PositionValue = "relative" | "absolute";
// GPUI does not expose CSS-style overflow auto; Raster maps auto to scroll.
export type OverflowValue = "visible" | "hidden" | "clip" | "scroll" | "auto";

export type BoxShadowPreset = "none" | "xs" | "sm" | "md" | "lg" | "xl";

export interface BoxShadowValue {
  offsetX?: number;
  offsetY?: number;
  blurRadius?: number;
  spreadRadius?: number;
  color?: string;
}

export type BoxShadowInput =
  | BoxShadowPreset
  | string
  | BoxShadowValue
  | ReadonlyArray<BoxShadowInput>;

export interface RasterStyle {
  display?: DisplayValue;
  flexDirection?: FlexDirectionValue;
  flexWrap?: FlexWrapValue;
  justifyContent?: JustifyContentValue;
  alignItems?: AlignValue;
  alignSelf?: AlignValue;
  alignContent?: AlignContentValue;
  width?: StyleDimension;
  height?: StyleDimension;
  minWidth?: StyleDimension;
  minHeight?: StyleDimension;
  maxWidth?: StyleDimension;
  maxHeight?: StyleDimension;
  aspectRatio?: number;
  flex?: number;
  flexGrow?: number;
  flexShrink?: number;
  flexBasis?: StyleDimension;
  gap?: number;
  rowGap?: number;
  columnGap?: number;
  position?: PositionValue;
  top?: StyleDimension;
  right?: StyleDimension;
  bottom?: StyleDimension;
  left?: StyleDimension;
  overflow?: OverflowValue;
  overflowX?: OverflowValue;
  overflowY?: OverflowValue;
  padding?: number | EdgeInsets;
  margin?: number | EdgeInsets;
  backgroundColor?: string;
  borderWidth?: number;
  borderTopWidth?: number;
  borderRightWidth?: number;
  borderBottomWidth?: number;
  borderLeftWidth?: number;
  borderColor?: string;
  borderRadius?: number;
  borderTopLeftRadius?: number;
  borderTopRightRadius?: number;
  borderBottomRightRadius?: number;
  borderBottomLeftRadius?: number;
  opacity?: number;
  color?: string;
  fontSize?: number;
  fontWeight?: FontWeight;
  fontStyle?: "normal" | "italic";
  textDecorationLine?: "none" | "underline";
  boxShadow?: BoxShadowInput;
  backdropBlur?: number;
}

