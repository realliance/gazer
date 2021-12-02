use std::io::Write;

use futures::{stream, StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use kube::api::{AttachParams, AttachedProcess, DeleteParams, ListParams, PostParams};
use kube::core::WatchEvent;
use kube::{Api, Client, ResourceExt};
use tracing::info;

use crate::controller::ControllerError;

pub async fn build_container(
  client: Client,
  namespace: String,
  git: String,
  git_ref: String,
  _: String,
) -> Result<(), ControllerError> {
  let prep_git = git.replace("https://", "").replace("http://", "");
  let context = vec![
    "--context=".to_string(),
    "git://".to_string(),
    prep_git,
    "#".to_string(),
    git_ref,
  ]
  .join("");

  let p: Pod = serde_json::from_value(serde_json::json!({
    "apiVersion": "v1",
    "kind": "Pod",
    "metadata": { "name": "gazer-build" },
    "spec": {
      "containers": [{
        "name": "kaniko",
        "image": "gcr.io/kaniko-project/executor:latest",
        "args": [
          context,
          "--no-push"
        ]
      }],
    }
  }))
  .unwrap();

  let pods: Api<Pod> = Api::namespaced(client, &namespace);
  pods.create(&PostParams::default(), &p).await.unwrap();

  // https://github.com/kube-rs/kube-rs/blob/6537cf006dbdd2a2f958d958d9fa916362581dca/examples/pod_exec.rs#L40
  let lp = ListParams::default().fields("metadata.name=gazer-build").timeout(10);
  let mut stream = pods.watch(&lp, "0").await.unwrap().boxed();
  while let Some(status) = stream.try_next().await.unwrap() {
    match status {
      WatchEvent::Added(o) => {
        info!("Added {}", o.name());
      },
      WatchEvent::Modified(o) => {
        let s = o.status.as_ref().expect("status exists on pod");
        if s.phase.clone().unwrap_or_default() == "Running" {
          info!("Ready to attach to {}", o.name());
          break;
        }
      },
      _ => {},
    }
  }

  let attached = pods.attach("gazer-build", &AttachParams::default()).await.unwrap();
  combined_output(attached).await;

  pods.delete("gazer-build", &DeleteParams::default()).await.unwrap();

  Ok(())
}

// https://github.com/kube-rs/kube-rs/blob/6537cf006dbdd2a2f958d958d9fa916362581dca/examples/pod_attach.rs#L102
async fn combined_output(mut attached: AttachedProcess) {
  let stdout = tokio_util::io::ReaderStream::new(attached.stdout().unwrap());
  let stderr = tokio_util::io::ReaderStream::new(attached.stderr().unwrap());
  let outputs = stream::select(stdout, stderr).for_each(|res| async {
    if let Ok(bytes) = res {
      let out = std::io::stdout();
      out.lock().write_all(&bytes).unwrap();
    }
  });
  outputs.await;

  if let Some(status) = attached.await {
    info!("{:?}", status);
  }
}
