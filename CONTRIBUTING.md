# Contributing to Beep

Beep is open to contributions from anyone interested in making it better.

## Setting up the development environment

### Prerequisites

- [Cargo](https://rustup.rs/)
- [Docker](https://docs.docker.com/get-docker/)

### 0. Setting up the environment

```bash
cp .env.example .env
```

Then, edit the `.env` file to set the correct values for the environment variables.

### 1. Setting up the bucket

```bash
docker compose up -d
./setup.sh >> .env
```

The setup script will create a bucket named `beep` and a key named `beep_admin` with read/write permissions for the bucket. It will output the credentials so we can redirect them to the `.env` file.

## Running the tests

For this repository, we are using [cargo-insta](https://github.com/mitsuhiko/insta) for snapshot testing.
To install it, run:

```bash
cargo install cargo-insta
```

To run the tests, you can use the following command:

```bash
cargo test
```

If your tests outputs are different from the snapshots, you can update the snapshots by running:

```bash
cargo insta review
```

## Traces & logs

How to view traces and logs locally:

- Start the collector (and other dev services) in the background:

```bash
docker compose up -d otel-collector
```

- Run the application locally with the OTLP endpoint pointed at the collector. The Rust instrumentation in `src/telemetry.rs` uses the OTLP exporter and defaults to gRPC; you can set the endpoint explicitly in the environment variable file:

```bash
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 
```

Notes:

- The collector is configured in `otel-config.yaml`.

- Because the collector in this repo uses the `debug` exporter, spans and logs will be written to the collector's stdout. To follow them in real time, tail the collector logs:

```bash
docker compose logs -f otel-collector
```

## Instrumenting the code — example

The code below demonstrates using `tracing` together with OpenTelemetry to create spans and attach structured fields. The same ideas apply to metrics and other telemetry signals.

```rust
#[tracing::instrument]
async fn example_handler() {
    // attach structured fields and basic metrics-like fields
    tracing::info!(
        "handling example",
        monotonic_counter_example = 1_u64,
        key_1 = "bar",
        key_2 = 10,
    );

    tracing::info!(histogram_example = 10_i64, "example histogram event");
}
```
