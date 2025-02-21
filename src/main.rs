use clap::Parser;
use std::convert::Infallible;
use std::net::SocketAddr;
use url::Url;
use warp::Filter;
use bytes::Bytes;
use reqwest::Client;
use log::{info, error, warn};
use colored::Colorize;
use serde_json;
use std::collections::BTreeMap;

/// Command-line options for the proxy.
#[derive(Parser, Debug, Clone)]
#[clap(author, version, about, long_about = None)]
struct Config {
    /// The target URL to which requests will be proxied.
    #[clap(long = "target-url", short = 't')]
    target_url: String,

    /// The API URL on which the proxy server will run.
    #[clap(long = "api-url", short = 'u')]
    api_url: String,

    /// (Optional) Flag to add CORS headers to responses.
    ///
    /// When enabled, the proxy will add headers such as
    /// Content-Type, Access-Control-Allow-Origin,
    /// Access-Control-Allow-Methods, and
    /// Access-Control-Allow-Headers.
    #[clap(long = "add-cors-headers", short = 'c')]
    add_cors_headers: bool,

    /// (Optional) Extra headers to add to responses.
    ///
    /// Format: "Header-Name: value". Can be repeated. For example:
    /// --extra-header='x-proxy-bob: yes'
    #[clap(long = "extra-header", short = 'e')]
    extra_headers: Vec<String>,
}

/// A filter to pass a clone of the configuration to each request.
fn with_config(config: Config) -> impl Filter<Extract = (Config,), Error = Infallible> + Clone {
    warp::any().map(move || config.clone())
}

/// A filter to pass a clone of the Reqwest client.
fn with_client(client: Client) -> impl Filter<Extract = (Client,), Error = Infallible> + Clone {
    warp::any().map(move || client.clone())
}

#[tokio::main]
async fn main() {
    // Initialize logging with a default level so logs are always visible.
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .init();

    // Parse command-line arguments.
    let config = Config::parse();
    info!("Starting proxy with config: {:?}", config);

    // Parse the API URL (where we will listen) to determine the host and port.
    let api_url_parsed = Url::parse(&config.api_url)
        .expect("Invalid api-url. Must be a valid URL like http://localhost:6969");
    let port = api_url_parsed.port_or_known_default().unwrap_or(6969);
    // Use the provided hostname if available (with "localhost" mapped to 127.0.0.1), otherwise default.
    let host = match api_url_parsed.host_str() {
        Some("localhost") => "127.0.0.1".to_string(),
        Some(h) => h.to_string(),
        None => "0.0.0.0".to_string(),
    };
    let socket_addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .expect("Unable to parse socket address");
    info!("Proxy server listening on {}", socket_addr);

    // Construct a Reqwest client.
    let client = Client::new();

    // Set up a warp filter that captures:
    //   • the HTTP method,
    //   • a clone of all headers,
    //   • the full request path,
    //   • the raw query string (or an empty string if none),
    //   • the full body as bytes,
    //   • plus our configuration and Reqwest client.
    let route = warp::any()
        .and(warp::method())
        .and(warp::header::headers_cloned())
        .and(warp::path::full())
        // Get the raw query string (default to empty string if missing).
        .and(warp::query::raw().or_else(|_| async { Ok::<(String,), Infallible>((String::new(),)) }))
        .and(warp::body::bytes())
        .and(with_config(config))
        .and(with_client(client))
        .and_then(proxy_handler);

    // Run the server.
    warp::serve(route).run(socket_addr).await;
}

/// The handler that proxies every request.
async fn proxy_handler(
    method: warp::http::Method,
    headers: warp::http::HeaderMap,
    full_path: warp::path::FullPath,
    query: String,
    body: Bytes,
    config: Config,
    client: Client,
) -> Result<impl warp::Reply, Infallible> {
    // Build complete URL string (including query, if provided)
    let complete_url = if query.is_empty() {
        full_path.as_str().to_string()
    } else {
        format!("{}?{}", full_path.as_str(), query)
    };

    // Fancy logging: display the HTTP verb (in bold blue) and complete request URL (in bold yellow)
    info!(
        "{} {}",
        "Incoming request:".bold().green(),
        format!("{} {}", method.to_string().bold().blue(), complete_url.bold().yellow())
    );

    // Pretty-print the request headers as pretty JSON.
    let headers_map: BTreeMap<_, _> = headers
        .iter()
        .map(|(k, v)| (k.as_str(), v.to_str().unwrap_or("")))
        .collect();
    info!("Request headers:\n{}", serde_json::to_string_pretty(&headers_map).unwrap());

    // Build the forwarding URL.
    let target_base = config.target_url.trim_end_matches('/');
    let mut new_url = format!("{}{}", target_base, full_path.as_str());
    if !query.is_empty() {
        new_url = format!("{}?{}", new_url, query);
    }
    info!("Forwarding to target URL: {}", new_url);

    // Create a new request to forward to the target using Reqwest.
    let mut req_builder = client.request(method.clone(), &new_url);

    // Copy every header from the incoming request except the "host" header.
    for (name, value) in headers.iter() {
        if name.as_str().to_lowercase() == "host" {
            continue;
        }
        req_builder = req_builder.header(name, value);
    }

    // Include the body if available.
    if !body.is_empty() {
        req_builder = req_builder.body(body);
    }

    // Send the request.
    let proxied_response = match req_builder.send().await {
        Ok(resp) => resp,
        Err(err) => {
            error!("Error forwarding request: {}", err);
            let reply = warp::http::Response::builder()
                .status(warp::http::StatusCode::BAD_GATEWAY)
                .header("content-type", "text/plain")
                .body(Bytes::from(format!("Error forwarding request: {}", err)))
                .expect("failed to build error response");
            return Ok(reply);
        }
    };

    // Retrieve the response status and headers.
    let status = proxied_response.status();
    let mut resp_headers = warp::http::HeaderMap::new();
    for (name, value) in proxied_response.headers().iter() {
        resp_headers.insert(name, value.clone());
    }

    // Get the response body as bytes.
    let resp_body = match proxied_response.bytes().await {
        Ok(b) => b,
        Err(err) => {
            error!("Error reading response body: {}", err);
            let reply = warp::http::Response::builder()
                .status(warp::http::StatusCode::BAD_GATEWAY)
                .header("content-type", "text/plain")
                .body(Bytes::from(format!("Error reading response body: {}", err)))
                .expect("failed to build error response");
            return Ok(reply);
        }
    };

    // Add extra headers provided by the user.
    for header in config.extra_headers.iter() {
        if let Some((name, value)) = header.split_once(":") {
            if let (Ok(header_name), Ok(header_value)) = (
                warp::http::header::HeaderName::from_bytes(name.trim().as_bytes()),
                warp::http::HeaderValue::from_str(value.trim()),
            ) {
                resp_headers.insert(header_name, header_value);
            } else {
                warn!("Invalid extra header format: {}", header);
            }
        } else {
            warn!("Extra header not in 'Key: Value' format: {}", header);
        }
    }

    // If the flag is set, add some useful CORS headers.
    if config.add_cors_headers {
        resp_headers.insert(
            warp::http::header::ACCESS_CONTROL_ALLOW_ORIGIN,
            warp::http::HeaderValue::from_static("*"),
        );
        resp_headers.insert(
            warp::http::header::ACCESS_CONTROL_ALLOW_METHODS,
            warp::http::HeaderValue::from_static("GET, POST, PUT, DELETE, OPTIONS"),
        );
        resp_headers.insert(
            warp::http::header::ACCESS_CONTROL_ALLOW_HEADERS,
            warp::http::HeaderValue::from_static("Content-Type, Authorization"),
        );
        if !resp_headers.contains_key(warp::http::header::CONTENT_TYPE) {
            resp_headers.insert(
                warp::http::header::CONTENT_TYPE,
                warp::http::HeaderValue::from_static("application/json"),
            );
        }
    }

    // Build the final response using the forwarded status, headers, and body.
    let mut response_builder = warp::http::Response::builder().status(status);
    for (name, value) in resp_headers.iter() {
        response_builder = response_builder.header(name, value);
    }
    let response = response_builder
        .body(resp_body)
        .expect("failed to build response");

    Ok(response)
}
