//! Handles controller operations

use futures::future::BoxFuture;
use futures::{FutureExt, StreamExt};
use kube::api::ListParams;
use kube::runtime::controller::{Context, ReconcilerAction};
use kube::runtime::wait::delete::Error;
use kube::runtime::Controller;
use kube::{Api, Client, ResourceExt};
use tokio::time::Duration;
use tracing::{info, warn};

use crate::StaticSite;

/// Context Data
#[derive(Clone)]
struct Data {
  /// Client to be used during reconciliation
  #[allow(dead_code)]
  client: Client,
}

async fn reconcile(site: StaticSite, ctx: Context<Data>) -> Result<ReconcilerAction, Error> {
  let _client = ctx.get_ref().client.clone();
  let _ns = ResourceExt::namespace(&site).expect("Expected site to be namespaced");
  let name = ResourceExt::name(&site);

  info!("Reconciled {}", name);

  Ok(ReconcilerAction {
    requeue_after: Some(Duration::from_secs(360)),
  })
}

fn error_policy(error: &Error, _: Context<Data>) -> ReconcilerAction {
  warn!("Reconcile failed: {:?}", error);
  ReconcilerAction {
    requeue_after: Some(Duration::from_secs(360)),
  }
}

/// Builds controller to manage StaticSite resources
pub async fn build_controller() -> BoxFuture<'static, ()> {
  let client = Client::try_default().await.expect("Failed to create client");
  let context = Context::new(Data { client: client.clone() });

  let static_sites: Api<StaticSite> = Api::all(client);
  let _check = static_sites
    .list(&ListParams::default().limit(1))
    .await
    .expect("CRD is not installed");

  Controller::new(static_sites, ListParams::default())
    .run(reconcile, error_policy, context)
    .filter_map(|x| async move { std::result::Result::ok(x) })
    .for_each(|_| futures::future::ready(()))
    .boxed()
}
