# ─────────────────────────────────────────────
# Stage 1: Build the React frontend
# ─────────────────────────────────────────────
FROM node:20-alpine AS frontend-builder

WORKDIR /app/frontend

# Copy package files first for better caching
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci

# Copy frontend source and build
COPY frontend/ ./
RUN npm run build

# ─────────────────────────────────────────────
# Stage 2: Build the Rust backend
# ─────────────────────────────────────────────
FROM rust:1.86-bookworm AS backend-builder

WORKDIR /app

# Copy Cargo files first for dependency caching
COPY Cargo.toml Cargo.lock ./
# Create a dummy main.rs so cargo can fetch dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src

# Copy real source and rebuild
COPY src/ src/
RUN touch src/main.rs && cargo build --release

# ─────────────────────────────────────────────
# Stage 3: Final minimal runtime image
# ─────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled binary
COPY --from=backend-builder /app/target/release/tmt ./tmt

# Copy the built frontend
COPY --from=frontend-builder /app/frontend/dist ./frontend/dist

# Copy test files if they exist (optional, for demo)
COPY test_file[s]/ ./frontend/dist/test_files/

# Create directory for translated files
RUN mkdir -p translated_files

# Expose the port (Render sets PORT env var)
EXPOSE 1997

# Run the server
CMD ["./tmt"]
