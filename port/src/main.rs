#[macro_use]
extern crate rocket;

#[macro_use]
extern crate log;

mod args;
mod build;
mod database;
mod deployment;
mod factory;
mod router;

use deployment::MAX_DEPLOYS;
use factory::ShuttleFactory;
use rocket::serde::json::Json;
use rocket::{tokio, Build, Data, Rocket, State};
use shuttle_common::project::ProjectName;
use shuttle_common::{DeploymentApiError, DeploymentMeta, Port};
use std::net::IpAddr;
use std::sync::Arc;
use structopt::StructOpt;
use uuid::Uuid;

use crate::args::Args;
use crate::build::{BuildSystem, FsBuildSystem};
use crate::deployment::DeploymentSystem;
use rocket::request::FromRequest;
use shuttle_service::rocket::Request;
use shuttle_service::rocket::request::Outcome;
use rocket::http::Status;

type ApiResult<T, E> = Result<Json<T>, E>;

struct ControlGuard;

#[async_trait]
impl<'r> FromRequest<'r> for ControlGuard {
    type Error = &'static str;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let state: &'r State<ApiState> = rocket::outcome::try_outcome!(request.guard().await.map_failure(|(s, _)| {
            (s, "a ControlGuard was used in a route without a state")
        }));
        if let Some(auth) = request.headers().get_one("Authorization") {
            let parts: Vec<&str> = auth.split(' ').collect();
            if parts.len() != 2 {
                return Outcome::Failure((Status::Unauthorized, "a request to the control plane was blocked: bad request"));
            }
            // unwrap ok because of explicit check above
            let secret = *parts.get(1).unwrap();
            if state.deployment_manager.is_authorized(secret) {
                Outcome::Success(ControlGuard)
            } else {
                Outcome::Failure((Status::Unauthorized, "a request to the control plane was blocked: wrong key"))
            }
        } else {
            return Outcome::Failure((Status::Unauthorized, "a request to the control plane was blocked: no auth"));
        }
    }
}

/// Status API to be used to check if the service is alive
#[get("/status")]
async fn status() {}

#[get("/<service>/deployments/<id>")]
async fn get_deployment(
    state: &State<ApiState>,
    service: String,
    id: Uuid,
    _guard: ControlGuard,
) -> ApiResult<DeploymentMeta, DeploymentApiError> {
    let deployment = state.deployment_manager.get_deployment(&id).await?;
    Ok(Json(deployment))
}

#[delete("/<service>/deployments/<id>")]
async fn delete_deployment(
    state: &State<ApiState>,
    service: String,
    id: Uuid,
    _guard: ControlGuard,
) -> ApiResult<DeploymentMeta, DeploymentApiError> {
    // TODO why twice?
    let _deployment = state.deployment_manager.get_deployment(&id).await?;
    let deployment = state.deployment_manager.kill_deployment(&id).await?;
    Ok(Json(deployment))
}

#[get("/<service>/deployments")]
async fn get_project(
    state: &State<ApiState>,
    service: ProjectName,
    _guard: ControlGuard,
) -> ApiResult<DeploymentMeta, DeploymentApiError> {
    let deployment = state
        .deployment_manager
        .get_deployment_for_project(&service)
        .await?;

    Ok(Json(deployment))
}

#[delete("/<service>")]
async fn delete_project(
    state: &State<ApiState>,
    service: ProjectName,
    _guard: ControlGuard,
) -> ApiResult<DeploymentMeta, DeploymentApiError> {
    let deployment = state
        .deployment_manager
        .kill_deployment_for_project(&service)
        .await?;
    Ok(Json(deployment))
}

#[post("/<service>", data = "<crate_file>")]
async fn create_project(
    state: &State<ApiState>,
    crate_file: Data<'_>,
    service: ProjectName,
    _guard: ControlGuard,
) -> ApiResult<DeploymentMeta, DeploymentApiError> {
    let deployment = state
        .deployment_manager
        .deploy(crate_file, service)
        .await?;
    Ok(Json(deployment))
}

struct ApiState {
    deployment_manager: Arc<DeploymentSystem>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .max_blocking_threads(MAX_DEPLOYS)
        .build()
        .unwrap()
        .block_on(async {
            rocket().await.launch().await?;

            Ok(())
        })
}

//noinspection ALL
async fn rocket() -> Rocket<Build> {
    env_logger::Builder::new()
        .filter_module("rocket", log::LevelFilter::Warn)
        .filter_module("_", log::LevelFilter::Warn)
        .filter_module("api", log::LevelFilter::Debug)
        .init();

    let args: Args = Args::from_args();
    let build_system = FsBuildSystem::initialise(args.path).unwrap();
    let deployment_manager = Arc::new(DeploymentSystem::new(Box::new(build_system)).await);

    let state = ApiState { deployment_manager };

    let config = rocket::Config {
        address: args.bind_addr,
        port: args.bind_port,
        ..Default::default()
    };
    rocket::custom(config)
        .mount(
            "/services",
            routes![
                delete_deployment,
                get_deployment,
                delete_project,
                create_project,
                get_project,
            ],
        )
        .mount("/", routes![status])
        .manage(state)
}