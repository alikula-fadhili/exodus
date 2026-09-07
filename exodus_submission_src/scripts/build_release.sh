#!/usr/bin/env bash
set -euo pipefail

# Project Exodus: Release Build & Packaging Script

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
DIST_DIR="${REPO_ROOT}/dist"
VERSION="0.1.0"
ARCHIVE_NAME="exodus-v${VERSION}-linux-x86_64"

echo "🔨 Building Project Exodus release binary (version ${VERSION})..."
cd "${REPO_ROOT}"

cargo build --release -p exodus-cli

echo "📦 Packaging distribution archive..."
mkdir -p "${DIST_DIR}/${ARCHIVE_NAME}/bin"

cp "${REPO_ROOT}/target/release/exodus" "${DIST_DIR}/${ARCHIVE_NAME}/bin/exodus"
strip "${DIST_DIR}/${ARCHIVE_NAME}/bin/exodus" || true

cp "${REPO_ROOT}/README.md" "${DIST_DIR}/${ARCHIVE_NAME}/"
cp "${REPO_ROOT}/REPRODUCTION.md" "${DIST_DIR}/${ARCHIVE_NAME}/"
cp -r "${REPO_ROOT}/fixtures" "${DIST_DIR}/${ARCHIVE_NAME}/"
cp -r "${REPO_ROOT}/migrations" "${DIST_DIR}/${ARCHIVE_NAME}/"

# Create standalone install script inside the archive
cat << 'INNER_EOF' > "${DIST_DIR}/${ARCHIVE_NAME}/install.sh"
#!/usr/bin/env bash
set -euo pipefail

INSTALL_DIR="${HOME}/.local/bin"
mkdir -p "${INSTALL_DIR}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cp "${SCRIPT_DIR}/bin/exodus" "${INSTALL_DIR}/exodus"
chmod +x "${INSTALL_DIR}/exodus"

echo "✅ Installed exodus to ${INSTALL_DIR}/exodus"
if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
    echo "⚠️ Notice: ${INSTALL_DIR} is not in your \$PATH. Add it via:"
    echo "   export PATH=\"\${HOME}/.local/bin:\$PATH\""
fi

echo "🔍 Running exodus doctor..."
"${INSTALL_DIR}/exodus" doctor || true
INNER_EOF
chmod +x "${DIST_DIR}/${ARCHIVE_NAME}/install.sh"

cd "${DIST_DIR}"
tar -czf "${ARCHIVE_NAME}.tar.gz" "${ARCHIVE_NAME}"

echo "🎉 Successfully built release package:"
echo "   Archive: ${DIST_DIR}/${ARCHIVE_NAME}.tar.gz"
echo "   Binary:  ${REPO_ROOT}/target/release/exodus"
