#!/usr/bin/env bash
# release.sh — Tag and push to trigger GitHub Actions release workflow
#
# Usage: ./scripts/release.sh v0.1.0

set -euo pipefail

SCRIPT_NAME="$(basename "$0")"
LOG_FILE="${LOG_FILE:-/tmp/${SCRIPT_NAME%.*}.log}"

log()   { printf '%s [%s] %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$1" "$2" | tee -a "$LOG_FILE"; }
info()  { log "INFO" "$1"; }
error() { log "ERROR" "$1" >&2; }
die()   { error "$1"; exit 1; }

[[ "${1:-}" == "-h" || "${1:-}" == "--help" ]] && { echo "Usage: $SCRIPT_NAME <version-tag>"; echo "  e.g. $SCRIPT_NAME v0.1.0"; exit 0; }
[[ $# -lt 1 ]] && die "Version tag required. Usage: $SCRIPT_NAME v0.1.0"

TAG="$1"
[[ "$TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "Invalid tag format '$TAG'. Expected e.g. v0.1.0"

info "Checking working directory is clean..."
[[ -z "$(git status --porcelain)" ]] || die "Working directory is not clean. Commit or stash changes first."

info "Creating and pushing git tag $TAG..."
git tag "$TAG"
git push origin "$TAG"

info "Tag $TAG pushed. GitHub Actions will build and publish the release."
