#!/usr/bin/env bash
# 平台 job 的最后一步：把安装包传到本次的草稿 Release，附上这个平台的摘要与构建信息片段
# （SHA256SUMS-<平台>、checksum-md5-<平台>.txt、checksum-sha1-<平台>.txt、build-info-<平台>.json），
# 由 publish job 的 finish-release.sh 合成最终文件。
#
#   tools/release/upload-build.sh <平台> <构建机描述> <安装包>...
#
# 需要 gh、jq 与环境变量 VERSION、GITHUB_REF_NAME、GITHUB_SHA；数据与模型摘要由 data-fetch.sh 写进 GITHUB_ENV。
set -euo pipefail

PLATFORM="$1"
RUNNER="$2"
shift 2
: "${VERSION:?}" "${GITHUB_REF_NAME:?}" "${GITHUB_SHA:?}"
OUT="$(mktemp -d)"
sha256() { if command -v sha256sum >/dev/null 2>&1; then sha256sum "$@"; else shasum -a 256 "$@"; fi; }
sha1() { if command -v sha1sum >/dev/null 2>&1; then sha1sum "$@"; else shasum -a 1 "$@"; fi; }
md5() {
  if command -v md5sum >/dev/null 2>&1; then
    md5sum "$@"
  elif command -v md5 >/dev/null 2>&1; then
    md5 -r "$@" | awk '{print $1 "  " $2}'
  else
    perl -MDigest::MD5 -e '
      for my $file (@ARGV) {
        open my $fh, "<", $file or die "$file: $!\n";
        binmode $fh;
        my $md5 = Digest::MD5->new;
        $md5->addfile($fh);
        print $md5->hexdigest, "  ", $file, "\n";
      }
    ' "$@"
  fi
}

write_checksums() {
  local output="$1" algorithm="$2"
  shift 2
  for file in "$@"; do
    (cd "$(dirname "$file")" && "$algorithm" "$(basename "$file")")
  done | tee "$OUT/$output"
}

write_checksums "SHA256SUMS-$PLATFORM" sha256 "$@"
write_checksums "checksum-md5-$PLATFORM.txt" md5 "$@"
write_checksums "checksum-sha1-$PLATFORM.txt" sha1 "$@"
jq -n --arg platform "$PLATFORM" --arg version "$VERSION" --arg tag "$GITHUB_REF_NAME" --arg commit "$GITHUB_SHA" \
  --arg built_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" --arg toolchain "$(rustc --version)" --arg runner "$RUNNER" \
  --arg data_tag "${DATA_TAG:-}" --arg data_sha256 "${DATA_SHA256:-}" --arg model_sha256 "${MODEL_SHA256:-}" --arg p2c_model_sha256 "${P2C_MODEL_SHA256:-}" \
  '{platform: $platform, version: $version, tag: $tag, commit: $commit, built_at: $built_at, toolchain: $toolchain, runner: $runner, data_tag: $data_tag, data_sha256: $data_sha256, model_sha256: $model_sha256, p2c_model_sha256: $p2c_model_sha256}' \
  | tee "$OUT/build-info-$PLATFORM.json"
gh release upload "$GITHUB_REF_NAME" "$@" \
  "$OUT/SHA256SUMS-$PLATFORM" \
  "$OUT/checksum-md5-$PLATFORM.txt" \
  "$OUT/checksum-sha1-$PLATFORM.txt" \
  "$OUT/build-info-$PLATFORM.json" --clobber
