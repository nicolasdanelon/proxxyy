# Rust Proxy Server

This project implements a simple proxy server in Rust that forwards incoming HTTP requests to a specified target URL. It supports optional CORS headers, additional extra headers in the response, and logs every incoming request.

## Features

- **Request Proxying:** Forwards any incoming request to the provided target URL.
- **CORS Headers:** Optionally adds default CORS headers (such as `Content-Type`, `Access-Control-Allow-Origin`, etc.) to responses.
- **Extra Headers:** Allows you to add additional custom response headers.
- **Logging:** Logs each incoming request (method, path, headers, etc.) using the `log` crate.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (Rust 1.40 or later is recommended, which comes with Cargo)
- Internet connectivity to fetch dependencies via Cargo.

## Building the Project

To compile the project in debug mode, run:

```bash
cargo build
```

For a release build (optimized), run:

```bash
cargo build --release
```

> **Tip:** If you prefer running the project with optimizations, you can launch it as a release build:
>
> ```bash
> cargo run --release -- -t 'https://api.example.com/api/' -u 'http://localhost:6969' -c -e 'x-proxy-bob: yes' -e 'x-proxy-alice: no'
> ```

## Running the Proxy Server

The proxy server accepts several command-line options:

- `--target-url` or `-t`  
  The target URL to which incoming requests are proxied.

- `--api-url` or `-u`  
  The URL where the proxy server will listen for incoming requests.

- `--add-cors-headers` or `-c`  
  When present, the proxy will add default CORS headers to the response.

- `--extra-header` or `-e`  
  Extra header(s) to include in the response. This option can be used multiple times with the format `"Header-Name: value"`.

### Example Usage

To run the proxy server with the full parameter names:

```bash
cargo run -- --target-url='https://api.example.com/api/' --api-url='http://localhost:6969' --add-cors-headers --extra-header='x-proxy-bob: yes' --extra-header='x-proxy-alice: no'
```

Using the shorthand options:

```bash
cargo run -- -t 'https://api.example.com/api/' -u 'http://localhost:6969' -c -e 'x-proxy-bob: yes' -e 'x-proxy-alice: no'
```

```bash
cargo run --release -- -t 'https://api.example.com/api/' -u 'http://localhost:6969' -c -e 'x-proxy-bob: yes' -e 'x-proxy-alice: no'
```

### Configuring Logging

The project uses the `env_logger` crate for logging. You can adjust the verbosity by setting the `RUST_LOG` environment variable before running the project. For example, to run the proxy with informational logging:

```bash
RUST_LOG=info cargo run -- --target-url='https://api.example.com/api/' --api-url='http://localhost:6969' --add-cors-headers --extra-header='x-proxy-bob: yes' --extra-header='x-proxy-alice: no'
```
