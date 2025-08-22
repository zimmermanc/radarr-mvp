#!/bin/bash

# Archive Kubernetes manifests for local-first development approach
# This script moves k8s/ directory to k8s.archived/ to remove container orchestration complexity

set -e

echo "=== Archiving Kubernetes Manifests ==="
echo "Moving to local-first development approach"

if [ -d "k8s" ]; then
    echo "Moving k8s/ to k8s.archived/"
    mv k8s k8s.archived
    echo "✓ Kubernetes manifests archived"
    echo ""
    echo "Note: Kubernetes manifests are preserved in k8s.archived/"
    echo "      but are no longer part of the active development workflow"
    echo "      which now focuses on direct server deployment"
else
    echo "k8s/ directory not found - already archived or removed"
fi

echo ""
echo "Local-first development setup complete:"
echo "  - Use scripts/deploy.sh for server deployment"
echo "  - Use systemd/radarr.service for service management"
echo "  - See DEPLOYMENT.md for complete instructions"