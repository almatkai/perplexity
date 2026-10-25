use serde_json::{json, Value};
use std::{process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
};
struct Process {
    child: Child,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
    dir: tempfile::TempDir,
}
impl Process {
    async fn start() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_tezbar-perplexity"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let input = child.stdin.take();
        let output = BufReader::new(child.stdout.take().unwrap());
        let dir = tempfile::tempdir().unwrap();
        let mut p = Self {
            child,
            input,
            output,
            dir,
        };
        let hello = p.recv().await;
        assert_eq!(
            hello,
            json!({"type":"hello","protocol":1,"id":"com.tezbar.native.perplexity","version":"1.0.0","sdkVersion":"0.1.0"})
        );
        let params = json!({"protocol":1,"extensionId":"com.tezbar.native.perplexity","platform":std::env::consts::OS,"packageDir":p.dir.path(),"dataDir":p.dir.path().join("data"),"tempDir":p.dir.path().join("temp"),"grantedCapabilities":["shell.open"],"reason":"test","migrationPaths":{}});
        assert_eq!(
            p.invoke("initialize", params).await["result"]["ready"],
            true
        );
        p
    }
    async fn recv(&mut self) -> Value {
        let mut line = String::new();
        assert!(
            tokio::time::timeout(Duration::from_secs(5), self.output.read_line(&mut line))
                .await
                .expect("stdio timeout")
                .unwrap()
                > 0
        );
        serde_json::from_str(&line).unwrap()
    }
    async fn send(&mut self, value: Value) {
        let mut bytes = serde_json::to_vec(&value).unwrap();
        bytes.push(b'\n');
        self.input
            .as_mut()
            .unwrap()
            .write_all(&bytes)
            .await
            .unwrap();
    }
    fn request(method: &str, params: Value) -> Value {
        json!({"type":"request","id":format!("h:{}",uuid::Uuid::new_v4()),"method":method,"params":params})
    }
    async fn invoke(&mut self, method: &str, params: Value) -> Value {
        let request = Self::request(method, params);
        self.send(request.clone()).await;
        let response = self.recv().await;
        assert_eq!(response["id"], request["id"]);
        response
    }
    async fn open(&mut self, id: &str, app: bool) {
        assert_eq!(self.invoke("session.open",json!({"sessionId":id,"commandName":"ask-perplexity","preferences":{"perplexityApp":app},"argumentValues":{}})).await["result"]["mode"],"view");
    }
    async fn finish(&mut self) {
        drop(self.input.take());
        assert!(
            tokio::time::timeout(Duration::from_secs(6), self.child.wait())
                .await
                .expect("cleanup timeout")
                .unwrap()
                .success()
        );
    }
}
#[tokio::test]
async fn real_form_validation_unicode_reverse_shell_and_dispose() {
    let mut p = Process::start().await;
    p.open("web", false).await;
    p.open("app", true).await;
    for query in [Value::Null, json!(""), json!(1), json!([])] {
        assert!(p.invoke("session.call",json!({"sessionId":"web","operation":"extension:invoke-action","payload":{"actionId":"perplexity:submit","formValues":{"query":query}}})).await.get("error").is_some());
    }
    let query = "Қазақ 日本語 😀\n +?&%~!*'()\r\n";
    let encoded="%D2%9A%D0%B0%D0%B7%D0%B0%D2%9B+%E6%97%A5%E6%9C%AC%E8%AA%9E+%F0%9F%98%80%0A+%2B%3F%26%25%7E%21*%27%28%29%0D%0A";
    for (id, base) in [
        ("web", "https://www.perplexity.ai/search"),
        ("app", "perplexity-app://search"),
    ] {
        let request = Process::request(
            "session.call",
            json!({"sessionId":id,"operation":"extension:invoke-action","payload":{"actionId":"perplexity:submit","formValues":{"query":query},"preferences":{"perplexityApp":id=="web"},"sessionId":"spoofed"}}),
        );
        p.send(request.clone()).await;
        let reverse = p.recv().await;
        assert_eq!(reverse["method"], "host.shell.open");
        assert!(reverse["id"].as_str().unwrap().starts_with("e:"));
        assert_eq!(
            reverse["params"],
            json!({"target":format!("{base}?q={encoded}")})
        );
        // While the shell RPC is pending, a second submit is rejected without a second shell action.
        assert!(p.invoke("session.call",json!({"sessionId":id,"operation":"extension:invoke-action","payload":{"actionId":"perplexity:submit","formValues":{"query":"duplicate"}}})).await.get("error").is_some());
        p.send(json!({"type":"response","id":reverse["id"],"result":null}))
            .await;
        let result = p.recv().await;
        assert_eq!(result["id"], request["id"]);
        assert_eq!(
            result["result"]["effects"][0]["value"],
            reverse["params"]["target"]
        );
    }
    assert_eq!(
        p.invoke(
            "session.call",
            json!({"sessionId":"web","operation":"extension:refresh-session"})
        )
        .await["result"]["mode"],
        "unchanged"
    );
    assert!(p
        .invoke("session.dispose", json!({"sessionId":"web"}))
        .await
        .get("result")
        .is_some());
    assert!(p.invoke("session.call",json!({"sessionId":"web","operation":"extension:invoke-action","payload":{"actionId":"perplexity:submit","formValues":{"query":"stale"}}})).await.get("error").is_some());
    p.finish().await;
}
#[tokio::test]
async fn immediate_argument_failure_retry_and_eof_cleanup() {
    let mut p = Process::start().await;
    for args in [json!([]), json!({"query":false})] {
        assert!(p
            .invoke(
                "session.open",
                json!({"sessionId":"invalid","commandName":"ask-perplexity","argumentValues":args})
            )
            .await
            .get("error")
            .is_some());
    }
    let request = Process::request(
        "session.open",
        json!({"sessionId":"immediate","commandName":"ask-perplexity","argumentValues":{"query":"a+b c"}}),
    );
    p.send(request.clone()).await;
    let reverse = p.recv().await;
    assert_eq!(
        reverse["params"]["target"],
        "https://www.perplexity.ai/search?q=a%2Bb+c"
    );
    p.send(json!({"type":"response","id":reverse["id"],"error":{"code":"open_failed","message":"fake open failed"}})).await;
    assert_eq!(p.recv().await["error"]["message"], "fake open failed");
    p.open("immediate", false).await;
    let request = Process::request(
        "session.call",
        json!({"sessionId":"immediate","operation":"extension:invoke-action","payload":{"actionId":"perplexity:submit","formValues":{"query":"pending"}}}),
    );
    p.send(request).await;
    assert_eq!(p.recv().await["type"], "request");
    p.finish().await;
}
#[tokio::test]
async fn cancellation_dispose_during_action_and_restart() {
    let mut p = Process::start().await;
    p.open("one", false).await;
    let request = Process::request(
        "session.call",
        json!({"sessionId":"one","operation":"extension:invoke-action","payload":{"actionId":"perplexity:submit","formValues":{"query":"hold"}}}),
    );
    p.send(request.clone()).await;
    let reverse = p.recv().await;
    let dispose = Process::request("session.dispose", json!({"sessionId":"one"}));
    p.send(dispose.clone()).await;
    let mut saw_cancel = false;
    let mut saw_dispose = false;
    let mut saw_error = false;
    for _ in 0..3 {
        let value = p.recv().await;
        if value["type"] == "cancel" {
            assert_eq!(value["requestId"], reverse["id"]);
            saw_cancel = true;
        } else if value["id"] == dispose["id"] {
            assert!(value.get("result").is_some());
            saw_dispose = true;
        } else {
            assert_eq!(value["id"], request["id"]);
            assert!(value.get("error").is_some());
            saw_error = true;
        }
    }
    assert!(saw_cancel && saw_dispose && saw_error);
    assert!(p
        .invoke("shutdown", json!({}))
        .await
        .get("result")
        .is_some());
    assert!(tokio::time::timeout(Duration::from_secs(2), p.child.wait())
        .await
        .expect("shutdown waits for stdin EOF")
        .unwrap()
        .success());
    p.finish().await;
    let mut restarted = Process::start().await;
    assert!(restarted
        .invoke(
            "session.call",
            json!({"sessionId":"one","operation":"extension:refresh-session"})
        )
        .await
        .get("error")
        .is_some());
    restarted.finish().await;
}
#[test]
fn executable_protocol_info_and_manifest_match() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tezbar-perplexity"))
        .arg("--protocol-info")
        .output()
        .unwrap();
    assert!(output.status.success());
    let info: Value = serde_json::from_slice(&output.stdout).unwrap();
    let manifest: Value = serde_json::from_str(include_str!("../extension.json")).unwrap();
    assert_eq!(info["id"], manifest["id"]);
    assert_eq!(info["version"], manifest["version"]);
    assert_eq!(info["protocol"], manifest["protocolVersion"]);
    assert_eq!(info["commands"], json!(["ask-perplexity"]));
}
