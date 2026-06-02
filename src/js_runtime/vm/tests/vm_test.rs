#[test]
fn runtime_bundle_loads() {
    let runtime = pollster::block_on(super::super::start()).expect("start js runtime");
    assert!(pollster::block_on(runtime.runtime_bundle_loaded()));
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

        "rendered";
        "#,
    ))
    .expect("reconcile view text app");

    println!("runtime bundle reconcile result: {result}");
    assert_eq!(result, "rendered");

    let batches = runtime.native_binding().drain_commits();
    assert_eq!(batches.len(), 1);

    let mutations = &batches[0].mutations;
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
}
