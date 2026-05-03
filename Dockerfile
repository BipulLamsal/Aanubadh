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

# Stage 2: Build the Rust backend
# ─────────────────────────────────────────────
FROM rust:bookworm AS backend-builder

# Install build dependencies for Debian
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy source and build
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
ENV CARGO_BUILD_JOBS=1
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true
RUN cargo build --release

# ─────────────────────────────────────────────
# Stage 3: Final Runtime Image
# ─────────────────────────────────────────────
# Using python slim as base for pdf2docx and docx2pdf
FROM python:3.11-slim-bookworm

# Install runtime dependencies
# We need libreoffice-writer for docx -> pdf conversion
# And fonts for Nepali/Tamang support
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl-dev \
    libreoffice-writer-nogui \
    libreoffice-java-common \
    default-jre-headless \
    fonts-dejavu \
    fonts-liberation \
    fonts-noto-core \
    fonts-noto-ui-devanagari \
    fontconfig \
    && rm -rf /var/lib/apt/lists/*

# Install Python libraries for PDF translation
# docx2pdf is not needed on Linux since we use libreoffice directly in Rust
RUN pip install --no-cache-dir pdf2docx

WORKDIR /app

# Copy the compiled binary from backend-builder
COPY --from=backend-builder /app/target/release/tmt ./tmt

# Copy local font for Nepali/Tamang support
COPY NotoSansDevanagari-Regular.ttf /usr/share/fonts/truetype/noto/
RUN fc-cache -f -v

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
