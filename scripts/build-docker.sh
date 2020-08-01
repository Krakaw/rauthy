#!/bin/bash
set -e
TAG="${1:-latest}"
NAME="${2:-nginx-auth}"
DOCKER_BUILDKIT=1 docker build -t "$NAME:$TAG" --progress=plain .

if [ -n "$SSH_HOST" ]; then
  echo "Deploying to $SSH_HOST"
  docker save "$NAME:$TAG" | bzip2 | pv | ssh -o 'RemoteCommand=none' "$SSH_HOST"  'bunzip2 | docker load'
else
  # shellcheck disable=SC2016
  echo 'Set $SSH_HOST to automatically deploy'
fi
