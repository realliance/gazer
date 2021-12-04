use k8s_openapi::api::batch::v1::Job;
use kube::api::{DeleteParams, PostParams, PropagationPolicy};
use kube::{Api, Client};

use crate::controller::ControllerError;

fn get_job_name(name: String) -> String {
  format!("gazer-build-{}", name)
}

pub async fn build_job(
  client: Client,
  namespace: String,
  name: String,
  git: String,
  git_ref: String,
  _: String,
) -> Result<(), ControllerError> {
  // Remote any http prefixes (common in github cases)
  let prep_git = git.replace("https://", "").replace("http://", "");

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
              "--no-push"
            ],
          }],
          "restartPolicy": "Never",
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
