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
pub struct StaticSiteSpec {
  pub git: String,
  pub branch: Option<String>,
  pub tag_blob: Option<String>,
  pub git_credentials: Option<String>,
  pub namespace: Option<String>,
  pub multi_site: Option<bool>,
  pub oci_repo: String,
  pub oci_credentials: Option<String>,
  pub ingress: Option<IngressConfig>,
}

/// Optioanl Ingress Configuration
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub struct IngressConfig {
  pub ingress_class: Option<String>,
  pub annotations: Option<String>,
}
