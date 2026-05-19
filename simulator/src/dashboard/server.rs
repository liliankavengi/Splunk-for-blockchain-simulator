use std::sync::Arc;

use axum::{middleware, routing::get, Router};
use tokio::task::JoinHandle;

use crate::dashboard::auth::bearer_auth;
use crate::dashboard::handlers::{index_handler, metrics_handler, status_handler, DashboardCtx};
use crate::dashboard::metrics::SimMetrics;
use crate::engine::EngineState;

pub struct DashboardServer {
    pub ctx: DashboardCtx,
    pub password: String,
    pub port: u16,
}

impl DashboardServer {
    pub fn new(
        metrics: Arc<SimMetrics>,
        state: Arc<EngineState>,
        scenario_name: String,
        password: String,
        port: u16,
    ) -> Self {
        Self {
            ctx: DashboardCtx {
                metrics,
                state,
                scenario_name,
            },
            password,
            port,
        }
    }

    /// Spawns the Axum HTTP server in the background.
    pub async fn serve(self) -> anyhow::Result<JoinHandle<()>> {
        let password = self.password.clone();
        let ctx = self.ctx.clone();

        let app = Router::new()
            .route("/", get(index_handler))
            .route("/status", get(status_handler))
            .route("/metrics", get(metrics_handler))
            .route_layer(middleware::from_fn(move |req, next| {
                let pw = password.clone();
                bearer_auth(pw, req, next)
            }))
            .with_state(ctx);

        let addr = format!("0.0.0.0:{}", self.port);
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .map_err(|e| anyhow::anyhow!("Dashboard bind failed on {}: {}", addr, e))?;

        tracing::info!(addr, "Admin dashboard listening");

        let handle = tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                tracing::error!(error = %e, "Dashboard server error");
            }
        });

        Ok(handle)
    }
}
