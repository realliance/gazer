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
  pub git_credentials: Option<GitCredentials>,
  pub namespace: Option<String>,
  pub multi_site: Option<bool>,
  pub oci_repo: String,
  pub oci_credentials: Option<String>,
  pub ingress: Option<IngressConfig>,
}

/// Optional Ingress Configuration
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")] 
pub struct GitCredentials {
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
