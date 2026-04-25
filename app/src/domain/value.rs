use std::fmt;

/// 세션 식별자.
///
/// Claude Code 세션의 고유 ID를 표현한다.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SessionId(String);

impl SessionId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 밀리초 Unix timestamp.
///
/// 이벤트 발생 시각을 밀리초 단위로 표현한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Timestamp(i64);

impl Timestamp {
    #[must_use]
    pub fn now() -> Self {
        Self(chrono::Utc::now().timestamp_millis())
    }

    #[must_use]
    pub fn new(value: i64) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn value(self) -> i64 {
        self.0
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 밀리초 단위 시간 간격.
///
/// 도구 호출 소요 시간 등을 표현한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Ms(i64);

impl Ms {
    #[must_use]
    pub fn new(value: i64) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn zero() -> Self {
        Self(0)
    }

    #[must_use]
    pub fn value(self) -> i64 {
        self.0
    }
}

impl fmt::Display for Ms {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}ms", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_id_creation_and_display() {
        let sid = SessionId::new("sess-abc");
        assert_eq!(sid.as_str(), "sess-abc");
        assert_eq!(format!("{sid}"), "sess-abc");
        assert_eq!(sid.clone(), sid);
    }

    #[test]
    fn timestamp_creation_and_value() {
        let ts = Timestamp::new(1_713_000_000_000);
        assert_eq!(ts.value(), 1_713_000_000_000);
        assert_eq!(format!("{ts}"), "1713000000000");
        assert_eq!(ts, ts);
    }

    #[test]
    fn timestamp_now_is_positive() {
        let ts = Timestamp::now();
        assert!(ts.value() > 0);
    }

    #[test]
    fn ms_creation_and_display() {
        let d = Ms::new(150);
        assert_eq!(d.value(), 150);
        assert_eq!(format!("{d}"), "150ms");

        let zero = Ms::zero();
        assert_eq!(zero.value(), 0);
    }
}
