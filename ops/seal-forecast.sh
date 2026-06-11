#!/usr/bin/env bash
# seal-forecast.sh — Hash-seal a prediction registry entry and create a signed git tag.
#
# Usage: bash ops/seal-forecast.sh data/forecast-registry/entries/<entry>.yml
#
# Protocol:
#   1. Read the entry YAML.
#   2. Strip the seal_digest and ots_proof fields (they must be null before sealing).
#   3. Compute SHA-256 of the stripped body — this is the canonical seal.
#   4. Write the seal_digest back into the file.
#   5. Create a git tag forecast/<entry_id>/v1 pointing at HEAD.
#   6. Print instructions for OpenTimestamps submission.
#
# After this script: git commit the updated file, then submit the tag hash to OpenTimestamps.

set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "Usage: bash ops/seal-forecast.sh <entry.yml>" >&2
    exit 1
fi

ENTRY_FILE="$1"

if [[ ! -f "$ENTRY_FILE" ]]; then
    echo "Error: entry file not found: $ENTRY_FILE" >&2
    exit 1
fi

# Extract entry_id from YAML (simple grep, not a full YAML parser)
ENTRY_ID=$(grep '^entry_id:' "$ENTRY_FILE" | awk '{print $2}' | tr -d '"')
if [[ -z "$ENTRY_ID" ]]; then
    echo "Error: could not find entry_id in $ENTRY_FILE" >&2
    exit 1
fi

# Check that seal_digest is currently null (not already sealed)
CURRENT_DIGEST=$(grep '^seal_digest:' "$ENTRY_FILE" | awk '{print $2}')
if [[ "$CURRENT_DIGEST" != "null" && -n "$CURRENT_DIGEST" ]]; then
    echo "Error: entry already sealed (seal_digest: $CURRENT_DIGEST)" >&2
    echo "To re-seal, set seal_digest: null first (this changes the canonical content)." >&2
    exit 1
fi

# Compute the canonical body: strip seal_digest and ots_proof lines, then hash
CANONICAL=$(grep -v '^seal_digest:' "$ENTRY_FILE" | grep -v '^ots_proof:')
DIGEST=$(printf '%s' "$CANONICAL" | sha256sum | awk '{print $1}')

echo "Entry ID  : $ENTRY_ID"
echo "SHA-256   : $DIGEST"

# Write seal_digest back into the file (replace the null line)
# Use a temp file for atomicity
TMP="$ENTRY_FILE.sealing"
sed "s/^seal_digest: null$/seal_digest: \"$DIGEST\"/" "$ENTRY_FILE" > "$TMP"
mv "$TMP" "$ENTRY_FILE"

echo "Wrote seal_digest to $ENTRY_FILE"

# Write current code hash
CODE_HASH=$(git rev-parse HEAD 2>/dev/null || echo "not-a-git-repo")
sed -i "s/^code_hash: null$/code_hash: \"$CODE_HASH\"/" "$ENTRY_FILE"
echo "Wrote code_hash: $CODE_HASH"

# Create git tag
TAG="forecast/$ENTRY_ID/v1"
if git tag -l "$TAG" | grep -q "$TAG"; then
    echo "Warning: tag $TAG already exists — skipping tag creation." >&2
else
    git tag -a "$TAG" -m "Sealed forecast entry: $ENTRY_ID (seal_digest: $DIGEST)"
    echo "Created git tag: $TAG"
fi

echo ""
echo "=== Next steps ==="
echo "1. git add $ENTRY_FILE && git commit -m 'feat(forecast): seal $ENTRY_ID'"
echo "2. Submit to OpenTimestamps:"
echo "     TAG_HASH=\$(git rev-parse $TAG)"
echo "     printf '%s' \"\$TAG_HASH\" | sha256sum -b | xxd -r -p > /tmp/tagref.bin"
echo "     ots stamp /tmp/tagref.bin && mv /tmp/tagref.bin.ots data/forecast-registry/entries/$ENTRY_ID.ots"
echo "3. Once the .ots file is obtained, set ots_proof in the YAML and re-commit."
