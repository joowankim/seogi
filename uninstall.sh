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

# 2. Claude Code에서 MCP 서버 설정 제거
CLAUDE_JSON="$HOME/.claude.json"
if [[ -f "$CLAUDE_JSON" ]]; then
  echo "Removing MCP server from Claude Code settings..."
  UPDATED_MCP=$(jq '
    if .mcpServers then
      del(.mcpServers.seogi) |
      if .mcpServers == {} then del(.mcpServers) else . end
    else . end
  ' "$CLAUDE_JSON")
  echo "$UPDATED_MCP" > "$CLAUDE_JSON"
fi

# 3. seogi 디렉토리 제거
if [[ -d "$SEOGI_DIR" ]]; then
  echo "Removing $SEOGI_DIR..."
  rm -rf "$SEOGI_DIR"
fi

echo ""
echo "✓ seogi uninstalled successfully!"
echo ""
echo "Note: Log files in ~/seogi-logs were preserved."
echo "To remove logs: rm -rf ~/seogi-logs"
