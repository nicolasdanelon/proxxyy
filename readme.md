# Proxxyy | Rust Proxy Server

> [!WARNING]
>
> ## Not production ready
>
> How did you even end up here in the first place? Anyway, this software is definitely not production-ready. Do not use it—at all—unless you want to make my cat very, very sad. And trust me, you don’t want that. 😾
>

This project implements a simple proxy server in Rust that forwards incoming HTTP requests to a specified target URL. It supports optional CORS headers, additional extra headers in the response, and logs every incoming request.

## Features

- **Request Proxying:** Forwards any incoming request to the provided target URL.
- **CORS Headers:** Optionally adds default CORS headers (such as `Content-Type`, `Access-Control-Allow-Origin`, etc.) to responses.
- **Extra Headers:** Allows you to add additional custom response headers.
- **Logging:** Logs each incoming request (method, path, headers, etc.) using the `log` crate.
- **Mocking Support (New!)**
  - *Mock Files:* You may provide a [TOML file](#using-mocks) specifying an array of mock configurations (`[[mocks]]`).
  - *Loading Body from File:* If the `body` parameter in the mock ends with `.json`, `.txt`, or `.html`, the system attempts to load that file's contents from disk. Otherwise, it uses the literal string as the body.

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

- `--mock-file` or `-m`
  The path to the TOML file containing mock configurations.

## Using Mocks

You can define a local TOML file (e.g., `mocks.toml`) with an array of `[[mocks]]` entries. Here's an example:

```toml
[[mocks]]
method = "GET"
path = "/test"
body = "Hello from test!"

[[mocks]]
method = "GET"
path = "/json-endpoint"
body = "data.json"

[mocks.headers]
Content-Type = "application/json"

[[mocks]]
method = "GET"
path = "/html-page"
body = "index.html"

[mocks.headers]
Content-Type = "text/html"
```

Here's how the `body` field works:
- If the `body` string **ends with** `.json`, `.txt`, or `.html`, the proxy attempts to read the file (e.g., `data.json`, `index.html`) from disk.
- If that file exists and is readable, its contents are returned as the mocked response body.
- If the file is missing, unreadable, or the extension does not match, the literal `body` string (e.g., `"Hello from test!"`) is served as-is.

In other words, your mocks can either embed a raw text response or point to a file for dynamic loading.

---

**Example usage with a mock file:**
```bash
cargo run -- \
    --target-url "https://api.example.com" \
    --api-url "http://localhost:3000" \
    --mocks "mocks.toml"
```

**or the short hand version:**
```bash
cargo run -- \
    -t "https://api.example.com" \
    -u "http://localhost:3000" \
    -m "~/demos/mocks/mocks.toml"
```

Make sure `mocks.toml` is in your working directory. Then, if you hit `GET /test` on `http://localhost:3000`, you'll see `"Hello from test!"` (literal string), while hitting `GET /json-endpoint` tries to serve the contents of `data.json`.

---

**Note:** Always verify your paths (relative vs. absolute) based on where you execute the binary. If you wish to keep the mock files separate, include the proper path in the `body` field (e.g. `"mocks/data.json"`) and ensure you run the binary from the project's root or otherwise adjust paths accordingly.

### Example Usage

To run the proxy server with the full parameter names:

```bash
cargo run -- \
    --target-url='https://api.example.com/api/' \
    --api-url='http://localhost:6969' \
    --add-cors-headers \
    --extra-header='x-proxy-bob: yes' \
    --extra-header='x-proxy-alice: no'
```

Using the shorthand options:

```bash
cargo run -- \
    -t 'https://api.example.com/api/' \
    -u 'http://localhost:6969' \
    -c \
    -e 'x-proxy-bob: yes' \
    -e 'x-proxy-alice: no' \
    -m 'mocks.toml'
```

```bash
cargo run --release \
    -t 'https://api.example.com/api/' \
    -u 'http://localhost:6969' \
    -c \
    -e 'x-proxy-bob: yes' \
    -e 'x-proxy-alice: no' \
    -m 'mocks.toml'
```

### Configuring Logging

The project uses the `env_logger` crate for logging. You can adjust the verbosity by setting the `RUST_LOG` environment variable before running the project. For example, to run the proxy with informational logging:

```bash
RUST_LOG=info cargo run -- \
    --target-url='https://api.example.com/api/' \
    --api-url='http://localhost:6969' \
    --add-cors-headers \
    --extra-header='x-proxy-bob: yes' \
    --extra-header='x-proxy-alice: no' \
    --mock-file='mocks.toml'
```
