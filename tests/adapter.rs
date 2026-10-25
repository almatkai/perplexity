use serde_json::{json, Value};
use tezbar_extension_sdk::{Extension, ExtensionPaths, TestContext};
use tezbar_perplexity::Perplexity;
fn fixture(dir: &std::path::Path) -> TestContext {
    TestContext::new(
        ExtensionPaths {
            package_dir: dir.into(),
            data_dir: dir.join("data"),
            temp_dir: dir.join("temp"),
        },
        &["shell.open"],
    )
}
#[tokio::test]
async fn form_and_validation_preserve_original_behavior() {
    let dir = tempfile::tempdir().unwrap();
    let test = fixture(dir.path());
    let mut state = json!({});
    for args in [json!({}), json!({"query":""})] {
        let payload = json!({"argumentValues":args});
        let form = Perplexity
            .handle(
                &test.context,
                "ask-perplexity",
                "extension:run-command",
                &payload,
                &mut state,
            )
            .await
            .unwrap();
        assert_eq!(form["mode"], "view");
        assert_eq!(form["title"], "Ask Perplexity");
        assert_eq!(
            form["root"]["children"][0],
            json!({"type":"Form.TextArea","props":{"id":"query","title":"Ask Anything"}})
        );
        assert!(form.get("sessionId").is_none());
        assert_eq!(form["actions"][0]["kind"], "submit-form");
    }
    for query in [Value::Null, json!(10), json!([]), json!({}), json!("")] {
        let payload = json!({"actionId":"perplexity:submit","formValues":{"query":query}});
        assert!(Perplexity
            .handle(
                &test.context,
                "ask-perplexity",
                "extension:invoke-action",
                &payload,
                &mut state
            )
            .await
            .is_err());
    }
    for payload in [
        json!({}),
        json!({"actionId":"wrong"}),
        json!({"actionId":"perplexity:submit","formValues":[]}),
    ] {
        assert!(Perplexity
            .handle(
                &test.context,
                "ask-perplexity",
                "extension:invoke-action",
                &payload,
                &mut state
            )
            .await
            .is_err());
    }
    for args in [
        Value::Null,
        json!([]),
        json!("query"),
        json!({"query":false}),
        json!({"query":1}),
        json!({"query":null}),
    ] {
        let payload = json!({"argumentValues":args});
        assert!(Perplexity
            .handle(
                &test.context,
                "ask-perplexity",
                "extension:run-command",
                &payload,
                &mut state
            )
            .await
            .is_err());
    }
    assert!(test.calls().is_empty());
}
#[tokio::test]
async fn urls_preferences_exact_encoding_and_failure() {
    let dir = tempfile::tempdir().unwrap();
    let test = fixture(dir.path());
    let mut state = json!({});
    let query = "Қазақ 日本語 😀\n +?&%~!*'()\r\n";
    let encoded="%D2%9A%D0%B0%D0%B7%D0%B0%D2%9B+%E6%97%A5%E6%9C%AC%E8%AA%9E+%F0%9F%98%80%0A+%2B%3F%26%25%7E%21*%27%28%29%0D%0A";
    for (app, base) in [
        (false, "https://www.perplexity.ai/search"),
        (true, "perplexity-app://search"),
    ] {
        let payload = json!({"preferences":{"perplexityApp":app},"argumentValues":{"query":query}});
        let result = test
            .run(Perplexity.handle(
                &test.context,
                "ask-perplexity",
                "extension:run-command",
                &payload,
                &mut state,
            ))
            .await
            .unwrap();
        assert_eq!(result["mode"], "no-view");
        assert_eq!(result["effects"][0]["value"], format!("{base}?q={encoded}"));
        assert_eq!(
            test.calls().last().unwrap()["params"]["target"],
            result["effects"][0]["value"]
        );
    }
    let payload = json!({"actionId":"perplexity:submit","formValues":{"query":" "}});
    assert_eq!(
        test.run(Perplexity.handle(
            &test.context,
            "ask-perplexity",
            "extension:invoke-action",
            &payload,
            &mut state
        ))
        .await
        .unwrap()["effects"][0]["value"],
        "https://www.perplexity.ai/search?q=+"
    );
    test.reply(Err("fake open failed".into()));
    assert_eq!(
        test.run(Perplexity.handle(
            &test.context,
            "ask-perplexity",
            "extension:invoke-action",
            &payload,
            &mut state
        ))
        .await
        .unwrap_err(),
        "fake open failed"
    );
}
#[tokio::test]
async fn refresh_dispose_and_unsupported_operations() {
    let dir = tempfile::tempdir().unwrap();
    let test = fixture(dir.path());
    let mut state = json!({});
    for op in ["extension:refresh-session", "extension:dispose-session"] {
        assert_eq!(
            Perplexity
                .handle(&test.context, "ask-perplexity", op, &json!({}), &mut state)
                .await
                .unwrap()["mode"],
            "unchanged"
        );
    }
    for op in [
        "extension:search-text-changed",
        "extension:load-more",
        "extension:unknown",
    ] {
        assert!(Perplexity
            .handle(&test.context, "ask-perplexity", op, &json!({}), &mut state)
            .await
            .is_err());
    }
    assert!(Perplexity
        .handle(
            &test.context,
            "other",
            "extension:run-command",
            &json!({}),
            &mut state
        )
        .await
        .is_err());
}
