# Stage 1: Build Admin Web
FROM node:20-slim AS frontend-builder
WORKDIR /app
COPY admin-web/package*.json ./
RUN npm install
COPY admin-web .
RUN npm run build

# Stage 2: Build Rust Backend
FROM rust:1.92-slim-trixie AS backend-builder
WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev build-essential && rm -rf /var/lib/apt/lists/*

# --- Dependency Caching Layer ---
# 1. Copy manifests
COPY into_response_derive into_response_derive
COPY Cargo.toml Cargo.lock ./

# 2. Create dummy source files and directories to satisfy build.rs and Cargo
RUN mkdir -p src && echo "fn main() {}" > src/main.rs

# 3. Build to cache dependencies
RUN cargo build --release

# 4. Cleanup dummy artifacts
RUN rm -rf src target/release/deps/tiny_bbs*

# --- Real Build ---
# Copy source code
COPY assets assets
COPY locales locales
COPY public public
COPY src src
COPY templates templates
COPY build.rs build.rs

# Copy frontend build artifacts to public/admin-web
# Create the directory first just in case
RUN mkdir -p public/admin-web
COPY --from=frontend-builder /app/dist/public/admin-web ./public/admin-web

# Build the application (release mode)
# This will trigger build.rs, which compiles SCSS and zips the public directory (including the copied admin-web)
RUN cargo build --release

# Stage 3: Runtime Environment
FROM debian:trixie-slim
WORKDIR /app

# Install runtime dependencies
# sqlite3 is needed for database initialization script
# ca-certificates and openssl are needed for HTTP requests (reqwest) and potential SSL usage
RUN apt-get update && apt-get install -y \
    sqlite3 \
    ca-certificates \
    openssl \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder stage
COPY --from=backend-builder /app/target/release/nodecask .

# Copy the database initialization SQL
COPY schema.sql .

# Copy the entrypoint script
COPY entrypoint.sh .
RUN chmod +x entrypoint.sh

# Expose the port (assuming default is 3000, adjust if config differs)
EXPOSE 3000

# Define volume for persistent data
VOLUME ["/app/data"]

# Set the entrypoint
ENTRYPOINT ["./entrypoint.sh"]
