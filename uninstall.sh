#!/usr/bin/env bash
set -euo pipefail

echo "Uninstalling seogi..."

SEOGI_DIR="$HOME/.seogi"
CLAUDE_SETTINGS="$HOME/.claude/settings.json"

# 1. Claude Code 설정에서 hook 제거
if [[ -f "$CLAUDE_SETTINGS" ]]; then
  echo "Removing hooks from Claude Code settings..."

  UPDATED_SETTINGS=$(jq --arg seogi_dir "$SEOGI_DIR" '
    if .hooks then
      .hooks.PreToolUse = [.hooks.PreToolUse[]? | select(.hooks[]? | (type == "string" and contains($seogi_dir)) | not)] |
      .hooks.PostToolUse = [.hooks.PostToolUse[]? | select(.hooks[]? | (type == "string" and contains($seogi_dir)) | not)] |
      .hooks.Notification = [.hooks.Notification[]? | select(.hooks[]? | (type == "string" and contains($seogi_dir)) | not)] |
      if .hooks.PreToolUse == [] then del(.hooks.PreToolUse) else . end |
      if .hooks.PostToolUse == [] then del(.hooks.PostToolUse) else . end |
      if .hooks.Notification == [] then del(.hooks.Notification) else . end |
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
