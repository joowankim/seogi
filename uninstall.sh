#!/usr/bin/env bash
set -euo pipefail

echo "Uninstalling seogi..."

SEOGI_DIR="$HOME/.seogi"
CLAUDE_SETTINGS="$HOME/.claude/settings.json"

# 1. Claude Code 설정에서 hook 제거
if [[ -f "$CLAUDE_SETTINGS" ]]; then
  echo "Removing hooks from Claude Code settings..."

  UPDATED_SETTINGS=$(jq --arg seogi_dir "$SEOGI_DIR" '
    # seogi 훅 포함 여부 판정: hooks 배열 안에 문자열 또는 객체의 command 필드에 seogi_dir 포함
    def has_seogi: .hooks[]? | if type == "string" then contains($seogi_dir) elif type == "object" then ((.command // "") | contains($seogi_dir)) else false end;
    if .hooks then
      .hooks.PreToolUse = [.hooks.PreToolUse[]? | select(has_seogi | not)] |
      .hooks.PostToolUse = [.hooks.PostToolUse[]? | select(has_seogi | not)] |
      .hooks.Notification = [.hooks.Notification[]? | select(has_seogi | not)] |
      .hooks.Stop = [.hooks.Stop[]? | select(has_seogi | not)] |
      .hooks.PostToolUseFailure = [.hooks.PostToolUseFailure[]? | select(has_seogi | not)] |
      if .hooks.PreToolUse == [] then del(.hooks.PreToolUse) else . end |
      if .hooks.PostToolUse == [] then del(.hooks.PostToolUse) else . end |
      if .hooks.Notification == [] then del(.hooks.Notification) else . end |
      if .hooks.Stop == [] then del(.hooks.Stop) else . end |
      if .hooks.PostToolUseFailure == [] then del(.hooks.PostToolUseFailure) else . end |
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
