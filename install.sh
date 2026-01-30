#!/usr/bin/env bash
set -euo pipefail

echo "Installing seogi..."

SEOGI_DIR="$HOME/.seogi"
SEOGI_LOGS_DIR="$HOME/seogi-logs"
CLAUDE_SETTINGS="$HOME/.claude/settings.json"

# 1. seogi 디렉토리 생성 및 파일 복사
echo "Creating $SEOGI_DIR..."
mkdir -p "$SEOGI_DIR/hooks" "$SEOGI_DIR/lib"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cp "$SCRIPT_DIR/config.json" "$SEOGI_DIR/"
cp "$SCRIPT_DIR/hooks/"*.sh "$SEOGI_DIR/hooks/"
cp "$SCRIPT_DIR/lib/"*.sh "$SEOGI_DIR/lib/"
chmod +x "$SEOGI_DIR/hooks/"*.sh "$SEOGI_DIR/lib/"*.sh

# 2. 기본 로그 디렉토리 생성
echo "Creating log directory at $SEOGI_LOGS_DIR..."
mkdir -p "$SEOGI_LOGS_DIR"

# 3. Claude Code 설정에 hook 추가
echo "Configuring Claude Code hooks..."
mkdir -p "$(dirname "$CLAUDE_SETTINGS")"

if [[ -f "$CLAUDE_SETTINGS" ]]; then
  # 기존 설정이 있으면 백업
  cp "$CLAUDE_SETTINGS" "$CLAUDE_SETTINGS.backup"

  # hooks 섹션 추가/병합
  UPDATED_SETTINGS=$(jq --arg seogi_dir "$SEOGI_DIR" '
    .hooks = (.hooks // {}) |
    .hooks.PreToolUse = (.hooks.PreToolUse // []) + [{
      "matcher": "*",
      "hooks": [($seogi_dir + "/hooks/pre-tool.sh")]
    }] |
    .hooks.PostToolUse = (.hooks.PostToolUse // []) + [{
      "matcher": "*",
      "hooks": [($seogi_dir + "/hooks/post-tool.sh")]
    }] |
    .hooks.Notification = (.hooks.Notification // []) + [{
      "matcher": "*",
      "hooks": [($seogi_dir + "/hooks/notification.sh")]
    }]
  ' "$CLAUDE_SETTINGS")

  echo "$UPDATED_SETTINGS" > "$CLAUDE_SETTINGS"
else
  # 새 설정 파일 생성
  jq -n --arg seogi_dir "$SEOGI_DIR" '{
    hooks: {
      PreToolUse: [{
        matcher: "*",
        hooks: [($seogi_dir + "/hooks/pre-tool.sh")]
      }],
      PostToolUse: [{
        matcher: "*",
        hooks: [($seogi_dir + "/hooks/post-tool.sh")]
      }],
      Notification: [{
        matcher: "*",
        hooks: [($seogi_dir + "/hooks/notification.sh")]
      }]
    }
  }' > "$CLAUDE_SETTINGS"
fi

echo ""
echo "✓ seogi installed successfully!"
echo ""
echo "Configuration: $SEOGI_DIR/config.json"
echo "Logs will be saved to: $SEOGI_LOGS_DIR"
echo ""
echo "To customize, edit $SEOGI_DIR/config.json:"
echo '  {"logDir": "~/seogi-logs", "maxFileSizeMB": 10}'
