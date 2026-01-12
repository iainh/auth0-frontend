mod config;
mod errors;
mod helpers;
mod routes;
mod state;
mod templates;

use auth0_mgmt_api::ManagementClient;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::state::build_app;

fn init_tracing() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    init_tracing();

    let config = Config::from_env()?;

    let client = ManagementClient::builder()
        .domain(&config.auth0_domain)
        .client_id(&config.auth0_client_id)
        .client_secret(&config.auth0_client_secret)
        .build()?;

    let app = build_app(client);

    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    tracing::info!("listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
