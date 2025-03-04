# Proxxyy | Rust Proxy Server

> [!WARNING]
>
> ## Not production ready
>
> How did you even end up here in the first place? Anyway, this software is definitely not production-ready. Do not use it—at all—unless you want to make my cat very, very sad. And trust me, you don't want that. 😾
>

This project implements a simple proxy server in Rust that forwards incoming HTTP requests to a specified target URL. It supports optional CORS headers, additional extra headers in the response, and logs every incoming request.

### Features

- **Request Proxying:** Forwards any incoming request to the provided target URL.
- **CORS Headers:** Optionally adds default CORS headers (such as `Content-Type`, `Access-Control-Allow-Origin`, etc.) to responses.
- **Extra Headers:** Allows you to add additional custom response headers.
- **Logging:** Logs each incoming request (method, path, headers, etc.) using the `log` crate.
- **Mocking Support (New!)**
  - *Mock Files:* You may provide a [TOML file](#using-mocks) specifying an array of mock configurations (`[[mocks]]`).
  - *Loading Body from File:* If the `body` parameter in the mock ends with `.json`, `.txt`, or `.html`, the system attempts to load that file's contents from disk. Otherwise, it uses the literal string as the body.

### Prerequisites

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
> cargo run --release -- \
>  -t 'https://api.example.com/api/' \
>  -u 'http://localhost:6969' \ \
>  -c \
>  -e 'x-proxy-bob: yes' \
>  -e 'x-proxy-alice: no'
> ```

Also you can install it with `cargo install --path=.`.

### Running the Proxy Server

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

- `--save-request-directory` or `-s`
  You can save incoming requests to a directory. Each request will be saved as a JSON file.

- `--hide-headers` or `-h`
  When present, request headers will not be logged. Useful for security or reducing log verbosity.

- `--hide-body` or `-b`
  When present, request bodies will not be logged. Useful for security, privacy, or reducing log verbosity.

### Using Mocks

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

## Example of Saving Requests Feature

The `--save-request-directory` (or `-s`) flag allows you to save all requests and responses to a specified directory. This is useful for:

- Creating a record of API interactions
- Generating mock configurations automatically
- Debugging API responses
- Creating test fixtures

### Usage

```bash
proxxyy -t "https://api.example.com" -u "http://localhost:6969" -s "./saved_requests"
```

### Generated Files

When you use the save feature, the following files are created in the specified directory:

1. **Timestamped JSON Response Files**
   - Each response is saved as a beautified JSON file
   - Filenames include the endpoint path, query parameters, and a timestamp
   - Example: `users_page_1_per_page_10_1700000000.json`
   - Special characters are replaced with underscores for valid filenames

2. **Mock Configuration File (mocked-request.toml)**
   - A single TOML file containing entries for all captured requests
   - Each entry includes the HTTP method, complete path with query parameters, and a reference to the JSON file
   - New requests are appended to this file, preserving the history of all requests

### Example Directory Structure

```bash
saved_requests/
├── mocked-request.toml
├── users_1700000000.json
├── users_id_123_1700000010.json
└── users_page_1_per_page_10_1700000020.json
```

### Example TOML File Content

```toml
# Mock configuration file generated by proxxyy
# Each entry represents a mock endpoint

[[mocks]]
method = "GET"
path = "/users"
status = 200
body = "users_1700000000.json"

[[mocks]]
method = "GET"
path = "/users/123"
status = 200
body = "users_id_123_1700000010.json"

[[mocks]]
method = "GET"
path = "/users?page=1&per_page=10"
status = 200
body = "users_page_1_per_page_10_1700000020.json"
```

### Using the Generated Mock Configuration

You can use the generated TOML file directly with the `--mock-config` flag to replay the saved responses:

```bash
proxxyy -t 'https://api.example.com/api/' \
   -u "http://localhost:6969" \
   -m "./saved_requests/mocked-request.toml"
```

This will serve the saved responses for matching requests, without forwarding to any target URL.

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

### Full Example Usage

To run the proxy server with the full parameter names:

```bash
proxxyy --add-cors-headers \
    --target-url='https://api.example.com/api/' \
    --api-url='http://localhost:6969' \
    --extra-header='x-proxy-bob: yes' \
    --extra-header='x-proxy-alice: no' \
    --save-request-directory='./requests' \
    --hide-headers \
    --hide-body
```

Using the shorthand options:

```bash
proxxyy -c \
    -t 'https://api.example.com/api/' \
    -u 'http://localhost:6969' \
    -e 'x-proxy-bob: yes' \
    -e 'x-proxy-alice: no' \
    -m 'mocks.toml' \
    -s './requests' \
    -h \
    -b
```
