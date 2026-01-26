ARG RUST_VERSION=1.89
FROM docker.io/rust:${RUST_VERSION}-trixie AS dependency
WORKDIR /opt/app

RUN mkdir -p setup/src && echo "fn main() {}" >> setup/src/main.rs
RUN mkdir -p core/src && echo "fn main() {}" >> core/src/main.rs

COPY Cargo.toml .
COPY Cargo.lock .
COPY setup/Cargo.toml setup/Cargo.toml
COPY core/Cargo.toml core/Cargo.toml

RUN cd core && cargo fetch

FROM dependency AS build

RUN rustup target add x86_64-unknown-linux-musl
RUN apt-get update && apt-get install -y musl-tools

COPY core/src core/src
RUN --mount=type=cache,target=/opt/target/ \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml  \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock  \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
	cd core && \
    cargo build --target=x86_64-unknown-linux-musl --release && \
    cp ../target/x86_64-unknown-linux-musl/release/content_core /bin/content_core

FROM debian:bullseye-slim AS final

# See https://docs.docker.com/develop/develop-images/dockerfile_best-practices/#user
RUN apt-get update && apt-get install --no-install-recommends -y ca-certificates && apt-get clean && rm -rf /var/lib/apt/lists/*

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "1000" \
    appuser
USER appuser

# Copy the executable from the "build" stage.
COPY --from=build /bin/content_core /bin/

WORKDIR /opt/app

# What the container should run when it is started.
ENTRYPOINT [ "/bin/bash", "-c"]
CMD ["/bin/content_core"]
