#!/usr/bin/env bash
# 构建本 devcontainer 镜像（仅本地 tag，不 push）
#   ./build_img.sh
# 代理：若本机 1080 端口有代理，则作为构建代理传入（--network=host 下用 127.0.0.1 即可）
set -euo pipefail
cd "$(dirname "$0")"

IMAGE="machine-dev:local"

BUILD_ARGS=()
if timeout 2 bash -c 'echo > /dev/tcp/127.0.0.1/1080' 2>/dev/null; then
  echo "[build] 检测到本机代理 127.0.0.1:1080，作为构建代理注入"
  BUILD_ARGS+=(--build-arg "HTTP_PROXY=http://127.0.0.1:1080"
               --build-arg "HTTPS_PROXY=http://127.0.0.1:1080"
               --build-arg "NO_PROXY=localhost,127.0.0.1,::1,192.168.0.0/16")
else
  echo "[build] 未检测到 1080 代理，直连构建"
fi

docker build \
  --network=host \
  "${BUILD_ARGS[@]}" \
  -t "$IMAGE" \
  .

echo
echo "[build] 完成（仅本地，未 push）:"
docker images "$IMAGE"
echo
echo "下一步: VS Code 打开本目录 → Reopen in Container（devcontainer.json 已指向 $IMAGE）"