//! Gazer is an automatic static site deployer for Kubernetes.

use cli::build_cli;
use controller::build_controller;
use crd::StaticSite;
use kube::CustomResourceExt;

pub mod cli;
pub mod controller;
pub mod crd;

#[tokio::main]
async fn main() {
  // Get command line arguments
  let cli_args = build_cli();

  // Generate CRD if flagged
  if cli_args.gen_crd {
    print!("{}", serde_yaml::to_string(&StaticSite::crd()).unwrap());
    return;
  }

  build_controller().await;
}
