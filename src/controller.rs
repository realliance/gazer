//! Handles controller operations

use kube::api::ListParams;
use kube::{Api, Client};

use crate::StaticSite;

/// Builds controller to manage StaticSite resources
pub async fn build_controller() {
  let client = Client::try_default().await.expect("Failed to create client");
  let ensure_crd_installed: Api<StaticSite> = Api::all(client);
  let _check = ensure_crd_installed
    .list(&ListParams::default().limit(1))
    .await
    .expect("CRD is not installed");
}
