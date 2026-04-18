#!/usr/bin/env bash
set -euo pipefail

echo "Uninstalling seogi..."

SEOGI_DIR="$HOME/.seogi"
CLAUDE_SETTINGS="$HOME/.claude/settings.json"

# 1. Claude Code 설정에서 seogi 훅 제거
if [[ -f "$CLAUDE_SETTINGS" ]]; then
  echo "Removing hooks from Claude Code settings..."

  UPDATED_SETTINGS=$(jq '
    if .hooks then
      .hooks |= with_entries(
        .value |= [.[]? | select(
          (.hooks // []) | all(
            (.command // "") | (contains("seogi hook") or contains(".seogi")) | not
          )
        )]
      ) |
      .hooks |= with_entries(select(.value | length > 0)) |
      if .hooks == {} then del(.hooks) else . end
    else . end
  ' "$CLAUDE_SETTINGS")

  echo "$UPDATED_SETTINGS" > "$CLAUDE_SETTINGS"
fi

# 2. seogi 디렉토리 제거
if [[ -d "$SEOGI_DIR" ]]; then
  echo "Removing $SEOGI_DIR..."
  rm -rf "$SEOGI_DIR"
fi

echo ""
echo "✓ seogi uninstalled successfully!"
echo ""
echo "Note: Log files in ~/seogi-logs were preserved."
echo "To remove logs: rm -rf ~/seogi-logs"
