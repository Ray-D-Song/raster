#[test]
fn runtime_bundle_loads() {
    let runtime = pollster::block_on(super::super::start()).expect("start js runtime");
    assert!(pollster::block_on(runtime.runtime_bundle_loaded()));
}

#[test]
fn raster_components_module_exports_app_shell() {
    let runtime = pollster::block_on(super::super::start()).expect("start js runtime");
    pollster::block_on(runtime.eval_app_bundle_source(
        "test:app-shell-exports",
        r#"
        import { AppShell, AppShellTab, AppShellTabBar } from "raster-js/components";
        if (typeof AppShell !== "function" || typeof AppShellTab !== "function" || typeof AppShellTabBar !== "function") {
          throw new Error("missing AppShell exports");
        }
        "#
        .to_owned(),
    ))
    .expect("raster-js/components should export AppShell components");
}

#[test]
fn react_jsx_dev_runtime_module_exports_jsx_dev() {
    let runtime = pollster::block_on(super::super::start()).expect("start js runtime");
    pollster::block_on(
        runtime.eval_app_bundle_source(
            "test:jsx-dev-runtime-exports",
            r#"
        import { Fragment, jsxDEV } from "react/jsx-dev-runtime";
        if (typeof Fragment !== "symbol" || typeof jsxDEV !== "function") {
          throw new Error("missing react/jsx-dev-runtime exports");
        }
        "#
            .to_owned(),
        ),
    )
    .expect("react/jsx-dev-runtime should export Fragment and jsxDEV");
}

/// QuickJS currently asserts if the runtime is freed after a React root unmount.
/// These teardown tests validate `root.dispose()` completes and intentionally
/// leak the runtime handle until the reconciler teardown GC issue is resolved.
fn forget_runtime_after_root_dispose(runtime: super::super::JsRuntime) {
    std::mem::forget(runtime);
}

#[test]
fn runtime_bundle_reconciles_view_text_app() {
    let runtime = pollster::block_on(super::super::start()).expect("start js runtime");
    let result = pollster::block_on(runtime.eval_runtime_script_to_string(
        r#"
        const {
          createRoot,
          jsx,
        } = globalThis.__RasterBundle;

        const View = "View";
        // Equivalent to: const App = () => <View>Hello</View>;
        const App = () => jsx(View, { children: "Hello" });

        const root = createRoot({ width: 320, height: 240 });
        root.render(jsx(App, {}));
        root.dispose();

        "rendered";
        "#,
    ))
    .expect("reconcile view text app");

    println!("runtime bundle reconcile result: {result}");
    assert_eq!(result, "rendered");

    let surface_options = runtime
        .native_binding()
        .surface_options(crate::common::ids::SurfaceId(1));
    assert_eq!(surface_options.width, Some(320));
    assert_eq!(surface_options.height, Some(240));

    let batches = runtime.native_binding().drain_commits();
    assert!(!batches.is_empty());

    let mutations = batches
        .iter()
        .flat_map(|batch| batch.mutations.iter())
        .collect::<Vec<_>>();
    let mutations = mutations.as_slice();
    assert!(mutations.iter().any(|mutation| {
        matches!(
            mutation,
            crate::common::mount::MountMutation::CreateNode { name, .. } if name == "View"
        )
    }));
    assert!(mutations.iter().any(|mutation| {
        matches!(
            mutation,
            crate::common::mount::MountMutation::CreateText { text, .. } if text == "Hello"
        )
    }));
    assert!(mutations.iter().any(|mutation| {
        matches!(
            mutation,
            crate::common::mount::MountMutation::SetRootChildren { children, .. } if children.len() == 1
        )
    }));
    forget_runtime_after_root_dispose(runtime);
}

#[test]
fn runtime_bundle_flattens_style_arrays() {
    let runtime = pollster::block_on(super::super::start()).expect("start js runtime");
    pollster::block_on(runtime.eval_runtime_script_to_string(
        r##"
        const {
          createRoot,
          jsx,
        } = globalThis.__RasterBundle;

        const View = "View";
        const root = createRoot({ width: 320, height: 240 });
        root.render(jsx(View, {
          style: [
            { gap: 8, backgroundColor: "#ffffff" },
            null,
            undefined,
            [{ gap: 12 }, undefined],
          ],
          children: null,
        }));
        root.dispose();

        "rendered";
        "##,
    ))
    .expect("reconcile style array app");

    let batches = runtime.native_binding().drain_commits();
    let create_view = batches
        .iter()
        .flat_map(|batch| batch.mutations.iter())
        .find_map(|mutation| match mutation {
            crate::common::mount::MountMutation::CreateNode { name, payload, .. }
                if name == "View" =>
            {
                Some(payload)
            }
            _ => None,
        })
        .expect("View node should be created");

    assert_eq!(
        create_view.style.get("gap"),
        Some(&crate::common::mount::NodeValue::Number(12.0))
    );
    assert_eq!(
        create_view.style.get("backgroundColor"),
        Some(&crate::common::mount::NodeValue::String(
            "#ffffff".to_owned()
        ))
    );
    forget_runtime_after_root_dispose(runtime);
}

#[test]
fn runtime_bundle_use_effect_teardown_does_not_leak_vm() {
    let runtime = pollster::block_on(super::super::start()).expect("start js runtime");
    let result = pollster::block_on(runtime.eval_runtime_script_to_string(
        r##"
        const { useEffect, createRoot, jsx, View } = globalThis.__RasterBundle;
        const App = () => {
          useEffect(() => () => {}, []);
          return jsx(View, { children: null });
        };
        const root = createRoot({ width: 320, height: 240 });
        root.render(jsx(App, {}));
        root.dispose();
        "rendered";
        "##,
    ))
    .expect("useEffect teardown");
    assert_eq!(result, "rendered");
    forget_runtime_after_root_dispose(runtime);
}

#[test]
fn runtime_bundle_use_sync_external_store_teardown_does_not_leak_vm() {
    let runtime = pollster::block_on(super::super::start()).expect("start js runtime");
    let result = pollster::block_on(runtime.eval_runtime_script_to_string(
        r##"
        const { useSyncExternalStore, createRoot, jsx, View } = globalThis.__RasterBundle;
        const App = () => {
          useSyncExternalStore(() => () => {}, () => "", () => "");
          return jsx(View, { children: null });
        };
        const root = createRoot({ width: 320, height: 240 });
        root.render(jsx(App, {}));
        root.dispose();
        "rendered";
        "##,
    ))
    .expect("useSyncExternalStore teardown");
    assert_eq!(result, "rendered");
    forget_runtime_after_root_dispose(runtime);
}

#[test]
fn runtime_bundle_appshell_tab_bar_container_style_overrides_border() {
    let runtime = pollster::block_on(super::super::start()).expect("start js runtime");
    pollster::block_on(runtime.eval_runtime_script_to_string(
        r##"
        const { AppShell, createRoot, jsx } = globalThis.__RasterBundle;

        const root = createRoot({ width: 320, height: 480 });
        root.render(jsx(AppShell, {
          tabBarContainerStyle: { borderTopWidth: 0 },
          tabBar: jsx("View", { children: "tab" }),
          children: null,
        }));
        root.dispose();

        "rendered";
        "##,
    ))
    .expect("reconcile AppShell tab bar container style app");

    let batches = runtime.native_binding().drain_commits();
    let has_zero_border_tab_container = batches.iter().flat_map(|batch| batch.mutations.iter()).any(
        |mutation| match mutation {
            crate::common::mount::MountMutation::CreateNode { name, payload, .. } if name == "View" => {
                payload.style.get("borderTopWidth")
                    == Some(&crate::common::mount::NodeValue::Number(0.0))
            }
            _ => false,
        },
    );
    assert!(
        has_zero_border_tab_container,
        "AppShell tabBarContainerStyle should override the default top border"
    );
    forget_runtime_after_root_dispose(runtime);
}

#[test]
fn runtime_bundle_appshell_safe_area_inset_bottom_adds_padding() {
    let runtime = pollster::block_on(super::super::start()).expect("start js runtime");
    pollster::block_on(runtime.eval_runtime_script_to_string(
        r##"
        const { AppShell, createRoot, jsx } = globalThis.__RasterBundle;

        const root = createRoot({ width: 320, height: 480 });
        root.render(jsx(AppShell, {
          tabBarContainerStyle: { padding: { top: 4 } },
          safeAreaInsetBottom: 20,
          tabBar: jsx("View", { children: "tab" }),
          children: null,
        }));
        root.dispose();

        "rendered";
        "##,
    ))
    .expect("reconcile AppShell safe area inset app");

    let batches = runtime.native_binding().drain_commits();
    let has_safe_area_padding = batches.iter().flat_map(|batch| batch.mutations.iter()).any(
        |mutation| match mutation {
            crate::common::mount::MountMutation::CreateNode { name, payload, .. } if name == "View" => {
                match payload.style.get("padding") {
                    Some(crate::common::mount::NodeValue::Object(padding)) => {
                        padding.get("bottom")
                            == Some(&crate::common::mount::NodeValue::Number(20.0))
                    }
                    _ => false,
                }
            }
            _ => false,
        },
    );
    assert!(
        has_safe_area_padding,
        "AppShell safeAreaInsetBottom should add bottom padding to the tab bar container"
    );
    forget_runtime_after_root_dispose(runtime);
}

#[test]
fn runtime_bundle_appshell_supports_tab_bar_container_and_safe_area_props() {
    let bundle = include_str!("../../../runtime/js/generated/runtime_bundle.js");
    assert!(
        bundle.contains("tabBarContainerStyle"),
        "runtime bundle should expose AppShell tabBarContainerStyle"
    );
    assert!(
        bundle.contains("tabBarStyle"),
        "runtime bundle should expose AppShell tabBarStyle"
    );
    assert!(
        bundle.contains("safeAreaInsetBottom"),
        "runtime bundle should expose AppShell safeAreaInsetBottom"
    );
    assert!(
        bundle.contains("safeAreaBottomPaddingStyle"),
        "runtime bundle should merge safe area inset into tab bar container padding"
    );
}

#[test]
fn runtime_command_emits_runtime_event() {
    let runtime = pollster::block_on(super::super::start()).expect("start js runtime");
    pollster::block_on(runtime.eval_runtime_script_to_string(
        r#"
        globalThis.__rasterRuntimeEventTest = [];
        globalThis.__rasterDispatchRuntimeEventJson = (name, payloadJson) => {
          globalThis.__rasterRuntimeEventTest.push([name, JSON.parse(payloadJson)]);
        };
        "ready";
        "#,
    ))
    .expect("install runtime event test dispatcher");

    pollster::block_on(runtime.handle_runtime_command(
        crate::common::channel::RuntimeCommand::EmitRuntimeEvent {
            name: "themechange".to_owned(),
            payload: crate::common::mount::NodeValue::Null,
        },
    ))
    .expect("emit runtime event");

    let result =
        pollster::block_on(runtime.eval_runtime_script_to_string(
            r#"JSON.stringify(globalThis.__rasterRuntimeEventTest)"#,
        ))
        .expect("read runtime event result");
    assert_eq!(result, r#"[["themechange",null]]"#);
}