//! RW-03: typed mutations against the Railway GraphQL API.
//!
//! Mutations supported:
//! - `serviceCreate` (image-based, project-scoped)
//! - `variableUpsert` (one variable at a time, scoped to project + env + service)
//! - `serviceInstanceDeployV2` (trigger a redeploy on the latest source)
//! - `serviceInstanceUpdate` (set source.image so the next deploy pulls a new image)
//! - `serviceDelete`
//!
//! All mutations log a `R7` triplet via `RailwayHash::seal` at the call site
//! (callers must do that — this module returns plain ids).

use serde::Deserialize;
use serde_json::json;

use crate::hash::RailwayHash;
use crate::ids::{DeployId, EnvironmentId, ProjectId, ServiceId};
use crate::transport::{Client, ClientError};

pub const M_SERVICE_CREATE: &str = "mutation M($input: ServiceCreateInput!) {
  serviceCreate(input: $input) { id name projectId }
}";

pub const M_VARIABLE_UPSERT: &str = "mutation M($input: VariableUpsertInput!) {
  variableUpsert(input: $input)
}";

pub const M_DEPLOY_REDEPLOY: &str = "mutation M($serviceId: String!, $environmentId: String!) {
  serviceInstanceRedeploy(serviceId: $serviceId, environmentId: $environmentId)
}";

pub const M_SERVICE_INSTANCE_UPDATE: &str =
    "mutation M($serviceId: String!, $environmentId: String!, $input: ServiceInstanceUpdateInput!) {
  serviceInstanceUpdate(serviceId: $serviceId, environmentId: $environmentId, input: $input)
}";

pub const M_SERVICE_DELETE: &str = "mutation M($id: String!) {
  serviceDelete(id: $id)
}";

#[derive(Debug, Clone, Deserialize)]
pub struct CreatedService {
    pub id: String,
    pub name: String,
    #[serde(rename = "projectId")]
    pub project_id: String,
}

/// Create a new service in a project. The image is set on the service
/// instance via a follow-up `serviceInstanceUpdate` call (Railway splits
/// service vs service-instance config).
pub async fn service_create(
    client: &Client,
    project: &ProjectId,
    name: &str,
) -> Result<CreatedService, ClientError> {
    #[derive(Deserialize)]
    struct R {
        #[serde(rename = "serviceCreate")]
        service_create: CreatedService,
    }
    let vars = json!({
        "input": {
            "projectId": project.as_str(),
            "name": name,
        }
    });
    let r: R = client.query(M_SERVICE_CREATE, Some(vars)).await?;
    Ok(r.service_create)
}

/// Pin the image source on a service instance. The next redeploy will use it.
pub async fn service_instance_set_image(
    client: &Client,
    service: &ServiceId,
    env: &EnvironmentId,
    image: &str,
) -> Result<(), ClientError> {
    let vars = json!({
        "serviceId": service.as_str(),
        "environmentId": env.as_str(),
        "input": {
            "source": { "image": image }
        }
    });
    let _: serde_json::Value = client.query(M_SERVICE_INSTANCE_UPDATE, Some(vars)).await?;
    Ok(())
}

/// Upsert a single environment variable for a service.
pub async fn variable_upsert(
    client: &Client,
    project: &ProjectId,
    env: &EnvironmentId,
    service: &ServiceId,
    name: &str,
    value: &str,
) -> Result<(), ClientError> {
    let vars = json!({
        "input": {
            "projectId": project.as_str(),
            "environmentId": env.as_str(),
            "serviceId": service.as_str(),
            "name": name,
            "value": value,
        }
    });
    let _: serde_json::Value = client.query(M_VARIABLE_UPSERT, Some(vars)).await?;
    Ok(())
}

/// Redeploy a service in an environment using the most recent source.
/// Returns the new deployment id.
pub async fn service_redeploy(
    client: &Client,
    service: &ServiceId,
    env: &EnvironmentId,
) -> Result<DeployId, ClientError> {
    #[derive(Deserialize)]
    struct R {
        #[serde(rename = "serviceInstanceRedeploy")]
        service_instance_redeploy: serde_json::Value,
    }
    let vars = json!({
        "serviceId": service.as_str(),
        "environmentId": env.as_str(),
    });
    let r: R = client.query(M_DEPLOY_REDEPLOY, Some(vars)).await?;
    let id = match r.service_instance_redeploy {
        serde_json::Value::String(s) => s,
        v => v.to_string(),
    };
    Ok(DeployId::new(id))
}

/// Permanently delete a service (and all its deployments).
pub async fn service_delete(client: &Client, service: &ServiceId) -> Result<(), ClientError> {
    let vars = json!({ "id": service.as_str() });
    let _: serde_json::Value = client.query(M_SERVICE_DELETE, Some(vars)).await?;
    Ok(())
}

impl Client {
    pub async fn deploy_service(
        &self,
        project: &ProjectId,
        env: &EnvironmentId,
        name: &str,
        image: &str,
    ) -> Result<(ServiceId, RailwayHash), ClientError> {
        let cs = service_create(self, project, name).await?;
        let sid = ServiceId::new(cs.id);
        service_instance_set_image(self, &sid, env, image).await?;
        let hash = RailwayHash::seal(
            "deploy_service",
            project,
            Some(&sid),
            &self.token_fingerprint(),
        );
        Ok((sid, hash))
    }

    pub async fn set_vars(
        &self,
        project: &ProjectId,
        env: &EnvironmentId,
        service: &ServiceId,
        vars: &[(String, String)],
    ) -> Result<RailwayHash, ClientError> {
        for (k, v) in vars {
            variable_upsert(self, project, env, service, k, v).await?;
        }
        let hash = RailwayHash::seal(
            "set_vars",
            project,
            Some(service),
            &self.token_fingerprint(),
        );
        Ok(hash)
    }

    pub async fn redeploy(
        &self,
        project: &ProjectId,
        service: &ServiceId,
        env: &EnvironmentId,
    ) -> Result<(DeployId, RailwayHash), ClientError> {
        let did = service_redeploy(self, service, env).await?;
        let hash = RailwayHash::seal(
            "redeploy",
            project,
            Some(service),
            &self.token_fingerprint(),
        );
        Ok((did, hash))
    }

    pub async fn stop(
        &self,
        project: &ProjectId,
        service: &ServiceId,
        _env: &EnvironmentId,
    ) -> Result<RailwayHash, ClientError> {
        service_delete(self, service).await?;
        let hash = RailwayHash::seal(
            "stop",
            project,
            Some(service),
            &self.token_fingerprint(),
        );
        Ok(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutation_strings_present() {
        for m in [
            M_SERVICE_CREATE,
            M_VARIABLE_UPSERT,
            M_DEPLOY_REDEPLOY,
            M_SERVICE_INSTANCE_UPDATE,
            M_SERVICE_DELETE,
        ] {
            assert!(m.contains("mutation M("));
        }
    }

    #[test]
    fn created_service_parses() {
        let raw = serde_json::json!({"id":"s1","name":"trios-train-seed-43","projectId":"p"});
        let cs: CreatedService = serde_json::from_value(raw).unwrap();
        assert_eq!(cs.name, "trios-train-seed-43");
    }

    #[tokio::test]
    async fn deploy_service_returns_id_and_triplet() {
        use crate::transport::AuthMode;

        let mut server = mockito::Server::new_async().await;
        let url = format!("{}/graphql/v2", server.url());

        server
            .mock("POST", "/graphql/v2")
            .match_header("Authorization", "Bearer test-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "data": {
                        "serviceCreate": {"id":"svc-1","name":"test-svc","projectId":"proj-1"},
                        "serviceInstanceUpdate": true
                    },
                    "errors": []
                })
                .to_string(),
            )
            .expect(2)
            .create_async()
            .await;

        let client = Client::with_token_and_mode("test-token", AuthMode::Team)
            .unwrap()
            .with_endpoint(url);

        let (sid, hash) = client
            .deploy_service(
                &ProjectId::new("proj-1"),
                &EnvironmentId::new("env-1"),
                "test-svc",
                "ghcr.io/test:latest",
            )
            .await
            .unwrap();

        assert_eq!(sid.as_str(), "svc-1");
        assert!(hash.triplet().starts_with("RAIL=deploy_service @"));
        assert!(hash.triplet().contains("project=proj-1"));
    }

    #[tokio::test]
    async fn set_vars_seals_triplet() {
        use crate::transport::AuthMode;

        let mut server = mockito::Server::new_async().await;
        let url = format!("{}/graphql/v2", server.url());

        server
            .mock("POST", "/graphql/v2")
            .match_header("Authorization", "Bearer tok")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "data": { "variableUpsert": true },
                    "errors": []
                })
                .to_string(),
            )
            .expect(2)
            .create_async()
            .await;

        let client = Client::with_token_and_mode("tok", AuthMode::Team)
            .unwrap()
            .with_endpoint(url);

        let hash = client
            .set_vars(
                &ProjectId::new("p1"),
                &EnvironmentId::new("e1"),
                &ServiceId::new("s1"),
                &[("K1".into(), "V1".into()), ("K2".into(), "V2".into())],
            )
            .await
            .unwrap();

        assert!(hash.triplet().starts_with("RAIL=set_vars @"));
    }

    #[tokio::test]
    async fn redeploy_returns_deploy_id() {
        use crate::transport::AuthMode;

        let mut server = mockito::Server::new_async().await;
        let url = format!("{}/graphql/v2", server.url());

        server
            .mock("POST", "/graphql/v2")
            .match_header("Authorization", "Bearer tok")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "data": { "serviceInstanceRedeploy": "deploy-abc" },
                    "errors": []
                })
                .to_string(),
            )
            .create_async()
            .await;

        let client = Client::with_token_and_mode("tok", AuthMode::Team)
            .unwrap()
            .with_endpoint(url);

        let (did, hash) = client
            .redeploy(
                &ProjectId::new("p1"),
                &ServiceId::new("s1"),
                &EnvironmentId::new("e1"),
            )
            .await
            .unwrap();

        assert_eq!(did.as_str(), "deploy-abc");
        assert!(hash.triplet().contains("RAIL=redeploy"));
    }

    #[tokio::test]
    async fn stop_seals_triplet() {
        use crate::transport::AuthMode;

        let mut server = mockito::Server::new_async().await;
        let url = format!("{}/graphql/v2", server.url());

        server
            .mock("POST", "/graphql/v2")
            .match_header("Authorization", "Bearer tok")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "data": { "serviceDelete": true },
                    "errors": []
                })
                .to_string(),
            )
            .create_async()
            .await;

        let client = Client::with_token_and_mode("tok", AuthMode::Team)
            .unwrap()
            .with_endpoint(url);

        let hash = client
            .stop(
                &ProjectId::new("p1"),
                &ServiceId::new("s1"),
                &EnvironmentId::new("e1"),
            )
            .await
            .unwrap();

        assert!(hash.triplet().starts_with("RAIL=stop @"));
    }
}
