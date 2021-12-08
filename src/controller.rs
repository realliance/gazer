//! Handles controller operations

use std::path::{Path, PathBuf};

use futures::future::BoxFuture;
use futures::{FutureExt, StreamExt};
use git2::{Direction, RemoteHead, Repository, RemoteCallbacks, Cred};
use kube::api::ListParams;
use kube::runtime::controller::{Context, ReconcilerAction};
use kube::runtime::Controller;
use kube::{Api, Client, ResourceExt};
use semver::Version;
use thiserror::Error;
use tokio::time::Duration;
use tracing::{info, warn};

use crate::build::{build_job, delete_job, get_job_status};
use crate::StaticSite;
use crate::crd::Credentials;

#[derive(Error, Debug)]
pub enum ControllerError {
  #[error("Kube Api Error: {0}")]
  KubeError(kube::Error),
  #[error("ReconcileError: {0}")]
  ReconcileError(String),
}

#[derive(PartialEq, Eq, Debug, Clone)]
enum GitRefType {
  Branch,
  Tag,
  Pull,
  HEAD,
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

#[derive(Debug, Clone)]
struct GitRef {
  pub ref_type: GitRefType,
  pub full_ref: String,
  pub oid: String,
  pub name: String,
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
      name: GitRef::extract_name(self.name().to_string()),
    }
  }
}

fn determine_selected_ref(site: &StaticSite, ref_list: Vec<GitRef>) -> Option<(String, String)> {
  // Use Semver
  if site.spec.use_semver.is_some() && site.spec.use_semver.unwrap() {
    // Get tags
    let tags: Vec<_> = ref_list.iter().filter(|&x| x.ref_type == GitRefType::Tag).collect();
    // Prepare tags for semver parsing (remove v prefixes if they exist)
    let semver_pairs: Vec<_> = tags.iter().map(|&x| (x.name.replace("v", ""), x.clone())).collect();
    // Get valid semver targets
    let mut semver_tags: Vec<(Version, GitRef)> = semver_pairs
      .iter()
      .map(|(tag, g)| (Version::parse(tag.as_str()), g.clone()))
      .filter(|(semver, _)| semver.is_ok())
      .map(|(valid_semver, g)| (valid_semver.unwrap(), g))
      .collect();
    // Find latest semver tag
    semver_tags.sort_by(|(a, _), (b, _)| b.cmp(a));
    if let Some((_, git_ref)) = semver_tags.first() {
      Some((git_ref.full_ref.clone(), git_ref.name.clone()))
    } else {
      None
    }
  // Use branch
  } else if let Some(branch) = site.spec.branch.clone() {
    if let Some(git_ref) = ref_list.iter().find(|&x| x.name == branch) {
      Some((git_ref.full_ref.clone(), git_ref.name.clone()))
    } else {
      None
    }
  // Use HEAD
  } else {
    if let Some(git_ref) = ref_list.iter().find(|&x| x.ref_type == GitRefType::HEAD) {
      Some((git_ref.full_ref.clone(), git_ref.oid.clone()))
    } else {
      None
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

fn get_targets(path: &PathBuf, git: String, credentials: Option<Credentials>) -> Result<Vec<GitRef>, ControllerError> {
  let repo_result = Repository::init(path.clone());
  if repo_result.is_err() {
    return Err(ControllerError::ReconcileError(format!(
      "Failed to init repo at {:?}",
      path
    )));
  }
  let repo = repo_result.unwrap();
  let remote_result = repo.remote_anonymous(&git);
  if remote_result.is_err() {
    return Err(ControllerError::ReconcileError(format!(
      "Failed to find remote {}",
      git.clone()
    )));
  }
  let mut remote = remote_result.unwrap();

  let mut remote_callbacks = RemoteCallbacks::new();
  
  if let Some(creds) = credentials {
    if let Some(plaintext) = creds.plaintext {
      remote_callbacks.credentials(move |_, _, _| {
        Cred::userpass_plaintext(&plaintext.username, &plaintext.password)
      });
    } else if let Some(_) = creds.from_secret {

    } else {
      return Err(ControllerError::ReconcileError("Provided an invalid crential entry!".to_string()));
    }
  }

  let conn = remote.connect_auth(Direction::Fetch, Some(remote_callbacks), None);

  match conn {
    Ok(connection) => {
      Ok(
        connection.list().unwrap()
          .iter()
          .map(|x| x.into())
          .filter(|x: &GitRef| x.ref_type != GitRefType::Pull)
          .collect::<Vec<GitRef>>().clone(),
      )
    },
    Err(err) => Err(ControllerError::ReconcileError(err.to_string()))
  }
}

async fn reconcile(site: StaticSite, ctx: Context<Data>) -> Result<ReconcilerAction, ControllerError> {
  let ns = ResourceExt::namespace(&site).unwrap_or("global".to_string());
  let name = ResourceExt::name(&site);
  let client = ctx.get_ref().client.clone();

  // Check if job already exists
  if let Ok(job) = get_job_status(client.clone(), ns.clone(), name.clone()).await {
    // Check if complete
    if job.status.unwrap().completion_time.is_some() {
      if let Err(err) = delete_job(client, ns.clone(), name.clone()).await {
        return Err(ControllerError::KubeError(err));
      } else {
        info!("{} reconciled, job completed", name);
        return Ok(ReconcilerAction {
          requeue_after: Some(Duration::from_secs(360)),
        });
      }
    }

    // Otherwise do another short requeue
    info!("{} reconciled, waiting on job", name);
    return Ok(ReconcilerAction {
      requeue_after: Some(Duration::from_secs(60)),
    });
  }

  // If not, check for changes and build job
  let path = Path::new("/tmp").join(format!("{}_{}", ns, name));

  let targets = get_targets(&path, site.spec.git.clone(), site.spec.git_credentials.clone());

  if let Err(err) = targets {
    return Err(err);
  }

  if let Some((selected_ref, image_tag)) = determine_selected_ref(&site, targets.unwrap()) {
    info!(
      "Selected Ref for {}: {} with image tag {}",
      name, selected_ref, image_tag
    );
    let build_result = build_job(client, ns, name.clone(), selected_ref, site.spec.clone(), image_tag).await;

    // Report any errors while building job
    if let Err(err) = build_result {
      return Err(err);
    }

    // Requeue earlier to check on running job
    return Ok(ReconcilerAction {
      requeue_after: Some(Duration::from_secs(60)),
    });
  } else {
    return Err(ControllerError::ReconcileError(
      "Could not find valid ref to watch!".to_string(),
    ));
  }
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
