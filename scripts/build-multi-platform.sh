#!/bin/bash
# Deviruchi 多平台构建脚本
# 支持 Linux、macOS 和 Windows

set -e

echo "╔══════════════════════════════════════════════════════════╗"
echo "║  Deviruchi 多平台构建脚本                                 ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

# 创建输出目录
mkdir -p releases

# 构建 Linux x86_64
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  构建 Linux x86_64 版本..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

TARGET="x86_64-unknown-linux-gnu"
echo "  目标平台: $TARGET"
cargo build --release --target $TARGET

# 打包 Linux 版本
echo "  打包 Linux 版本..."
mkdir -p releases/deviruchi-linux-x64
cp target/$TARGET/release/deviruchi releases/deviruchi-linux-x64/
cp target/$TARGET/release/devi-agent releases/deviruchi-linux-x64/
cp deviruchi.toml releases/deviruchi-linux-x64/ 2>/dev/null || true
cp README.md releases/deviruchi-linux-x64/
cp -r db releases/deviruchi-linux-x64/ 2>/dev/null || true

cd releases
tar -czf deviruchi-linux-x64.tar.gz deviruchi-linux-x64
cd ..

echo "  ✓ Linux 版本构建完成: releases/deviruchi-linux-x64.tar.gz"
echo ""

# 构建 macOS (当前平台)
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  构建 macOS 版本..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# 检测当前架构
ARCH=$(uname -m)
if [ "$ARCH" = "arm64" ]; then
    TARGET="aarch64-apple-darwin"
    PLATFORM="macos-arm64"
else
    TARGET="x86_64-apple-darwin"
    PLATFORM="macos-x64"
fi

echo "  目标平台: $TARGET"
cargo build --release --target $TARGET

# 打包 macOS 版本
echo "  打包 macOS 版本..."
mkdir -p releases/deviruchi-$PLATFORM
cp target/$TARGET/release/deviruchi releases/deviruchi-$PLATFORM/
cp target/$TARGET/release/devi-agent releases/deviruchi-$PLATFORM/
cp deviruchi.toml releases/deviruchi-$PLATFORM/ 2>/dev/null || true
cp README.md releases/deviruchi-$PLATFORM/
cp -r db releases/deviruchi-$PLATFORM/ 2>/dev/null || true

cd releases
tar -czf deviruchi-$PLATFORM.tar.gz deviruchi-$PLATFORM
cd ..

echo "  ✓ macOS 版本构建完成: releases/deviruchi-$PLATFORM.tar.gz"
echo ""

# 构建 Windows
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  构建 Windows 版本..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

TARGET="x86_64-pc-windows-msvc"
echo "  目标平台: $TARGET"
cargo build --release --target $TARGET

# 打包 Windows 版本
echo "  打包 Windows 版本..."
mkdir -p releases/deviruchi-windows-x64
cp target/$TARGET/release/deviruchi.exe releases/deviruchi-windows-x64/
cp target/$TARGET/release/devi-agent.exe releases/deviruchi-windows-x64/
cp deviruchi.toml releases/deviruchi-windows-x64/ 2>/dev/null || true
cp README.md releases/deviruchi-windows-x64/
cp -r db releases/deviruchi-windows-x64/ 2>/dev/null || true

cd releases
zip -r deviruchi-windows-x64.zip deviruchi-windows-x64
cd ..

echo "  ✓ Windows 版本构建完成: releases/deviruchi-windows-x64.zip"
echo ""

# 显示结果
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  构建完成！"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "  生成的文件:"
ls -lh releases/*.tar.gz releases/*.zip 2>/dev/null
echo ""
echo "  支持的平台:"
echo "  - Linux x86_64: deviruchi-linux-x64.tar.gz"
echo "  - macOS ARM64: deviruchi-macos-arm64.tar.gz"
echo "  - macOS x64: deviruchi-macos-x64.tar.gz"
echo "  - Windows x64: deviruchi-windows-x64.zip"
echo ""
