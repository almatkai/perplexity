mod perplexity;
mod v2_perplexity;
use serde_json::Value;
use tezbar_extension_sdk::{
    BoxFuture, CommandConcurrency, CoreResult, Extension, ExtensionContext,
};
pub struct Perplexity;
impl Extension for Perplexity {
    fn id(&self) -> &'static str {
        "com.tezbar.native.perplexity"
    }
    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
    fn commands(&self) -> &'static [&'static str] {
        v2_perplexity::commands()
    }
    fn concurrency(&self, _command: &str) -> CommandConcurrency {
        CommandConcurrency::RejectWhileBusy
    }
    fn handle<'a>(
        &'a self,
        ctx: &'a ExtensionContext,
        command: &'a str,
        operation: &'a str,
        payload: &'a Value,
        state: &'a mut Value,
    ) -> BoxFuture<'a, CoreResult> {
        Box::pin(v2_perplexity::handle(
            ctx, command, operation, payload, state,
        ))
    }
}
