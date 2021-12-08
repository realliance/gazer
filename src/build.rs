use k8s_openapi::api::batch::v1::Job;
use k8s_openapi::api::core::v1::Secret;
use kube::api::{DeleteParams, PostParams, PropagationPolicy, PatchParams, Patch};
use kube::{Api, Client};
use tracing::info;

use crate::controller::ControllerError;
use crate::crd::{StaticSiteSpec, OciRepo};

fn get_job_name(name: String) -> String {
  format!("gazer-build-{}", name)
}

fn get_auth_url(spec: &OciRepo) -> Option<String> {
  if let Some(provider) = &spec.provider {
    Some(provider.get_auth_url())
  } else if let Some(custom) = &spec.custom {
    Some(custom.auth_url.clone())
  } else {
    None
  }
}

fn get_push_url(spec: &OciRepo, tag: String) -> Option<String> {
  if let Some(provider) = &spec.provider {
    Some(format!("{}/{}:{}", provider.get_push_url(), spec.repo, tag))
  } else if let Some(custom) = &spec.custom {
    Some(format!("{}/{}:{}", custom.push_url, spec.repo, tag))
  } else {
    // TODO, should warn of no push result
    None
  }
}

fn construct_config_json(spec: &StaticSiteSpec) -> serde_json::Value {
  if spec.oci_credentials.is_none() {
    return serde_json::json!({});
  }
  let credentials = spec.oci_credentials.as_ref().unwrap();
  if let Some(plaintext) = &credentials.plaintext {
    let auth = base64::encode(format!("{}:{}", plaintext.username, plaintext.password));
    if let Some(auth_url) = get_auth_url(&spec.oci_repo) {
      serde_json::json!({
        "auths": {
          auth_url: {
            "auth": auth
          }
        }
      })
    } else {
      serde_json::json!({})
    }
  } else if let Some(secret) = &credentials.from_secret {
    // TODO
    serde_json::json!({})
  } else {
    serde_json::json!({})
  }
}

pub async fn build_job(
  client: Client,
  namespace: String,
  name: String,
  git_ref: String,
  spec: StaticSiteSpec,
  tag: String,
) -> Result<(), ControllerError> {
  // Remote any http prefixes (common in github cases)
  let prep_git = spec.git.replace("https://", "").replace("http://", "");

  // Build kaniko context
  let context = vec![
    "--context=".to_string(),
    "git://".to_string(),
    prep_git,
    "#".to_string(),
    git_ref,
  ]
  .join("");

  let job_name = get_job_name(name.clone());
  let job_pod_name = format!("{}-worker", job_name);

  let config_json = construct_config_json(&spec);
  let encoded_config = base64::encode(config_json.to_string());


  let destination = if let Some(dest) = get_push_url(&spec.oci_repo, tag) {
    format!("--destination={}", dest)
  } else {
    "--no-push".to_string()
  };

  let secrets: Api<Secret> = Api::namespaced(client.clone(), &namespace);
  
  let secret: Secret = serde_json::from_value(serde_json::json!({
    "apiVersion": "v1",
    "kind": "Secret",
    "metadata": {
      "name": job_pod_name
    },
    "type": "kubernetes.io/dockerconfigjson",
    "data": {
      ".dockerconfigjson": encoded_config
    }
  })).unwrap();

  if secrets.get(&job_pod_name).await.is_ok() {
    secrets.delete(&job_pod_name, &DeleteParams::default()).await.unwrap();
  }

  secrets.create(&PostParams::default(), &secret).await.unwrap();

  let job: Job = serde_json::from_value(serde_json::json!({
    "apiVersion": "batch/v1",
    "kind": "Job",
    "metadata": { "name": job_name },
    "spec": {
      "template": {
        "metadata": {
          "name": job_pod_name
        },
        "spec": {
          "containers": [{
            "name": "kaniko",
            "image": "gcr.io/kaniko-project/executor:latest",
            "args": [
              context,
              destination
            ],
            "volumeMounts": [
              {
                "name": "docker-config",
                "mountPath": "/kaniko/.docker/"
              }
            ]
          }],
          "restartPolicy": "Never",
          "volumes": [
            {
              "name": "docker-config",
              "secret": {
                "secretName": job_pod_name,
                "items": [
                  {
                    "key": ".dockerconfigjson",
                    "path": "config.json"
                  }
                ]
              }
            }
          ]
        },
      },
    },
  }))
  .unwrap();

  let jobs: Api<Job> = Api::namespaced(client, &namespace);
  jobs.create(&PostParams::default(), &job).await.unwrap();

  Ok(())
}

pub async fn get_job_status(client: Client, namespace: String, name: String) -> Result<Job, kube::Error> {
  let jobs: Api<Job> = Api::namespaced(client, &namespace);

  jobs.get_status(&get_job_name(name)).await
}

pub async fn delete_job(client: Client, namespace: String, name: String) -> Result<(), kube::Error> {
  let jobs: Api<Job> = Api::namespaced(client, &namespace);
  if let Err(err) = jobs
    .delete(
      &get_job_name(name),
      &DeleteParams {
        dry_run: false,
        propagation_policy: Some(PropagationPolicy::Background),
        ..DeleteParams::default()
      },
    )
    .await
  {
    Err(err)
  } else {
    Ok(())
  }
}
