FROM rust:1.85-alpine AS build
WORKDIR /app

# Install required system dependencies
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig

# Copy manifest files
COPY Cargo.toml Cargo.lock ./

# Create a dummy source to cache dependencies
RUN mkdir -p src/bin && echo "fn main() {}" > src/bin/server.rs && touch src/lib.rs
RUN cargo build --release --bin server
RUN rm -rf src

# Copy real source
COPY src ./src

# Ensure both the library and binary are rebuilt with real source
RUN touch src/lib.rs src/bin/server.rs && cargo build --release --bin server

# Production stage
FROM alpine:latest
WORKDIR /app

# Runtime dependencies
RUN apk add --no-cache libssl3 libcrypto3 ca-certificates gcompat

COPY --from=build /app/target/release/server .
CMD ["./server"]
