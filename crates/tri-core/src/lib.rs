//! # tri-core
//!
//! Core Railway service management operations: deploy, kill, rotate, snapshot, fleet_list.
//!
//! This crate provides the stable public API for managing Railway services in the IGLA project.

use trios_railway_core::{mutations as M, queries as Q};

pub use trios_railway_core::{Client, DeployId, EnvironmentId, ProjectId, ServiceId};

/// Service deployment configuration.
#[derive(Debug, Clone)]
pub struct DeployConfig {
    /// Project ID to deploy to.
    pub project_id: ProjectId,
    /// Environment ID to deploy to.
    pub environment_id: EnvironmentId,
    /// Service name.
    pub name: String,
    /// Docker image to deploy.
    pub image: String,
    /// Environment variables to set.
    pub vars: Vec<(String, String)>,
    /// Optional existing service ID to reuse instead of creating new.
    pub existing_service_id: Option<ServiceId>,
}

/// Result of a deployment operation.
#[derive(Debug, Clone)]
pub struct DeployResult {
    /// The service ID that was created or reused.
    pub service_id: ServiceId,
    /// The deploy ID that was triggered.
    pub deploy_id: DeployId,
}

/// Snapshot result containing fleet information.
#[derive(Debug, Clone)]
pub struct FleetSnapshot {
    /// Project ID.
    pub project_id: String,
    /// Project name.
    pub project_name: String,
    /// Services in the fleet.
    pub services: Vec<ServiceInfo>,
}

/// Information about a single service.
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    /// Service ID.
    pub id: String,
    /// Service name.
    pub name: String,
    /// Creation timestamp.
    pub created_at: String,
}

/// Deploy a new service or redeploy an existing one.
///
/// # Arguments
///
/// * `client` - Railway API client
/// * `config` - Deployment configuration
///
/// # Returns
///
/// Returns `DeployResult` with the service ID and deploy ID.
///
/// # Errors
///
/// Returns an error if the Railway API call fails.
pub async fn deploy(client: &Client, config: DeployConfig) -> anyhow::Result<DeployResult> {
    let DeployConfig {
        project_id,
        environment_id,
        name,
        image,
        vars,
        existing_service_id,
    } = config;

    let service_id = if let Some(eid) = existing_service_id {
        eid
    } else {
        let created = M::service_create(client, &project_id, &name).await?;
        tracing::info!("created service {} ({})", created.name, created.id);
        ServiceId::from(created.id)
    };

    M::service_instance_set_image(client, &service_id, &environment_id, &image).await?;
    tracing::info!("set image: {image}");

    for (key, value) in &vars {
        M::variable_upsert(
            client,
            &project_id,
            &environment_id,
            &service_id,
            key,
            value,
        )
        .await?;
        tracing::info!("var: {key}=<{}>", value.len());
    }

    let deploy_id = M::service_redeploy(client, &service_id, &environment_id).await?;
    tracing::info!("redeploy triggered: {deploy_id}");

    Ok(DeployResult {
        service_id,
        deploy_id,
    })
}

/// Permanently delete a service.
///
/// # Arguments
///
/// * `client` - Railway API client
/// * `service_id` - Service ID to delete
///
/// # Errors
///
/// Returns an error if the Railway API call fails.
pub async fn kill(client: &Client, service_id: &ServiceId) -> anyhow::Result<()> {
    M::service_delete(client, service_id).await?;
    tracing::info!("deleted service: {service_id}");
    Ok(())
}

/// Trigger a redeploy of an existing service.
///
/// # Arguments
///
/// * `client` - Railway API client
/// * `service_id` - Service ID to redeploy
/// * `environment_id` - Environment ID to redeploy in
///
/// # Returns
///
/// Returns the deploy ID that was triggered.
///
/// # Errors
///
/// Returns an error if the Railway API call fails.
pub async fn rotate(
    client: &Client,
    service_id: &ServiceId,
    environment_id: &EnvironmentId,
) -> anyhow::Result<DeployId> {
    let deploy_id = M::service_redeploy(client, service_id, environment_id).await?;
    tracing::info!("redeploy triggered: {deploy_id}");
    Ok(deploy_id)
}

/// Create a snapshot of the fleet services in a project.
///
/// # Arguments
///
/// * `client` - Railway API client
/// * `project_id` - Project ID to snapshot
///
/// # Returns
///
/// Returns `FleetSnapshot` with project and service information.
///
/// # Errors
///
/// Returns an error if the Railway API call fails.
pub async fn snapshot(
    client: &Client,
    project_id: &ProjectId,
) -> anyhow::Result<FleetSnapshot> {
    let pv = Q::project_view(client, project_id).await?;

    let services = pv
        .services()
        .into_iter()
        .map(|s| ServiceInfo {
            id: s.id.clone(),
            name: s.name.clone(),
            created_at: s.created_at.clone(),
        })
        .collect();

    Ok(FleetSnapshot {
        project_id: pv.id.clone(),
        project_name: pv.name.clone(),
        services,
    })
}

/// List all services in a project.
///
/// # Arguments
///
/// * `client` - Railway API client
/// * `project_id` - Project ID to list services from
///
/// # Returns
///
/// Returns a vector of `ServiceInfo` for all services in the project.
///
/// # Errors
///
/// Returns an error if the Railway API call fails.
pub async fn fleet_list(
    client: &Client,
    project_id: &ProjectId,
) -> anyhow::Result<Vec<ServiceInfo>> {
    let snapshot = snapshot(client, project_id).await?;
    Ok(snapshot.services)
}
