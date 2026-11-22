FROM rust:1.100-bookworm AS chef
RUN apt update -y && apt install -y --no-install-recommends lld clang
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

# Create a script file sourced by both interactive and non-interactive bash shells
ENV BASH_ENV "${HOME}/.bash_env"
RUN touch "${BASH_ENV}" && echo '. "${BASH_ENV}"' >> ~/.bashrc

COPY --from=planner /app/recipe.json recipe.json

# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json

# Build application
COPY . .

RUN cargo build --release -p sigil-server

# We do not need the Rust toolchain to run the binary!
FROM debian:bookworm-slim AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/sigil-server /usr/local/bin

EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/sigil-server"]
