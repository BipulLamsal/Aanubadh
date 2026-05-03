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
FROM rust:1.85-bookworm AS backend-builder

# Install build dependencies for Debian
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy source and build
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
RUN cargo build --release

# ─────────────────────────────────────────────
# Stage 3: Final Runtime Image
# ─────────────────────────────────────────────
# Using python slim as base for pdf2docx and docx2pdf
FROM python:3.11-slim-bookworm

# Install runtime dependencies
# We need libreoffice for docx2pdf to work on Linux
# And fonts for Nepali/Tamang support
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl-dev \
    libreoffice \
    fonts-dejavu \
    fonts-liberation \
    fonts-noto-core \
    && rm -rf /var/lib/apt/lists/*

# Install Python libraries for PDF translation
RUN pip install --no-cache-dir pdf2docx docx2pdf

WORKDIR /app

# Copy the compiled binary from backend-builder
COPY --from=backend-builder /app/target/release/tmt ./tmt

# Copy the built frontend
COPY --from=frontend-builder /app/frontend/dist ./frontend/dist

# Create directory for translated files
RUN mkdir -p translated_files

# Ensure the app knows where to find Python
ENV PYTHONUNBUFFERED=1

# Expose the port (Render sets PORT env var)
EXPOSE 1997

# Run the server
CMD ["./tmt"]
