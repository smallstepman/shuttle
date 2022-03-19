use std::convert::Infallible;
use std::io;
use std::sync::Arc;

use hyper::body::Body;
use hyper::server::conn::AddrStream;
use hyper::service::{
    make_service_fn,
    service_fn
};
use hyper::{
    Request,
    Response,
    StatusCode
};

use shuttle_common::DeploymentMeta;

use crate::service::GatewayService;
use crate::{ProjectName, CLIENT_PORT, Refresh};

const SHUTTLEAPP_SUFFIX: &'static str = ".shuttleapp.rs";

pub async fn serve_proxy(service: Arc<GatewayService>) -> io::Result<()> {
    let proxy_make_service = make_service_fn(move |addr: &AddrStream| {
        let remote_addr = addr.remote_addr();
        let shared_service = Arc::clone(&service);
        async move {
            Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                let shared_service = Arc::clone(&shared_service);
                async move {
                    let resp = if let Some(project_host) = req
                        .headers()
                        .get("Host")
                        .map(|head| head.to_str().unwrap())
                        .and_then(|host| {
                            host.strip_suffix(".")
                                .unwrap_or(host)
                                .strip_suffix(SHUTTLEAPP_SUFFIX)
                        }) {
                        let project_name: ProjectName = project_host.parse().unwrap(); // TODO invalid project
                        let project = shared_service.find_project(&project_name).await.unwrap(); // TODO project not found
                        let project = project.refresh(&shared_service.context()).await.unwrap();
                        let target = project.target_ip().unwrap().unwrap(); // TODO project not ready
                        let port = project.active_port().unwrap().unwrap(); // TODO no deployed service
                        hyper_reverse_proxy::call(
                            remote_addr.ip(),
                            &format!("http://{}:{}", target, port),
                            req
                        )
                        .await
                        .unwrap()
                    } else {
                        Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .body(Body::empty())
                            .unwrap()
                    };
                    Ok::<_, Infallible>(resp)
                }
            }))
        }
    });

    hyper::Server::bind(&format!("0.0.0.0:{}", CLIENT_PORT).parse().unwrap())
        .serve(proxy_make_service)
        .await
        .unwrap();

    Ok(())
}
