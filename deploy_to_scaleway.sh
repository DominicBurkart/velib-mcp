#!/usr/bin/env sh

set -euo pipefail

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

cd "$SCRIPT_DIR"

if [ ! -f "$SCRIPT_DIR/.env.deploy" ]; then
    echo "Missing .env.deploy — copy .env.deploy.example and fill in values"
    exit 1
fi
# shellcheck source=.env.deploy
. "$SCRIPT_DIR/.env.deploy"

: "${SCALEWAY_REGISTRY:?SCALEWAY_REGISTRY must be set in .env.deploy}"
: "${SCALEWAY_CONTAINER_NAME:?SCALEWAY_CONTAINER_NAME must be set in .env.deploy}"

BRANCH="$(git rev-parse --abbrev-ref HEAD | awk '{$1=$1};1')"
[[ $BRANCH == "main" ]] || (echo "not on main" && exit 1)

git diff-index --quiet HEAD -- || (echo 'Local uncommitted changes. Commit?' && exit)
git remote update
GIT_STATUS="$(LANG=en_US.UTF-8 git status -uno)"
if ! [[ $GIT_STATUS == *"Your branch is up to date"* ]]; then
    echo 'Remote not in sync. Push?'
    exit 1
fi

echo "Building container with Nix..."
nix build .#container --print-build-logs

echo "Loading container into podman..."
CONTAINER_PATH=$(nix path-info .#container)
skopeo copy "docker-archive:$CONTAINER_PATH" "containers-storage:velib-mcp:latest"

echo "Running container checks..."
$SCRIPT_DIR/container_checks.sh

podman tag velib-mcp:latest "$SCALEWAY_REGISTRY/velib-mcp:latest"
podman push "$SCALEWAY_REGISTRY/velib-mcp:latest" || (echo 'failed to push, are we logged in? https://www.scaleway.com/en/docs/compute/container-registry/how-to/push-images/' && exit 1)
CONTAINER_ID=$(scw container container list -o json | jq -r "first(.[] | select(.name==\"$SCALEWAY_CONTAINER_NAME\")) | .id")
scw container container deploy "$CONTAINER_ID"
