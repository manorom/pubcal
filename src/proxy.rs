use hyper::header::{self, HeaderMap, HeaderValue};
use hyper::{Body, Request, Response, Uri};
use std::net::IpAddr;

use crate::config::Calendar;

const HOP_HEADERS: [&str; 8] = [
    "Connection",
    "Keep-Alive",
    "Proxy-Authentication",
    "Proxy-Authorization",
    "Te",
    "Trailers",
    "Transfer-Encoding",
    "Upgrade",
];

fn is_hop_header(name: &str) -> bool {
    HOP_HEADERS.iter().any(|h| h.eq_ignore_ascii_case(name))
}

fn remove_hop_headers(headers: &HeaderMap<HeaderValue>) -> HeaderMap<HeaderValue> {
    headers
        .iter()
        .filter_map(|(name, val)| {
            if !is_hop_header(name.as_str()) {
                Some((name.clone(), val.clone()))
            } else {
                None
            }
        })
        .collect::<HeaderMap<HeaderValue>>()
}

pub fn request(
    req: &Request<Body>,
    client_ip: &IpAddr,
    calendar: &Calendar,
    base_url: Uri,
    credential: &str,
) -> anyhow::Result<Request<Body>> {
    let uri = calendar.collection_uri(base_url);
    let mut proxy_req = Request::get(uri).body(Body::empty())?;
    *proxy_req.headers_mut() = remove_hop_headers(req.headers());
    proxy_req.headers_mut().insert(
        header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Basic {}", credential))?,
    );
    // TODO: forwarded header must be augmented
    proxy_req
        .headers_mut()
        .entry("X-Forwarded-For")
        .or_insert(client_ip.to_string().parse()?);

    Ok(proxy_req)
}

pub fn response<B>(mut resp: Response<B>) -> anyhow::Result<Response<B>> {
    *resp.headers_mut() = remove_hop_headers(resp.headers());

    Ok(resp)
}
