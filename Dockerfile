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
FROM rust:alpine AS backend-builder

# Install build dependencies for Alpine
RUN apk add --no-cache musl-dev pkgconfig openssl-dev gcc

WORKDIR /app

# Copy source and build
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
RUN cargo build --release

# ─────────────────────────────────────────────
# Stage 3: Final minimal runtime image
# ─────────────────────────────────────────────
FROM alpine:latest

# Install runtime dependencies for Alpine
RUN apk add --no-cache ca-certificates libgcc openssl

WORKDIR /app

# Copy the compiled binary
COPY --from=backend-builder /app/target/release/tmt ./tmt

# Copy the built frontend
COPY --from=frontend-builder /app/frontend/dist ./frontend/dist

# Create directory for translated files
RUN mkdir -p translated_files

# Expose the port (Render sets PORT env var)
EXPOSE 1997

# Run the server
CMD ["./tmt"]
