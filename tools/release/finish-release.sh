#!/usr/bin/env bash
# publish job：把各平台传上来的摘要片段合成 SHA256SUMS、checksum-md5.txt、checksum-sha1.txt 与 build-info.json，
# 删掉片段，草稿转正。
#
#   tools/release/finish-release.sh <标签> <版本> <输出目录>
#
# build-info.json 顶层的 commit / built_at / toolchain / data_* 是 releases_json.py 读的（built_at 取最晚的平台），各平台的构建机在 platforms 里。
set -euo pipefail

TAG="$1"
VERSION="$2"
OUT="$3"
mkdir -p "$OUT"
gh release download "$TAG" \
  --pattern 'SHA256SUMS-*' \
  --pattern 'checksum-md5-*.txt' \
  --pattern 'checksum-sha1-*.txt' \
  --pattern 'build-info-*.json' \
  --dir "$OUT" --clobber
cat "$OUT"/SHA256SUMS-* | tr -d '\r' | sort -k2 | tee "$OUT/SHA256SUMS"
cat "$OUT"/checksum-md5-* | tr -d '\r' | sort -k2 | tee "$OUT/checksum-md5.txt"
cat "$OUT"/checksum-sha1-* | tr -d '\r' | sort -k2 | tee "$OUT/checksum-sha1.txt"
jq -s --arg version "$VERSION" --arg tag "$TAG" \
  '{version: $version, tag: $tag, commit: .[0].commit, built_at: (map(.built_at) | max), toolchain: .[0].toolchain,
    data_tag: .[0].data_tag, data_sha256: .[0].data_sha256, model_sha256: .[0].model_sha256,
    p2c_model_sha256: .[0].p2c_model_sha256,
    platforms: (map({(.platform): {built_at, runner}}) | add)}' \
  "$OUT"/build-info-*.json | tee "$OUT/build-info.json"
gh release upload "$TAG" \
  "$OUT/SHA256SUMS" \
  "$OUT/checksum-md5.txt" \
  "$OUT/checksum-sha1.txt" \
  "$OUT/build-info.json" --clobber
for fragment in "$OUT"/SHA256SUMS-* "$OUT"/checksum-md5-* "$OUT"/checksum-sha1-* "$OUT"/build-info-*.json; do
  gh release delete-asset "$TAG" "$(basename "$fragment")" --yes
done
# 预发布（0.1.4-beta.1）不抢 GitHub 的 latest
LATEST=(--latest); [[ "$VERSION" == *-* ]] && LATEST=(--latest=false)
gh release edit "$TAG" --draft=false "${LATEST[@]}"
