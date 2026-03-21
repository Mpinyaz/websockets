FROM rust:1.85-alpine AS build
WORKDIR /app

# Install required system dependencies
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig

# Copy all source code (respects .dockerignore)
COPY . .

# Build mdworkers
WORKDIR /app/mdworkers
RUN cargo build --release --bin server

# Production stage
FROM alpine:latest
WORKDIR /app
COPY --from=build /app/mdworkers/target/release/server .
CMD ["./server"]
