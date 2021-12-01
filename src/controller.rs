//! Handles controller operations

use std::path::Path;

use futures::future::BoxFuture;
use futures::{FutureExt, StreamExt};
use git2::{Repository, Direction, RemoteHead};
use kube::api::ListParams;
use kube::runtime::controller::{Context, ReconcilerAction};
use kube::runtime::Controller;
use kube::{Api, Client, ResourceExt};
use tokio::time::Duration;
use tracing::{info, warn};
use thiserror::Error;

use crate::StaticSite;

#[derive(Error, Debug)]
enum ControllerError {
  #[error("Kube Api Error: {0}")]
  KubeError(kube::Error),
  #[error("ReconcileError: {0}")]
  ReconcileError(String)
}

#[derive(PartialEq, Eq, Debug)]
enum GitRefType {
  Branch,
  Tag,
  Pull,
  HEAD
}

impl Into<GitRefType> for &str {
  fn into(self) -> GitRefType {
    if self.contains("refs/heads") {
      GitRefType::Branch
    } else if self.contains("refs/pull") {
      GitRefType::Pull
    } else if self.contains("refs/tags") {
      GitRefType::Tag
    } else if self.contains("HEAD") {
      GitRefType::HEAD
    } else {
      panic!("{} is an invalid git ref type!", self)
    }
  }
}

#[derive(Debug)]
struct GitRef {
  pub ref_type: GitRefType,
  pub full_ref: String,
  pub oid: String,
  pub name: String
}

impl GitRef {
  fn extract_name(full_ref: String) -> String {
    let mut slash_sections: Vec<_> = full_ref.split("/").collect();
    if slash_sections.len() == 1 {
      return full_ref;
    }
    let name_vec: Vec<_> = slash_sections.drain(2..).collect();
    name_vec.join("/")
  }
}

impl<'a> Into<GitRef> for &RemoteHead<'a> {
  fn into(self) -> GitRef {
    GitRef {
      ref_type: self.name().into(),
      full_ref: self.name().to_string(),
      oid: self.oid().to_string(),
      name: GitRef::extract_name(self.name().to_string())
    }
  }
}

/// Context Data
#[derive(Clone)]
struct Data {
  /// Client to be used during reconciliation
  #[allow(dead_code)]
  client: Client,
}

async fn reconcile(site: StaticSite, ctx: Context<Data>) -> Result<ReconcilerAction, ControllerError> {
  let _client = ctx.get_ref().client.clone();
  let ns = ResourceExt::namespace(&site).unwrap_or("global".to_string());
  let name = ResourceExt::name(&site);
  let path = Path::new("/tmp").join(format!("{}_{}", ns, name));

  let repo_result = Repository::init(path.clone());
  if repo_result.is_err() {
    return Err(ControllerError::ReconcileError(format!("Failed to init repo at {:?}", path)));
  }
  let repo = repo_result.unwrap();
  let remote_result = repo.remote_anonymous(&site.spec.git);
  if remote_result.is_err() {
    return Err(ControllerError::ReconcileError(format!("Failed to find remote {}", site.spec.git.clone())));
  }
  let mut remote = remote_result.unwrap();
  let connection = remote.connect_auth(Direction::Fetch, None, None).unwrap();

  // If so, clone into /tmp/{proj}

  // Build and push cloned with kaniko

  // Clean up tmp

  let remote_head = connection.list().unwrap();
  let valid_update_targets: Vec<GitRef> = remote_head.iter().map(|x| x.into()).filter(|x: &GitRef| x.ref_type == GitRefType::Branch || x.ref_type == GitRefType::Tag).collect();
  for target in valid_update_targets {
    info!("{}: {:?}", name, target);
  }

  info!("{} Reconciled", name);

  Ok(ReconcilerAction {
    requeue_after: Some(Duration::from_secs(360)),
  })
}

fn error_policy(error: &ControllerError, _: Context<Data>) -> ReconcilerAction {
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
