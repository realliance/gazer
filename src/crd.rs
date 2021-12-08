//! Contains definitions for CRD

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Spec for [StaticSite] CRD
#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
  group = "realliance.net",
  version = "v1",
  kind = "StaticSite",
  namespaced,
  singular = "site",
  plural = "sites"
)]
#[serde(rename_all = "camelCase")] 
pub struct StaticSiteSpec {
  pub git: String,
  pub branch: Option<String>,
  pub use_semver: Option<bool>,
  pub git_credentials: Option<Credentials>,
  pub namespace: Option<String>,
  pub multi_site: Option<bool>,
  pub oci_repo: OciRepo,
  pub oci_credentials: Option<Credentials>,
  pub ingress: Option<IngressConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")] 
pub enum OciRepoProvider {
  Docker,
  Quay
}

impl OciRepoProvider {
  pub fn get_auth_url(&self) -> String {
    match self {
      Self::Docker => "https://index.docker.io/v1/".to_string(),
      Self::Quay => "quay.io".to_string()
    }
  }

  pub fn get_push_url(&self) -> String {
    match self {
      Self::Docker => "docker.io".to_string(),
      Self::Quay => "quay.io".to_string()
    }
  }
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")] 
pub struct CustomOciDestination {
  pub auth_url: String,
  pub push_url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")] 
pub struct OciRepo {
  pub provider: Option<OciRepoProvider>,
  pub custom: Option<CustomOciDestination>,
  pub repo: String
}

/// Optional Ingress Configuration
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")] 
pub struct Credentials {
  pub plaintext: Option<PlainTextCredentials>,
  pub from_secret: Option<FromSecret>
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")] 
pub struct PlainTextCredentials {
  pub username: String,
  pub password: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")] 
pub struct FromSecret {
  pub secret_name: String,
  pub username_entry: String,
  pub password_entry: String,
}

/// Optional Ingress Configuration
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")] 
pub struct IngressConfig {
  pub ingress_class: Option<String>,
  pub annotations: Option<String>,
}
