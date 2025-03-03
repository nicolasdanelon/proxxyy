use bytes::Bytes;
use chrono;
use clap::Parser;
use colored::Colorize;
use log::{error, info, warn};
use reqwest::Client;
use serde::Deserialize;
use serde_json;
use std::collections::{BTreeMap, HashMap};
use std::convert::Infallible;
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use url::Url;
use warp::Filter;

/// Main configuration for the proxy, including optional mock config file.
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

    /// (Optional) Path to a TOML file describing mock endpoints.
    ///
    /// If provided, the proxy will check for a matching mock before forwarding.
    #[clap(long = "mock-config", short = 'm')]
    mock_config: Option<String>,

    /// (Optional) Directory to save incoming requests as JSON files.
    ///
    /// If provided, each incoming request will be saved as a JSON file
    /// in this directory, containing the request details.
    #[clap(long = "save-request-directory", short = 's')]
    save_request_directory: Option<String>,

    /// (Optional) Hide request headers from logs.
    ///
    /// When enabled, request headers will not be logged, which can be useful
    /// for security or reducing log verbosity.
    #[clap(long = "hide-headers", short = 'h')]
    hide_headers: bool,

    /// (Optional) Hide request bodies from logs.
    ///
    /// When enabled, request bodies will not be logged, which can be useful
    /// for security, privacy, or reducing log verbosity.
    #[clap(long = "hide-body", short = 'b')]
    hide_body: bool,
}

/// A single mock rule (loaded from the config file).
/// For example, from TOML:
///
/// [[mocks]]
/// method = "GET"
/// path = "/v1/mock"
/// status = 200
/// body = "Mocked body."
///
/// [mocks.headers]
/// X-My-Header = "123"
#[derive(Debug, Deserialize, Clone)]
struct Mock {
    method: String,
    path: String,
    #[serde(default = "default_status")]
    status: u16,
    #[serde(default)]
    body: String,
    #[serde(default)]
    headers: HashMap<String, String>,
}

fn default_status() -> u16 {
    200
}

/// The top-level structure of the TOML file:
/// e.g.
/// [[mocks]]
/// method = "GET"
/// ...
#[derive(Debug, Deserialize, Clone)]
struct MockFile {
    #[serde(default)]
    mocks: Vec<Mock>,
}

/// A filter to pass a clone of the configuration to each request.
fn with_config(config: Config) -> impl Filter<Extract = (Config,), Error = Infallible> + Clone {
    warp::any().map(move || config.clone())
}

/// A filter to pass a clone of the vector of mocks to each request.
fn with_mocks(
    mocks: Option<Vec<Mock>>,
) -> impl Filter<Extract = (Option<Vec<Mock>>,), Error = Infallible> + Clone {
    warp::any().map(move || mocks.clone())
}

/// A filter to pass a clone of the Reqwest client.
fn with_client(client: Client) -> impl Filter<Extract = (Client,), Error = Infallible> + Clone {
    warp::any().map(move || client.clone())
}

/// Loads the body content from a file only if the `body_value` ends with .json, .txt, or .html.
/// Otherwise returns the literal `body_value`.
fn load_body_content(body_value: &str) -> String {
    use log::error;
    use std::fs;
    use std::path::Path;

    // Convert &str to `Path` so we can check the extension.
    let path = Path::new(body_value);
    let extension = path.extension().and_then(|ext| ext.to_str());

    // Check for recognized extensions
    match extension {
        Some("json") | Some("txt") | Some("html") => {
            // Attempt to read the file. If it fails, log and return the original string.
            match fs::read_to_string(path) {
                Ok(contents) => contents,
                Err(e) => {
                    error!("Error reading {}: {}", body_value, e);
                    body_value.to_string()
                }
            }
        }
        // If extension not recognized or missing, return just the literal
        _ => body_value.to_string(),
    }
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

    // If a --mock-config path is provided, parse that file.
    let optional_mocks = if let Some(ref path) = config.mock_config {
        match fs::read_to_string(path) {
            Ok(contents) => match toml::from_str::<MockFile>(&contents) {
                Ok(parsed) => {
                    info!("Loaded {} mock(s) from {}", parsed.mocks.len(), path);
                    Some(parsed.mocks)
                }
                Err(err) => {
                    error!("Failed to parse mock config ({}): {}", path, err);
                    None
                }
            },
            Err(err) => {
                error!("Failed to read mock config file {}: {}", path, err);
                None
            }
        }
    } else {
        None
    };

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
    //   • plus our configuration, mocks, and Reqwest client.
    let route = warp::any()
        .and(warp::method())
        .and(warp::header::headers_cloned())
        .and(warp::path::full())
        // Get the raw query string (default to empty string if missing).
        .and(
            warp::query::raw().or_else(|_| async { Ok::<(String,), Infallible>((String::new(),)) }),
        )
        .and(warp::body::bytes())
        .and(with_config(config))
        .and(with_mocks(optional_mocks))
        .and(with_client(client))
        .and_then(proxy_handler);

    // Run the server.
    warp::serve(route).run(socket_addr).await;
}

/// The handler that proxies every request (or returns a mock).
async fn proxy_handler(
    method: warp::http::Method,
    headers: warp::http::HeaderMap,
    full_path: warp::path::FullPath,
    query: String,
    body: Bytes,
    config: Config,
    mocks: Option<Vec<Mock>>,
    client: Client,
) -> Result<impl warp::Reply, Infallible> {
    // Fancy logging: display the HTTP verb (in bold blue) and complete request URL (in bold yellow)
    let complete_url = if query.is_empty() {
        full_path.as_str().to_string()
    } else {
        format!("{}?{}", full_path.as_str(), query)
    };
    info!(
        "{} {}",
        "Incoming request:".bold().green(),
        format!(
            "{} {}",
            method.to_string().bold().blue(),
            complete_url.bold().yellow()
        )
    );

    // Make a clone of the body for forwarding
    let body_for_forwarding = body.clone();

    // 1) Check if we have a matching mock.
    if let Some(ref mock_list) = mocks {
        if let Some(matched) = mock_list.iter().find(|m| {
            m.method.eq_ignore_ascii_case(method.as_str())
                && m.path.eq_ignore_ascii_case(full_path.as_str())
        }) {
            // If matched, return the mock response immediately, no forwarding.
            info!(
                "Matched mock for method {} and path {}",
                matched.method, matched.path
            );

            // Build a mock response with the given status, body, and headers.
            let mut builder = warp::http::Response::builder().status(matched.status);
            // Add the mock headers
            for (k, v) in &matched.headers {
                builder = builder.header(k, v);
            }
            // If user set --add-cors-headers, add them as well
            if config.add_cors_headers {
                builder = builder
                    .header("Access-Control-Allow-Origin", "*")
                    .header(
                        "Access-Control-Allow-Methods",
                        "GET, POST, PUT, DELETE, OPTIONS",
                    )
                    .header(
                        "Access-Control-Allow-Headers",
                        "Content-Type, Authorization",
                    );
                if !matched.headers.contains_key("Content-Type") {
                    builder = builder.header("Content-Type", "application/json");
                }
            }
            // Add extra headers from the CLI
            for h in &config.extra_headers {
                if let Some((name, value)) = h.split_once(":") {
                    let (name, value) = (name.trim(), value.trim());
                    builder = builder.header(name, value);
                }
            }
            let response_body = Bytes::from(load_body_content(&matched.body));

            // Log the mock response size
            info!("Mock response status: {}", matched.status);

            // Save response if save directory is specified
            if let Some(save_dir) = &config.save_request_directory {
                save_response_to_file(
                    save_dir,
                    &method,
                    &full_path,
                    &query,
                    &String::from_utf8_lossy(&response_body),
                );
            }

            let response = builder
                .body(response_body)
                .expect("failed to build mock response");
            return Ok(response);
        }
    }

    // 2) No mock matched -> Forward to real target.
    let target_url = config.target_url.trim_end_matches('/');
    let mut new_url = format!("{}{}", target_url, full_path.as_str());
    if !query.is_empty() {
        new_url = format!("{}?{}", new_url, query);
    }
    info!("No mock matched. Forwarding to target URL: {}", new_url);

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
    if !body_for_forwarding.is_empty() {
        req_builder = req_builder.body(body_for_forwarding);
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

    // Log the response size
    info!("Response status: {}", status);

    // Pretty-print the request headers as pretty JSON if not hidden
    if !config.hide_headers {
        let headers_map: BTreeMap<_, _> = headers
            .iter()
            .map(|(k, v)| (k.as_str(), v.to_str().unwrap_or("")))
            .collect();
        info!(
            "Request headers:\n{}",
            serde_json::to_string_pretty(&headers_map).unwrap()
        );
    } else {
        info!("Request headers: [hidden]");
    }

    // print the response body. beautify it if it's valid JSON. but first check if config is set to hide the body
    if !config.hide_body {
        let response_body_str = String::from_utf8_lossy(&resp_body);
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&response_body_str) {
            info!(
                "Response body: {}",
                serde_json::to_string_pretty(&json_value).unwrap()
            );
        } else {
            info!("Response body: {}", response_body_str);
        }
    } else {
        info!("Response body: [hidden] ({} bytes)", resp_body.len());
    }

    // Save response if save directory is specified
    if let Some(save_dir) = &config.save_request_directory {
        save_response_to_file(
            save_dir,
            &method,
            &full_path,
            &query,
            &String::from_utf8_lossy(&resp_body),
        );
    }

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

/// Helper function to save response data to a file
fn save_response_to_file(
    save_dir: &str,
    method: &warp::http::Method,
    full_path: &warp::path::FullPath,
    query: &str,
    response_body: &str,
) {
    // Get current timestamp for unique filenames
    let timestamp = chrono::Utc::now().timestamp();

    // Combine path and query into URI for the TOML file
    let complete_uri = if query.is_empty() {
        full_path.as_str().to_string()
    } else {
        format!("{}?{}", full_path.as_str(), query)
    };

    // Create safe filename base (without extension)
    let mut filename_base = full_path.as_str().to_string();
    // Remove leading slash
    if filename_base.starts_with('/') {
        filename_base.remove(0);
    }

    // Add query parameters to filename (sanitized)
    if !query.is_empty() {
        filename_base = format!(
            "{}_{}",
            filename_base,
            query.replace('&', "_").replace('=', "_")
        );
    }

    // Replace special characters with underscores
    filename_base = filename_base.replace(|c: char| !c.is_ascii_alphanumeric() && c != '_', "_");

    // Create directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(save_dir) {
        error!("Failed to create save directory {}: {}", save_dir, e);
        return;
    }

    // 1. Save the beautified JSON response body with timestamp
    let json_filename = format!("{}_{}.json", filename_base, timestamp);
    let json_path = Path::new(save_dir).join(&json_filename);

    // Try to parse the response body as JSON for beautification
    let beautified_body = match serde_json::from_str::<serde_json::Value>(response_body) {
        Ok(json_value) => {
            // If it's valid JSON, beautify it
            match serde_json::to_string_pretty(&json_value) {
                Ok(pretty) => pretty,
                Err(_) => response_body.to_string(), // Fallback to original if beautification fails
            }
        }
        Err(_) => {
            // Not valid JSON, use as-is
            response_body.to_string()
        }
    };

    // Write the beautified JSON to file
    if let Err(e) = std::fs::write(&json_path, &beautified_body) {
        error!(
            "Failed to save JSON response to {}: {}",
            json_path.display(),
            e
        );
        return;
    }
    info!("Saved JSON response to {}", json_path.display());

    // 2. Create or update the TOML mock configuration file
    let toml_filename = "mocked-request.toml";
    let toml_path = Path::new(save_dir).join(toml_filename);

    // Relative path to the JSON file from the TOML file's perspective
    let relative_json_path = json_filename;

    // Create the TOML content for this mock entry
    let mock_entry = format!(
        "[[mocks]]\nmethod = \"{}\"\npath = \"{}\"\nstatus = 200\nbody = \"{}\"\n",
        method.to_string(),
        complete_uri,
        relative_json_path
    );

    // Check if the TOML file already exists
    let toml_content = if toml_path.exists() {
        // Read existing content and append the new mock entry
        match fs::read_to_string(&toml_path) {
            Ok(content) => {
                // Always append the new entry (we want to keep all entries with timestamps)
                format!("{}\n{}", content, mock_entry)
            }
            Err(e) => {
                error!(
                    "Failed to read existing TOML file {}: {}",
                    toml_path.display(),
                    e
                );
                // Create new file with header if we can't read the existing one
                format!(
                    "# Mock configuration file generated by proxxyy\n# Each entry represents a mock endpoint\n\n{}",
                    mock_entry
                )
            }
        }
    } else {
        // Create new TOML file with header comment
        format!(
            "# Mock configuration file generated by proxxyy\n# Each entry represents a mock endpoint\n\n{}",
            mock_entry
        )
    };

    // Write the TOML file
    if let Err(e) = std::fs::write(&toml_path, toml_content) {
        error!(
            "Failed to save TOML mock config to {}: {}",
            toml_path.display(),
            e
        );
    } else {
        info!("Updated TOML mock config at {}", toml_path.display());
    }
}
