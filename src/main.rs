//! Gazer is an automatic static site deployer for Kubernetes.

use cli::build_cli;
use controller::build_controller;
use crd::StaticSite;
use kube::CustomResourceExt;
use tracing::{info, warn};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, Registry};

pub mod cli;
pub mod controller;
pub mod crd;

#[tokio::main]
async fn main() {
  // Init tracing
  let logger = tracing_subscriber::fmt::layer().json();
  let env_filter = EnvFilter::try_from_default_env()
    .or_else(|_| EnvFilter::try_new("info"))
    .unwrap();
  let collector = Registry::default().with(logger).with(env_filter);
  tracing::subscriber::set_global_default(collector).unwrap();

  // Get command line arguments
  let cli_args = build_cli();

  // Generate CRD if flagged
  if cli_args.gen_crd {
    print!("{}", serde_yaml::to_string(&StaticSite::crd()).unwrap());
    return;
  }

  let drainer = build_controller().await;

  info!("Controller started");

  tokio::select! {
    _ = drainer => warn!("Controller Drained")
  }
}
