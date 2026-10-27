use std::future::Future;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use std::task::{Context, Poll, Wake, Waker};
use sysknife_brain::planner::{LlmPlanner, PlanningError};
use sysknife_brain::provider::{Completion, LlmProvider, Message, ProviderError, ToolDefinition};
use sysknife_brain::state_client::{CuratedState, StateClient};
use sysknife_types::{DistroHint, DISTRO_FAMILY_DEBIAN};

struct Capture(Arc<AtomicUsize>);

#[async_trait::async_trait]
impl LlmProvider for Capture {
    async fn complete(&self, system: &str, _: &[Message], tools: &[ToolDefinition], max_tokens: u32)
        -> Result<Completion, ProviderError>
    {
        self.0.fetch_add(1, Ordering::SeqCst);
        let tools: Vec<_> = tools.iter().map(|tool| serde_json::json!({
            "type": "function", "function": {"name": tool.name,
            "description": tool.description, "parameters": tool.input_schema}
        })).collect();
        // Capture the real production factory output through its public provider
        // boundary. No plan or successful provider reply is supplied here.
        println!("{}", serde_json::json!({
            "model": "sysknife-pr695", "stream": false, "keep_alive": "30m",
            "max_tokens": max_tokens,
            "messages": [{"role": "system", "content": system},
                         {"role": "user", "content": "Acknowledge initialization briefly."}],
            "tools": tools, "options": {"num_predict": 1}
        }));
        // Stop at the first provider boundary without returning a model reply
        // or entering the planner's transport-error retry path.
        std::future::pending().await
    }
}

struct NoQueries;
impl StateClient for NoQueries {
    fn curated_state(&self) -> Result<CuratedState, PlanningError> {
        panic!("prefix capture must not fabricate system state")
    }
    fn query_action(&self, _: &str, _: &serde_json::Value) -> Result<String, PlanningError> {
        panic!("prefix capture must not fabricate query results")
    }
}

struct NoopWake;
impl Wake for NoopWake { fn wake(self: Arc<Self>) {} }
fn capture<F: Future>(future: F) {
    let waker = Waker::from(Arc::new(NoopWake));
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending),
        "capture must stop before any model reply");
}

fn main() {
    let calls = Arc::new(AtomicUsize::new(0));
    let planner = LlmPlanner::new(Box::new(Capture(calls.clone())), Box::new(NoQueries), 1)
        .with_distro(DistroHint {id: "ubuntu".into(), family: DISTRO_FAMILY_DEBIAN,
            version: Some("Ubuntu 24.04".into())});
    capture(planner.plan_intent("Acknowledge initialization briefly."));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
