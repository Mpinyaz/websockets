FROM rust:slim-bullseye AS build
WORKDIR /app

# Install required system dependencies
RUN apt-get update && apt-get install -y libssl-dev pkg-config build-essential && rm -rf /var/lib/apt/lists/*

# Copy all source code (respects .dockerignore)
COPY . .

# Build mdwstrading
WORKDIR /app/mdwstrading
RUN cargo build --release

# Production stage
FROM debian:bullseye-slim
WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=build /app/mdwstrading/target/release/mdwstrading .

CMD ["./mdwstrading"]
