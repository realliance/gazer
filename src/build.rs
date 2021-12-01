use k8s_openapi::api::core::v1::Pod;
use kube::api::{PostParams, ListParams};
use kube::core::WatchEvent;
use kube::runtime::wait::delete::Error;
use kube::{Api, Client};
use tracing::info;


pub async fn get_build_pod(client: Client, namespace: String) -> Result<Api<Pod>, Error> {
  let p: Pod = serde_json::from_value(serde_json::json!({
    "apiVersion": "v1",
    "kind": "Pod",
    "metadata": { "name": "build" },
    "spec": {
      "containers": [{
        "name": "build",
        "image": "alpine",
        // Do nothing
        "command": ["tail", "-f", "/dev/null"],
      }],
    }
  })).unwrap();

  let pods: Api<Pod> = Api::namespaced(client, &namespace);
  pods.create(&PostParams::default(), &p).await.unwrap();

  // https://github.com/kube-rs/kube-rs/blob/6537cf006dbdd2a2f958d958d9fa916362581dca/examples/pod_exec.rs#L40
  let lp = ListParams::default().fields("metadata.name=example").timeout(10);
  let mut stream = pods.watch(&lp, "0").await?.boxed();
  while let Some(status) = stream.try_next().await? {
    match status {
      WatchEvent::Added(o) => {
        info!("Added {}", o.name());
      }
      WatchEvent::Modified(o) => {
        let s = o.status.as_ref().expect("status exists on pod");
        if s.phase.clone().unwrap_or_default() == "Running" {
          info!("Ready to attach to {}", o.name());
          break;
        }
      }
      _ => {}
    }
  }

  Ok(pods)
}
