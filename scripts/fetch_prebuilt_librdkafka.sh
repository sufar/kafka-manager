#!/usr/bin/env bash
# 下载 sufar/rdkafka-sys 的预编译 librdkafka 静态库，用于跳过 kafka-manager
# 构建时 librdkafka 的 cmake 编译（约 2 分钟 → 秒级）。
#
# 用法：
#   ./scripts/fetch_prebuilt_librdkafka.sh [tag] [target]   # tag 默认 prebuilt（滚动 CI 产物）；
#   target 默认自动探测，可显式指定（多目标构建如 universal-apple-darwin 时分多次调用）
#   export LIBRDKAFKA_PREBUILT_DIR=<脚本输出的路径>           # 之后正常 cargo build
#
# 产物布局：~/.cache/kafka-manager/librdkafka-prebuilt/<tag>/<target>/librdkafka.a
# 单目标构建：export 指向 <tag>/<target>（含 .a 的目录）即可；
# 多目标构建（universal-apple-darwin）：分别取两个 target 后 export 指向 <tag> 父目录，
# build.rs 会自动按 <dir>/<target-triple>/ 子目录匹配。
#
# 约束：预编译库 feature 集 = zlib+zstd+lz4-ext+snappy，无 SSL/SASL/CURL，
# 与本项目 rdkafka feature 集一致；改了 feature 就不要用预编译。
# Linux 有两类产物：CI 版（ubuntu-22.04 x86_64/aarch64，glibc 2.35，推荐）与
# 本机版（-glibc2.43 后缀，要求 glibc ≥ 2.43，当前仅 aarch64）；脚本优先 CI 版，404 时回退本机版。
set -euo pipefail

REPO="sufar/rdkafka-sys"
TAG="${1:-prebuilt}"
TARGET_ARG="${2:-}"

detect_target() {
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64)    echo "x86_64-unknown-linux-gnu" ;;
    Linux-aarch64)   echo "aarch64-unknown-linux-gnu" ;;
    Darwin-x86_64)   echo "x86_64-apple-darwin" ;;
    Darwin-arm64)    echo "aarch64-apple-darwin" ;;
    MINGW*-x86_64|MSYS*-x86_64|CYGWIN*-x86_64)
                     echo "x86_64-pc-windows-msvc" ;;
    *) echo "" ;;
  esac
}

TARGET="${TARGET_ARG:-$(detect_target)}"
[ -n "$TARGET" ] || { echo "不支持的平台: $(uname -s)-$(uname -m)（可用第二个参数显式指定 target）" >&2; exit 1; }
case "$TARGET" in
  *windows-msvc) LIB="rdkafka.lib" ;;
  *)             LIB="librdkafka.a" ;;
esac

DEST="$HOME/.cache/kafka-manager/librdkafka-prebuilt/$TAG/$TARGET"
if [ -f "$DEST/$LIB" ]; then
  echo "已存在: ${DEST}/${LIB}（删除后重跑可强制刷新）"
  echo "export LIBRDKAFKA_PREBUILT_DIR=$DEST"
  exit 0
fi

mkdir -p "$DEST" && cd "$DEST"

download() { curl -fSL --connect-timeout 15 -o "$1" \
  "https://github.com/$REPO/releases/download/$TAG/$1"; }

ASSET="librdkafka-$TARGET.tar.gz"
if ! download "$ASSET" 2>/dev/null; then
  if [ "${TARGET#*-}" = "unknown-linux-gnu" ]; then
    ASSET="librdkafka-$TARGET-glibc2.43.tar.gz"
    # glibc2.43 本机版只存在于 v4.10.0+2.15.1 release（非 prebuilt 滚动 release）
    TAG="v4.10.0+2.15.1"
    DEST="$HOME/.cache/kafka-manager/librdkafka-prebuilt/$TAG/$TARGET"
    mkdir -p "$DEST" && cd "$DEST"
    echo "CI 产物不可用，回退本机构建版（要求 glibc ≥ 2.43）"
    download "$ASSET"
    download "$ASSET.sha256"
  else
    echo "Release 中没有 ${ASSET}（CI 可能尚未运行完成）" >&2
    exit 1
  fi
else
  download "$ASSET.sha256"
fi

if command -v sha256sum >/dev/null 2>&1; then
  sha256sum -c "$ASSET.sha256"
else
  shasum -a 256 -c "$ASSET.sha256"
fi
tar xzf "$ASSET"
rm -f "$ASSET" "$ASSET.sha256"

echo "预编译 librdkafka 就绪: ${DEST}（含 licenses/ 协议文本）"
echo "export LIBRDKAFKA_PREBUILT_DIR=${DEST}"
