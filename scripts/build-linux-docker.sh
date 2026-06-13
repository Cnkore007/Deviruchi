#!/bin/bash
# 使用 Docker 构建 Linux 版本

set -e

echo "╔══════════════════════════════════════════════════════════╗"
echo "║  使用 Docker 构建 Linux 版本                               ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

# 创建输出目录
mkdir -p releases

# 使用 Rust 官方 Docker 镜像构建
echo "  构建 Linux x86_64 版本..."
docker run --rm -v "$(pwd)":/workspace -w /workspace rust:latest \
    bash -c "
        rustup target add x86_64-unknown-linux-gnu &&
        apt-get update && apt-get install -y gcc-x86-64-linux-gnu pkg-config libssl-dev &&
        export CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc &&
        cargo build --release --target x86_64-unknown-linux-gnu &&
        mkdir -p releases/deviruchi-linux-x64 &&
        cp target/x86_64-unknown-linux-gnu/release/deviruchi releases/deviruchi-linux-x64/ &&
        cp target/x86_64-unknown-linux-gnu/release/devi-agent releases/deviruchi-linux-x64/ &&
        cp deviruchi.toml releases/deviruchi-linux-x64/ 2>/dev/null || true &&
        cp README.md releases/deviruchi-linux-x64/ &&
        cp -r db releases/deviruchi-linux-x64/ 2>/dev/null || true &&
        cd releases &&
        tar -czf deviruchi-linux-x64.tar.gz deviruchi-linux-x64
    "

echo "  ✓ Linux 版本构建完成: releases/deviruchi-linux-x64.tar.gz"
echo ""
ls -lh releases/deviruchi-linux-x64.tar.gz
