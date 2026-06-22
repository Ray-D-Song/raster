export type RasterThemeMode = "light" | "dark" | "system";

export enum ThemePreset {
  Adventure = "Adventure",
  AdventureTime = "Adventure Time",
  Alduin = "Alduin",
  Asciinema = "Asciinema",
  AyuLight = "Ayu Light",
  AyuDark = "Ayu Dark",
  CatppuccinLatte = "Catppuccin Latte",
  CatppuccinFrappe = "Catppuccin Frappe",
  CatppuccinMacchiato = "Catppuccin Macchiato",
  CatppuccinMocha = "Catppuccin Mocha",
  EverforestLight = "Everforest Light",
  EverforestDark = "Everforest Dark",
  Fahrenheit = "Fahrenheit",
  FlexokiLight = "Flexoki Light",
  FlexokiDark = "Flexoki Dark",
  GruvboxLight = "Gruvbox Light",
  GruvboxDark = "Gruvbox Dark",
  Harper = "Harper",
  HybridLight = "Hybrid Light",
  HybridDark = "Hybrid Dark",
  Jellybeans = "Jellybeans",
  Kibble = "Kibble",
  MacOSClassicLight = "macOS Classic Light",
  MacOSClassicDark = "macOS Classic Dark",
  Matrix = "Matrix",
  MellifluousLight = "Mellifluous Light",
  MellifluousDark = "Mellifluous Dark",
  MolokaiLight = "Molokai Light",
  MolokaiDark = "Molokai Dark",
  SolarizedLight = "Solarized Light",
  SolarizedDark = "Solarized Dark",
  Spaceduck = "Spaceduck",
  TokyoNight = "Tokyo Night",
  TokyoStorm = "Tokyo Storm",
  TokyoMoon = "Tokyo Moon",
  Twilight = "Twilight",
}

export interface RasterThemePresetPair {
  light?: ThemePreset | `${ThemePreset}`;
  dark?: ThemePreset | `${ThemePreset}`;
}

export type RasterThemePreset = ThemePreset | `${ThemePreset}` | RasterThemePresetPair;

export interface RasterThemeColors {
  background?: string;
  foreground?: string;
  border?: string;
  input?: string;
  primary?: string;
  primaryForeground?: string;
  secondary?: string;
  secondaryForeground?: string;
  secondaryHover?: string;
  secondaryActive?: string;
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
  preset?: RasterThemePreset;
  mode?: RasterThemeMode;
  radius?: number;
  radiusLg?: number;
  fontSize?: number;
  fontFamily?: string;
  monoFontSize?: number;
  monoFontFamily?: string;
  colors?: RasterThemeColors;
}

export interface RasterResolvedThemeColors {
  accent: string;
  accentForeground: string;
  accordion: string;
  accordionHover: string;
  background: string;
  border: string;
  buttonPrimary: string;
  buttonPrimaryActive: string;
  buttonPrimaryForeground: string;
  buttonPrimaryHover: string;
  groupBox: string;
  groupBoxForeground: string;
  caret: string;
  chart1: string;
  chart2: string;
  chart3: string;
  chart4: string;
  chart5: string;
  chartBullish: string;
  chartBearish: string;
  danger: string;
  dangerActive: string;
  dangerForeground: string;
  dangerHover: string;
  descriptionListLabel: string;
  descriptionListLabelForeground: string;
  dragBorder: string;
  dropTarget: string;
  foreground: string;
  info: string;
  infoActive: string;
  infoForeground: string;
  infoHover: string;
  input: string;
  link: string;
  linkActive: string;
  linkHover: string;
  list: string;
  listActive: string;
  listActiveBorder: string;
  listEven: string;
  listHead: string;
  listHover: string;
  muted: string;
  mutedForeground: string;
  popover: string;
  popoverForeground: string;
  primary: string;
  primaryActive: string;
  primaryForeground: string;
  primaryHover: string;
  progressBar: string;
  ring: string;
  scrollbar: string;
  scrollbarThumb: string;
  scrollbarThumbHover: string;
  secondary: string;
  secondaryActive: string;
  secondaryForeground: string;
  secondaryHover: string;
  selection: string;
  sidebar: string;
  sidebarAccent: string;
  sidebarAccentForeground: string;
  sidebarBorder: string;
  sidebarForeground: string;
  sidebarPrimary: string;
  sidebarPrimaryForeground: string;
  skeleton: string;
  sliderBar: string;
  sliderThumb: string;
  success: string;
  successForeground: string;
  successHover: string;
  successActive: string;
  switch: string;
  switchThumb: string;
  tab: string;
  tabActive: string;
  tabActiveForeground: string;
  tabBar: string;
  tabBarSegmented: string;
  tabForeground: string;
  table: string;
  tableActive: string;
  tableActiveBorder: string;
  tableEven: string;
  tableHead: string;
  tableHeadForeground: string;
  tableFoot: string;
  tableFootForeground: string;
  tableHover: string;
  tableRowBorder: string;
  titleBar: string;
  titleBarBorder: string;
  tiles: string;
  warning: string;
  warningActive: string;
  warningHover: string;
  warningForeground: string;
  overlay: string;
  windowBorder: string;
  red: string;
  redLight: string;
  green: string;
  greenLight: string;
  blue: string;
  blueLight: string;
  yellow: string;
  yellowLight: string;
  magenta: string;
  magentaLight: string;
  cyan: string;
  cyanLight: string;
}

export interface RasterResolvedThemeEdges {
  top: number;
  right: number;
  bottom: number;
  left: number;
}

export interface RasterThemeStyleSnapshot {
  color?: string;
  fontStyle?: "normal" | "italic" | "underline";
  fontWeight?: 100 | 200 | 300 | 400 | 500 | 600 | 700 | 800 | 900;
}

export type RasterSyntaxColorsSnapshot = Record<string, RasterThemeStyleSnapshot | null | undefined>;

export interface RasterHighlightThemeStyleSnapshot {
  editorBackground?: string | null;
  editorForeground?: string | null;
  editorActiveLine?: string | null;
  editorLineNumber?: string | null;
  editorActiveLineNumber?: string | null;
  editorInvisible?: string | null;
  error?: string | null;
  errorBackground?: string | null;
  errorBorder?: string | null;
  warning?: string | null;
  warningBackground?: string | null;
  warningBorder?: string | null;
  info?: string | null;
  infoBackground?: string | null;
  infoBorder?: string | null;
  success?: string | null;
  successBackground?: string | null;
  successBorder?: string | null;
  hint?: string | null;
  hintBackground?: string | null;
  hintBorder?: string | null;
  syntax: RasterSyntaxColorsSnapshot;
}

export interface RasterHighlightThemeSnapshot {
  name: string;
  appearance: "light" | "dark";
  style: RasterHighlightThemeStyleSnapshot;
}

export interface RasterThemeConfigColorsSnapshot {
  accentBackground?: string;
  accentForeground?: string;
  accordionBackground?: string;
  accordionHoverBackground?: string;
  background?: string;
  border?: string;
  buttonPrimaryBackground?: string;
  buttonPrimaryActiveBackground?: string;
  buttonPrimaryForeground?: string;
  buttonPrimaryHoverBackground?: string;
  groupBoxBackground?: string;
  groupBoxForeground?: string;
  groupBoxTitleForeground?: string;
  caret?: string;
  chart1?: string;
  chart2?: string;
  chart3?: string;
  chart4?: string;
  chart5?: string;
  chartBullish?: string;
  chartBearish?: string;
  dangerBackground?: string;
  dangerActiveBackground?: string;
  dangerForeground?: string;
  dangerHoverBackground?: string;
  descriptionListLabelBackground?: string;
  descriptionListLabelForeground?: string;
  dragBorder?: string;
  dropTargetBackground?: string;
  foreground?: string;
  infoBackground?: string;
  infoActiveBackground?: string;
  infoForeground?: string;
  infoHoverBackground?: string;
  inputBorder?: string;
  link?: string;
  linkActive?: string;
  linkHover?: string;
  listBackground?: string;
  listActiveBackground?: string;
  listActiveBorder?: string;
  listEvenBackground?: string;
  listHeadBackground?: string;
  listHoverBackground?: string;
  mutedBackground?: string;
  mutedForeground?: string;
  popoverBackground?: string;
  popoverForeground?: string;
  primaryBackground?: string;
  primaryActiveBackground?: string;
  primaryForeground?: string;
  primaryHoverBackground?: string;
  progressBarBackground?: string;
  ring?: string;
  scrollbarBackground?: string;
  scrollbarThumbBackground?: string;
  scrollbarThumbHoverBackground?: string;
  secondaryBackground?: string;
  secondaryActiveBackground?: string;
  secondaryForeground?: string;
  secondaryHoverBackground?: string;
  selectionBackground?: string;
  sidebarBackground?: string;
  sidebarAccentBackground?: string;
  sidebarAccentForeground?: string;
  sidebarBorder?: string;
  sidebarForeground?: string;
  sidebarPrimaryBackground?: string;
  sidebarPrimaryForeground?: string;
  skeletonBackground?: string;
  sliderBackground?: string;
  sliderThumbBackground?: string;
  successBackground?: string;
  successForeground?: string;
  successHoverBackground?: string;
  successActiveBackground?: string;
  switchBackground?: string;
  switchThumbBackground?: string;
  tabBackground?: string;
  tabActiveBackground?: string;
  tabActiveForeground?: string;
  tabBarBackground?: string;
  tabBarSegmentedBackground?: string;
  tabForeground?: string;
  tableBackground?: string;
  tableActiveBackground?: string;
  tableActiveBorder?: string;
  tableEvenBackground?: string;
  tableHeadBackground?: string;
  tableHeadForeground?: string;
  tableFootBackground?: string;
  tableFootForeground?: string;
  tableHoverBackground?: string;
  tableRowBorder?: string;
  titleBarBackground?: string;
  titleBarBorder?: string;
  tilesBackground?: string;
  warningBackground?: string;
  warningActiveBackground?: string;
  warningHoverBackground?: string;
  warningForeground?: string;
  overlay?: string;
  windowBorder?: string;
  baseBlue?: string;
  baseBlueLight?: string;
  baseCyan?: string;
  baseCyanLight?: string;
  baseGreen?: string;
  baseGreenLight?: string;
  baseMagenta?: string;
  baseMagentaLight?: string;
  baseRed?: string;
  baseRedLight?: string;
  baseYellow?: string;
  baseYellowLight?: string;
}

export interface RasterThemeConfigSnapshot {
  isDefault: boolean;
  name: string;
  mode: "light" | "dark";
  fontSize?: number | null;
  fontFamily?: string | null;
  monoFontFamily?: string | null;
  monoFontSize?: number | null;
  radius?: number | null;
  radiusLg?: number | null;
  shadow?: boolean | null;
  colors: RasterThemeConfigColorsSnapshot;
  highlight?: RasterHighlightThemeStyleSnapshot | null;
}

export interface RasterResolvedTheme {
  colors: RasterResolvedThemeColors;
  highlightTheme: RasterHighlightThemeSnapshot;
  lightTheme: RasterThemeConfigSnapshot;
  darkTheme: RasterThemeConfigSnapshot;
  mode: "light" | "dark";
  fontFamily: string;
  fontSize: number;
  monoFontFamily: string;
  monoFontSize: number;
  radius: number;
  radiusLg: number;
  shadow: boolean;
  transparent: string;
  scrollbarShow: "scrolling" | "hover" | "always";
  notification: {
    placement:
      | "topLeft"
      | "topCenter"
      | "topRight"
      | "bottomLeft"
      | "bottomCenter"
      | "bottomRight"
      | "leftCenter"
      | "rightCenter";
    margins: RasterResolvedThemeEdges;
    maxItems: number;
  };
  tileGridSize: number;
  tileShadow: boolean;
  tileRadius: number;
  list: {
    activeHighlight: boolean;
  };
  sheet: {
    marginTop: number;
  };
}
