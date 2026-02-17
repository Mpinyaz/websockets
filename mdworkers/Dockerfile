FROM rust:1.85-alpine AS build
WORKDIR /app

# Install required system dependencies
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig

COPY Cargo.toml Cargo.lock ./

# Cache dependencies with dummy binary
RUN mkdir -p src/bin && echo "fn main() {}" > src/bin/server.rs
RUN cargo build --release --bin server
RUN rm -rf src

# Copy real source and build
COPY src ./src
RUN cargo build --release --bin server

# Production stage
FROM alpine:latest
WORKDIR /app

# Runtime openssl dependency
RUN apk add --no-cache libssl3 libcrypto3

COPY --from=build /app/target/release/server .
CMD ["./server"]
