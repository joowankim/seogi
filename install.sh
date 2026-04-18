#!/usr/bin/env bash
set -euo pipefail

echo "Installing seogi..."

SEOGI_DIR="$HOME/.seogi"
CLAUDE_SETTINGS="$HOME/.claude/settings.json"

# 1. seogi 디렉토리 생성
echo "Creating $SEOGI_DIR..."
mkdir -p "$SEOGI_DIR"

# 2. Rust CLI 바이너리 설치
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if command -v cargo &>/dev/null; then
  echo "Installing seogi CLI via cargo..."
  cargo install --path "$SCRIPT_DIR/cli" --quiet
else
  echo "Error: cargo not found. Install Rust toolchain first."
  exit 1
fi

# 3. Claude Code 설정에 Rust 훅 등록
echo "Configuring Claude Code hooks..."
mkdir -p "$(dirname "$CLAUDE_SETTINGS")"

# 기존 seogi 훅 제거 후 새로 추가 (멱등성 보장)
if [[ -f "$CLAUDE_SETTINGS" ]]; then
  cp "$CLAUDE_SETTINGS" "$CLAUDE_SETTINGS.backup"

  # 기존 seogi 훅 제거
  CLEANED=$(jq '
    if .hooks then
      .hooks |= with_entries(
        .value |= [.[]? | select(
          (.hooks // []) | all(
            (.command // "") | (contains("seogi hook") or contains(".seogi")) | not
          )
        )]
      ) |
      .hooks |= with_entries(select(.value | length > 0))
    else . end
  ' "$CLAUDE_SETTINGS")

  # 새 seogi 훅 추가
  UPDATED=$(echo "$CLEANED" | jq '
    .hooks = (.hooks // {}) |
    .hooks.PreToolUse = (.hooks.PreToolUse // []) + [{
      "matcher": "*",
      "hooks": [{"type": "command", "command": "seogi hook pre-tool"}]
    }] |
    .hooks.PostToolUse = (.hooks.PostToolUse // []) + [{
      "matcher": "*",
      "hooks": [{"type": "command", "command": "seogi hook post-tool"}]
    }] |
    .hooks.PostToolUseFailure = (.hooks.PostToolUseFailure // []) + [{
      "matcher": "*",
      "hooks": [{"type": "command", "command": "seogi hook post-tool-failure"}]
    }] |
    .hooks.Notification = (.hooks.Notification // []) + [{
      "matcher": "*",
      "hooks": [{"type": "command", "command": "seogi hook notification"}]
    }] |
    .hooks.Stop = (.hooks.Stop // []) + [{
      "hooks": [{"type": "command", "command": "seogi hook stop"}]
    }]
  ')

  echo "$UPDATED" > "$CLAUDE_SETTINGS"
else
  jq -n '{
    hooks: {
      PreToolUse: [{
        matcher: "*",
        hooks: [{"type": "command", "command": "seogi hook pre-tool"}]
      }],
      PostToolUse: [{
        matcher: "*",
        hooks: [{"type": "command", "command": "seogi hook post-tool"}]
      }],
      PostToolUseFailure: [{
        matcher: "*",
        hooks: [{"type": "command", "command": "seogi hook post-tool-failure"}]
      }],
      Notification: [{
        matcher: "*",
        hooks: [{"type": "command", "command": "seogi hook notification"}]
      }],
      Stop: [{
        hooks: [{"type": "command", "command": "seogi hook stop"}]
      }]
    }
  }' > "$CLAUDE_SETTINGS"
fi

echo ""
echo "✓ seogi installed successfully!"
echo ""
echo "Database: $SEOGI_DIR/seogi.db"
echo ""
echo "To migrate existing JSONL logs: seogi --config ~/.seogi/config.json migrate"
