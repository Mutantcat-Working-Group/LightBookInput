#!/usr/bin/env bash
# 按 tools/release/data.lock 下载并校验单包产品数据；旧锁仍可读取分离的模型附件。
#
#   tools/release/data-fetch.sh            # 下载 + 校验 + 解开
#   tools/release/data-fetch.sh --verify   # 只校验 target/release-data/ 里已下载的文件
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
LOCK="$ROOT/tools/release/data.lock"
OUT="$ROOT/target/release-data"
# 默认取上游 qingjian-team/qingjian 的 data-vN release：轻书是青简的同源分支，数据包字节一致
# （data.lock 里的哈希就是按这份算的）。等轻书自己发 data-vN release 后用这两个环境变量指回来。
REPO="${LIGHTBOOKINPUT_DATA_REPO:-qingjian-team/qingjian}"
# 锁文件与 CI 里一律按本仓的规范名记；上游资产叫 qingjian-data.tar.gz，下载后改回规范名。
DATA_KEY=lightbookinput-data.tar.gz
DATA_ASSET="${LIGHTBOOKINPUT_DATA_ASSET:-qingjian-data.tar.gz}"
ASSETS=("$DATA_KEY")
cd "$ROOT"

[[ -f "$LOCK" ]] || { echo "缺少 $LOCK" >&2; exit 1; }
lock_value() { sed -nE "s/^$1 *= *//p" "$LOCK" | head -1 | tr -d '[:space:]'; }
sha256() { if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1; }

TAG="$(lock_value tag)"
[[ -n "$TAG" ]] || { echo "$LOCK 里没有 tag" >&2; exit 1; }
LEGACY_MODEL="$(lock_value model.qjm)"
if [[ -n "$LEGACY_MODEL" ]]; then
  ASSETS+=(model.qjm)
  [[ -n "$(lock_value model-p2c.qjm)" ]] && ASSETS+=(model-p2c.qjm)
fi
mkdir -p "$OUT"

if [[ "${1:-}" != "--verify" ]]; then
  download() {
    local f="$1" asset="${2:-$1}"
    rm -f "$OUT/$f"
    if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
      gh release download "$TAG" --repo "$REPO" --pattern "$asset" --dir "$OUT"
    else
      curl -fL --retry 3 -o "$OUT/$asset" "https://github.com/$REPO/releases/download/$TAG/$asset"
    fi
    [[ "$asset" == "$f" ]] || mv -f "$OUT/$asset" "$OUT/$f"
  }
  download "$DATA_KEY" "$DATA_ASSET"
  for f in "${ASSETS[@]}"; do
    [[ "$f" == "$DATA_KEY" ]] || download "$f"
  done
fi

for f in "${ASSETS[@]}"; do
  expected="$(lock_value "$f")"
  actual="$(sha256 "$OUT/$f")"
  [[ -n "$expected" && "$actual" == "$expected" ]] || { echo "$f 与 data.lock 不符（$TAG）：期望 $expected，实际 $actual" >&2; exit 1; }
  echo "$f  $actual"
done
[[ "${1:-}" == "--verify" ]] && exit 0

if [[ -n "$LEGACY_MODEL" ]]; then
  mkdir -p data/generated data/models/hanzhang-zhiwei
  tar -xzf "$OUT/lightbookinput-data.tar.gz" -C data/generated
  cp "$OUT/model.qjm" data/models/hanzhang-zhiwei/model.qjm
  MODEL_SOURCE=data/models/hanzhang-zhiwei/model.qjm
  P2C_MODEL_SOURCE=""
  if [[ -n "$(lock_value model-p2c.qjm)" ]]; then
    mkdir -p data/models/hanzhang-tongbian
    cp "$OUT/model-p2c.qjm" data/models/hanzhang-tongbian/model.qjm
    P2C_MODEL_SOURCE=data/models/hanzhang-tongbian/model.qjm
  fi
else
  tar -xzf "$OUT/lightbookinput-data.tar.gz" -C "$ROOT"
  MODEL_SOURCE=data/models/hanzhang-zhiwei/hanzhang-zhiwei-small.qjm
  P2C_MODEL_SOURCE=data/models/hanzhang-tongbian/hanzhang-tongbian-small.qjm
  [[ -f "$MODEL_SOURCE" && -f "$P2C_MODEL_SOURCE" && -f data/generated/dict.qj ]] || {
    echo "产品数据包缺少词库或含章模型（$TAG）" >&2; exit 1;
  }
fi
# 解出来的 mtime 比 checkout 出来的 TSV 旧，bundle.sh 会以为要重打
find data/generated -type f -exec touch {} +
touch "$MODEL_SOURCE"
[[ -n "$P2C_MODEL_SOURCE" ]] && touch "$P2C_MODEL_SOURCE"
echo "产品数据 $TAG 已就位"

if [[ -n "${GITHUB_ENV:-}" ]]; then
  {
    echo "DATA_TAG=$TAG"
    echo "DATA_SHA256=$(lock_value lightbookinput-data.tar.gz)"
    echo "MODEL_SHA256=$(sha256 "$MODEL_SOURCE")"
    if [[ -n "$P2C_MODEL_SOURCE" ]]; then
      echo "P2C_MODEL_SHA256=$(sha256 "$P2C_MODEL_SOURCE")"
    else
      echo "P2C_MODEL_SHA256="
    fi
  } >> "$GITHUB_ENV"
fi
