use std::future::Future;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait Task: Send + Sync {
	async fn run(&self, input: &Value) -> Result<Value>;
}

/// Wrap an async closure as a `Task`.
pub fn from_async_fn<F, Fut>(f: F) -> Arc<dyn Task>
where
	F: Send + Sync + 'static + Fn(&Value) -> Fut,
	Fut: Future<Output = Result<Value>> + Send + 'static,
{
	struct ClosureTask<F, Fut>
	where
		F: Send + Sync + 'static + Fn(&serde_json::Value) -> Fut,
		Fut: Future<Output = Result<serde_json::Value>> + Send + 'static,
	{
		f: F,
	}

	#[async_trait]
	impl<F, Fut> Task for ClosureTask<F, Fut>
	where
		F: Send + Sync + 'static + Fn(&serde_json::Value) -> Fut,
		Fut: Future<Output = Result<serde_json::Value>> + Send + 'static,
	{
		async fn run(&self, input: &Value) -> Result<Value> {
			(self.f)(input).await
		}
	}

	Arc::new(ClosureTask { f })
}


