# Contributing to Beep

Beep is open to contributions from anyone interested in making it better.

## Setting up the development environment

### Prerequisites

- [Cargo](https://rustup.rs/)
- [Docker](https://docs.docker.com/get-docker/)

### 1. Setting up the bucket

```bash
docker compose up -d
./setup.sh > .env
```

The setup script will create a bucket named `beep` and a key named `beep_admin` with read/write permissions for the bucket. It will output the credentials so we can redirect them to the `.env` file.

## Running the tests

For this repository, we are using [cargo-insta](https://github.com/mitsuhiko/insta) for snapshot testing.

To run the tests, you can use the following command:

```bash
cargo test
```
