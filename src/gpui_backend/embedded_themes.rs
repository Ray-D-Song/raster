use gpui::App;
use gpui_component::{ThemeConfig, ThemeMode, ThemeRegistry};
use std::rc::Rc;

use crate::common::utils::logger;

pub(in crate::gpui_backend) const DEFAULT_LIGHT_THEME: &str = "macOS Classic Light";
pub(in crate::gpui_backend) const DEFAULT_DARK_THEME: &str = "macOS Classic Dark";

const EMBEDDED_THEMES: &[&str] = &[
    include_str!("../../deps/gpui-component/themes/adventure.json"),
    include_str!("../../deps/gpui-component/themes/alduin.json"),
    include_str!("../../deps/gpui-component/themes/asciinema.json"),
    include_str!("../../deps/gpui-component/themes/ayu.json"),
    include_str!("../../deps/gpui-component/themes/catppuccin.json"),
    include_str!("../../deps/gpui-component/themes/everforest.json"),
    include_str!("../../deps/gpui-component/themes/fahrenheit.json"),
    include_str!("../../deps/gpui-component/themes/flexoki.json"),
    include_str!("../../deps/gpui-component/themes/gruvbox.json"),
    include_str!("../../deps/gpui-component/themes/harper.json"),
    include_str!("../../deps/gpui-component/themes/hybrid.json"),
    include_str!("../../deps/gpui-component/themes/jellybeans.json"),
    include_str!("../../deps/gpui-component/themes/kibble.json"),
    include_str!("../../deps/gpui-component/themes/macos-classic.json"),
    include_str!("../../deps/gpui-component/themes/matrix.json"),
    include_str!("../../deps/gpui-component/themes/mellifluous.json"),
    include_str!("../../deps/gpui-component/themes/molokai.json"),
    include_str!("../../deps/gpui-component/themes/solarized.json"),
    include_str!("../../deps/gpui-component/themes/spaceduck.json"),
    include_str!("../../deps/gpui-component/themes/tokyonight.json"),
    include_str!("../../deps/gpui-component/themes/twilight.json"),
];

pub(in crate::gpui_backend) fn load_embedded_themes(cx: &mut App) {
    let registry = ThemeRegistry::global_mut(cx);
    for source in EMBEDDED_THEMES {
        if let Err(error) = registry.load_themes_from_str(source) {
            logger::warn(format!("failed to load embedded Raster theme: {error}"));
        }
    }
}

pub(in crate::gpui_backend) fn registry_theme(cx: &App, name: &str) -> Option<Rc<ThemeConfig>> {
    ThemeRegistry::global(cx).themes().get(name).cloned()
}

pub(in crate::gpui_backend) fn default_theme_name(mode: ThemeMode) -> &'static str {
    if mode.is_dark() {
        DEFAULT_DARK_THEME
    } else {
        DEFAULT_LIGHT_THEME
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_component::ThemeSet;
    use std::collections::BTreeSet;

    #[test]
    fn embedded_themes_parse_and_include_expected_names() {
        let names = EMBEDDED_THEMES
            .iter()
            .flat_map(|source| {
                serde_json::from_str::<ThemeSet>(source)
                    .expect("theme set")
                    .themes
            })
            .map(|theme| theme.name.to_string())
            .collect::<BTreeSet<_>>();

        assert!(names.contains("Ayu Light"));
        assert!(names.contains("Ayu Dark"));
        assert!(names.contains(DEFAULT_LIGHT_THEME));
        assert!(names.contains(DEFAULT_DARK_THEME));
        assert!(names.contains("Tokyo Moon"));
    }
}
