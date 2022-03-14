use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server};
use std::convert::Infallible;
use std::env;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

mod config;
mod proxy;

use config::Config;

type Client = hyper::Client<hyper::client::HttpConnector>;

async fn handle(
    req: &Request<Body>,
    config: &Config,
    client: &Client,
    client_ip: &IpAddr,
) -> anyhow::Result<Response<Body>> {
    if *req.method() != Method::GET {
        log::info!("Request: {} {} -> 405", req.method(), req.uri());
        return Ok(Response::builder().status(405).body(Body::empty()).unwrap());
    }

    let (calendar, credential) = if let Some(c) = config.match_request(req.uri()) {
        c
    } else {
        log::info!("Request: {} {} -> 404", req.method(), req.uri());
        return Ok(Response::builder().status(404).body(Body::empty()).unwrap());
    };

    let proxied_req = proxy::request(
        req,
        client_ip,
        &calendar,
        config.server.upstream_base_url.clone(),
        credential,
    )?;
    let response = client.request(proxied_req).await?;

    log::info!("Request: {} {} -> collection {} {}", req.method(), req.uri(), calendar.collection_id, response.status());

    proxy::response(response)
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let config_file_path = match env::var("PUBCAL_CONFIG") {
        Ok(var) => var,
        Err(env::VarError::NotPresent) => "pubcal.toml".to_string(),
        Err(e) => panic!("{}", e),
    };
    log::info!("Reading configuration file {}", config_file_path);
    let config = Arc::new(Config::load(&config_file_path).expect("Could not load config"));

    let bind_addr = SocketAddr::new(config.server.bind_addr, config.server.bind_port);

    let client = Arc::new(Client::new());

    let make_svc = make_service_fn(|socket: &AddrStream| {
        let remote_addr = socket.remote_addr();
        let config = config.clone();
        let client = client.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                let client = client.clone();
                let config = config.clone();
                async move {
                    match handle(&req, &config, &client, &remote_addr.ip()).await {
                        Ok(r) => Ok::<_, Infallible>(r),
                        Err(e) => {
                            log::error!("Proxy Error: {}", e);
                            log::info!("Request: {} {} -> 500", req.method(), req.uri());
                            Ok::<_, Infallible>(
                                Response::builder().status(500).body(Body::empty()).unwrap(),
                            )
                        }
                    }
                }
            }))
        }
    });

    let server = Server::bind(&bind_addr).serve(make_svc);

    log::info!("Starting up server");
    // And run forever...
    if let Err(e) = server.await {
        log::error!("server error: {}", e);
    }
}
