FROM rust:slim-bullseye AS build
WORKDIR /app

# Install required system dependencies
RUN apt-get update && apt-get install -y libssl-dev pkg-config libfontconfig1-dev build-essential && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./

# Cache dependencies with dummy binaries for all explicit and implicit binaries
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    echo "fn main() {}" > src/tui.rs
RUN cargo build --release
RUN rm -rf src

# Copy real source and build
COPY src ./src
RUN cargo build --release

# Production stage
FROM debian:bullseye-slim
WORKDIR /app

# Runtime openssl dependency
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=build /app/target/release/mdanalytics .
COPY --from=build /app/target/release/tui .
CMD ["./mdanalytics"]
