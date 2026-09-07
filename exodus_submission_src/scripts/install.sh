#!/usr/bin/env bash
set -euo pipefail

# Project Exodus: Local Release Installer

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
INSTALL_DIR="${HOME}/.local/bin"

mkdir -p "${INSTALL_DIR}"

echo "🔨 Building and installing Project Exodus release binary..."
cd "${REPO_ROOT}"
cargo build --release -p exodus-cli

cp "${REPO_ROOT}/target/release/exodus" "${INSTALL_DIR}/exodus"
chmod +x "${INSTALL_DIR}/exodus"

echo "✅ Installed exodus to ${INSTALL_DIR}/exodus"

if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
    echo "⚠️ Warning: ${INSTALL_DIR} is not in your current PATH."
    echo "   Add it by appending to ~/.bashrc or ~/.zshrc:"
    echo "   export PATH=\"\${HOME}/.local/bin:\$PATH\""
    export PATH="${INSTALL_DIR}:${PATH}"
fi

echo "🔍 Running pre-flight system diagnostics..."
"${INSTALL_DIR}/exodus" doctor

echo "🗄️ Initializing embedded SurrealDB knowledge store..."
"${INSTALL_DIR}/exodus" db init

echo "✨ Installation complete! You can now run 'exodus --help' or 'exodus demo learning-loop'."
