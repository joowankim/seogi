use crate::domain::status::StatusCategory;
use crate::domain::task::TaskEvent;
use crate::domain::value::{Ms, Timestamp};

/// 상태 이름 → 카테고리 매핑을 나타내는 타입 별칭.
type StatusMap<'a> = &'a [(String, StatusCategory)];

/// 상태 이름을 카테고리로 변환한다.
fn resolve_category(status_name: &str, status_map: StatusMap<'_>) -> Option<StatusCategory> {
    status_map
        .iter()
        .find(|(name, _)| name == status_name)
        .map(|(_, cat)| *cat)
}

/// 이벤트 중 `to_status`가 지정 카테고리에 해당하는 첫 이벤트의 timestamp를 반환한다.
fn first_transition_to(
    events: &[TaskEvent],
    category: StatusCategory,
    status_map: StatusMap<'_>,
) -> Option<Timestamp> {
    events
        .iter()
        .find(|e| resolve_category(e.to_status(), status_map) == Some(category))
        .map(TaskEvent::timestamp)
}

/// Backlog → Unstarted 전환 시간.
///
/// 첫 Backlog 카테고리 도착 시점부터 첫 Unstarted 카테고리 도착 시점까지의 간격.
#[must_use]
pub fn triage_time(events: &[TaskEvent], status_map: StatusMap<'_>) -> Option<Ms> {
    let backlog_ts = first_transition_to(events, StatusCategory::Backlog, status_map)?;
    let unstarted_ts = first_transition_to(events, StatusCategory::Unstarted, status_map)?;
    Some(Ms::new(unstarted_ts.value() - backlog_ts.value()))
}

/// Started → Completed 전환 시간.
///
/// 첫 Started 카테고리 도착 시점부터 첫 Completed 카테고리 도착 시점까지의 간격.
#[must_use]
pub fn cycle_time(events: &[TaskEvent], status_map: StatusMap<'_>) -> Option<Ms> {
    let started_ts = first_transition_to(events, StatusCategory::Started, status_map)?;
    let completed_ts = first_transition_to(events, StatusCategory::Completed, status_map)?;
    Some(Ms::new(completed_ts.value() - started_ts.value()))
}

/// 생성 시점 → 첫 Completed 시간.
///
/// `created_at_ms`는 태스크 생성 시각의 밀리초 Unix timestamp.
#[must_use]
pub fn lead_time(
    events: &[TaskEvent],
    created_at_ms: Timestamp,
    status_map: StatusMap<'_>,
) -> Option<Ms> {
    let completed_ts = first_transition_to(events, StatusCategory::Completed, status_map)?;
    Some(Ms::new(completed_ts.value() - created_at_ms.value()))
}

/// `lead_time` - `cycle_time` (대기 시간).
#[must_use]
pub fn wait_time(lead: Option<Ms>, cycle: Option<Ms>) -> Option<Ms> {
    let l = lead?;
    let c = cycle?;
    Some(Ms::new(l.value() - c.value()))
}

/// `cycle_time` / `lead_time` (흐름 효율).
#[must_use]
pub fn flow_efficiency(cycle: Option<Ms>, lead: Option<Ms>) -> Option<f64> {
    let c = cycle?.value();
    let l = lead?.value();
    if l == 0 {
        return None;
    }
    Some(c as f64 / l as f64)
}

/// 기간 내 Completed 전환 이벤트 수.
///
/// `completed_events`는 이미 Completed 카테고리로 필터된 이벤트 목록.
#[must_use]
pub fn throughput(completed_events: &[TaskEvent]) -> u32 {
    let mut task_ids: Vec<&str> = completed_events.iter().map(TaskEvent::task_id).collect();
    task_ids.sort_unstable();
    task_ids.dedup();
    task_ids.len() as u32
}

/// Completed→Started 전환이 발생한 태스크가 있는지 확인.
fn has_rework(events: &[TaskEvent], status_map: StatusMap<'_>) -> bool {
    events.windows(2).any(|pair| {
        let from_cat = resolve_category(pair[0].to_status(), status_map);
        let to_cat = resolve_category(pair[1].to_status(), status_map);
        from_cat == Some(StatusCategory::Completed) && to_cat == Some(StatusCategory::Started)
    })
}

/// rework가 발생한 태스크 비율.
///
/// `all_completed_event_groups`는 완료된 각 태스크의 이벤트 목록.
/// 반환값은 rework 발생 태스크 수 / 전체 완료 태스크 수.
#[must_use]
pub fn rework_rate(all_completed_event_groups: &[&[TaskEvent]], status_map: StatusMap<'_>) -> f64 {
    if all_completed_event_groups.is_empty() {
        return 0.0;
    }
    let rework_count = all_completed_event_groups
        .iter()
        .filter(|events| has_rework(events, status_map))
        .count();
    rework_count as f64 / all_completed_event_groups.len() as f64
}

/// rework 없이 완료된 태스크 비율.
#[must_use]
pub fn first_time_done_rate(
    all_completed_event_groups: &[&[TaskEvent]],
    status_map: StatusMap<'_>,
) -> f64 {
    if all_completed_event_groups.is_empty() {
        return 0.0;
    }
    let no_rework_count = all_completed_event_groups
        .iter()
        .filter(|events| !has_rework(events, status_map))
        .count();
    no_rework_count as f64 / all_completed_event_groups.len() as f64
}

/// 미완료 태스크의 나이 (현재 시각 - 생성 시각).
#[must_use]
pub fn issue_age(
    events: &[TaskEvent],
    created_at_ms: Timestamp,
    now_ms: Timestamp,
    status_map: StatusMap<'_>,
) -> Option<Ms> {
    let is_completed = first_transition_to(events, StatusCategory::Completed, status_map).is_some();
    let is_canceled = first_transition_to(events, StatusCategory::Canceled, status_map).is_some();
    if is_completed || is_canceled {
        return None;
    }
    Some(Ms::new(now_ms.value() - created_at_ms.value()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status_map() -> Vec<(String, StatusCategory)> {
        vec![
            ("backlog".to_string(), StatusCategory::Backlog),
            ("todo".to_string(), StatusCategory::Unstarted),
            ("in_progress".to_string(), StatusCategory::Started),
            ("in_review".to_string(), StatusCategory::Started),
            ("blocked".to_string(), StatusCategory::Started),
            ("done".to_string(), StatusCategory::Completed),
            ("canceled".to_string(), StatusCategory::Canceled),
        ]
    }

    fn event(from: Option<&str>, to: &str, ts: i64) -> TaskEvent {
        TaskEvent::new("SEO-1", from, to, "CLI", Timestamp::new(ts))
    }

    fn event_for_task(task_id: &str, from: Option<&str>, to: &str, ts: i64) -> TaskEvent {
        TaskEvent::new(task_id, from, to, "CLI", Timestamp::new(ts))
    }

    // --- triage_time ---

    #[test]
    fn triage_time_backlog_to_unstarted() {
        let sm = status_map();
        let events = vec![
            event(None, "backlog", 1000),
            event(Some("backlog"), "todo", 3000),
        ];
        assert_eq!(triage_time(&events, &sm), Some(Ms::new(2000)));
    }

    #[test]
    fn triage_time_no_unstarted() {
        let sm = status_map();
        let events = vec![event(None, "backlog", 1000)];
        assert_eq!(triage_time(&events, &sm), None);
    }

    #[test]
    fn triage_time_no_events() {
        let sm = status_map();
        assert_eq!(triage_time(&[], &sm), None);
    }

    // --- cycle_time ---

    #[test]
    fn cycle_time_started_to_completed() {
        let sm = status_map();
        let events = vec![
            event(None, "backlog", 1000),
            event(Some("backlog"), "todo", 2000),
            event(Some("todo"), "in_progress", 3000),
            event(Some("in_progress"), "done", 8000),
        ];
        assert_eq!(cycle_time(&events, &sm), Some(Ms::new(5000)));
    }

    #[test]
    fn cycle_time_no_completed() {
        let sm = status_map();
        let events = vec![
            event(None, "backlog", 1000),
            event(Some("backlog"), "in_progress", 3000),
        ];
        assert_eq!(cycle_time(&events, &sm), None);
    }

    #[test]
    fn cycle_time_no_started() {
        let sm = status_map();
        let events = vec![event(None, "backlog", 1000)];
        assert_eq!(cycle_time(&events, &sm), None);
    }

    // --- lead_time ---

    #[test]
    fn lead_time_created_to_completed() {
        let sm = status_map();
        let events = vec![
            event(None, "backlog", 1000),
            event(Some("backlog"), "done", 10000),
        ];
        assert_eq!(
            lead_time(&events, Timestamp::new(500), &sm),
            Some(Ms::new(9500))
        );
    }

    #[test]
    fn lead_time_no_completed() {
        let sm = status_map();
        let events = vec![event(None, "backlog", 1000)];
        assert_eq!(lead_time(&events, Timestamp::new(500), &sm), None);
    }

    // --- wait_time ---

    #[test]
    fn wait_time_both_present() {
        assert_eq!(
            wait_time(Some(Ms::new(10000)), Some(Ms::new(5000))),
            Some(Ms::new(5000))
        );
    }

    #[test]
    fn wait_time_missing_lead() {
        assert_eq!(wait_time(None, Some(Ms::new(5000))), None);
    }

    #[test]
    fn wait_time_missing_cycle() {
        assert_eq!(wait_time(Some(Ms::new(10000)), None), None);
    }

    // --- flow_efficiency ---

    #[test]
    fn flow_efficiency_normal() {
        let eff = flow_efficiency(Some(Ms::new(5000)), Some(Ms::new(10000)));
        assert!((eff.unwrap() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn flow_efficiency_zero_lead() {
        assert_eq!(flow_efficiency(Some(Ms::new(0)), Some(Ms::new(0))), None);
    }

    #[test]
    fn flow_efficiency_missing() {
        assert_eq!(flow_efficiency(None, Some(Ms::new(10000))), None);
        assert_eq!(flow_efficiency(Some(Ms::new(5000)), None), None);
    }

    // --- throughput ---

    #[test]
    fn throughput_counts_unique_tasks() {
        let events = vec![
            event_for_task("SEO-1", Some("in_progress"), "done", 1000),
            event_for_task("SEO-2", Some("in_progress"), "done", 2000),
            // duplicate SEO-1 (rework then re-completed)
            event_for_task("SEO-1", Some("in_progress"), "done", 3000),
        ];
        assert_eq!(throughput(&events), 2);
    }

    #[test]
    fn throughput_empty() {
        assert_eq!(throughput(&[]), 0);
    }

    // --- rework_rate ---

    #[test]
    fn rework_rate_one_rework() {
        let sm = status_map();
        // Task with rework: done -> in_progress -> done
        let events_rework = vec![
            event(Some("todo"), "in_progress", 1000),
            event(Some("in_progress"), "done", 2000),
            event(Some("done"), "in_progress", 3000),
            event(Some("in_progress"), "done", 4000),
        ];
        // Task without rework
        let events_clean = vec![
            event(Some("todo"), "in_progress", 1000),
            event(Some("in_progress"), "done", 5000),
        ];
        let groups: Vec<&[TaskEvent]> = vec![&events_rework, &events_clean];
        assert!((rework_rate(&groups, &sm) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn rework_rate_no_completed() {
        let sm = status_map();
        let groups: Vec<&[TaskEvent]> = vec![];
        assert!((rework_rate(&groups, &sm)).abs() < f64::EPSILON);
    }

    // --- first_time_done_rate ---

    #[test]
    fn first_time_done_rate_all_clean() {
        let sm = status_map();
        let events = vec![
            event(Some("todo"), "in_progress", 1000),
            event(Some("in_progress"), "done", 5000),
        ];
        let groups: Vec<&[TaskEvent]> = vec![&events];
        assert!((first_time_done_rate(&groups, &sm) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn first_time_done_rate_with_rework() {
        let sm = status_map();
        let events_rework = vec![
            event(Some("todo"), "in_progress", 1000),
            event(Some("in_progress"), "done", 2000),
            event(Some("done"), "in_progress", 3000),
            event(Some("in_progress"), "done", 4000),
        ];
        let events_clean = vec![
            event(Some("todo"), "in_progress", 1000),
            event(Some("in_progress"), "done", 5000),
        ];
        let groups: Vec<&[TaskEvent]> = vec![&events_rework, &events_clean];
        assert!((first_time_done_rate(&groups, &sm) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn first_time_done_rate_empty() {
        let sm = status_map();
        let groups: Vec<&[TaskEvent]> = vec![];
        assert!((first_time_done_rate(&groups, &sm)).abs() < f64::EPSILON);
    }

    // --- issue_age ---

    #[test]
    fn issue_age_incomplete_task() {
        let sm = status_map();
        let events = vec![
            event(None, "backlog", 1000),
            event(Some("backlog"), "in_progress", 2000),
        ];
        let now = Timestamp::new(10000);
        assert_eq!(
            issue_age(&events, Timestamp::new(500), now, &sm),
            Some(Ms::new(9500))
        );
    }

    #[test]
    fn issue_age_completed_task() {
        let sm = status_map();
        let events = vec![
            event(None, "backlog", 1000),
            event(Some("backlog"), "done", 5000),
        ];
        let now = Timestamp::new(10000);
        assert_eq!(issue_age(&events, Timestamp::new(500), now, &sm), None);
    }

    #[test]
    fn issue_age_canceled_task() {
        let sm = status_map();
        let events = vec![
            event(None, "backlog", 1000),
            event(Some("backlog"), "canceled", 5000),
        ];
        let now = Timestamp::new(10000);
        assert_eq!(issue_age(&events, Timestamp::new(500), now, &sm), None);
    }

    #[test]
    fn issue_age_no_events() {
        let sm = status_map();
        let now = Timestamp::new(10000);
        assert_eq!(
            issue_age(&[], Timestamp::new(500), now, &sm),
            Some(Ms::new(9500))
        );
    }

    // --- resolve_category edge cases ---

    #[test]
    fn resolve_category_unknown_status() {
        let sm = status_map();
        assert_eq!(resolve_category("unknown_status", &sm), None);
    }

    #[test]
    fn triage_time_custom_status_names() {
        let custom_map = vec![
            ("triage".to_string(), StatusCategory::Backlog),
            ("ready".to_string(), StatusCategory::Unstarted),
        ];
        let events = vec![
            event(None, "triage", 1000),
            event(Some("triage"), "ready", 4000),
        ];
        assert_eq!(triage_time(&events, &custom_map), Some(Ms::new(3000)));
    }

    // --- rework detection edge: consecutive completed→started ---

    #[test]
    fn rework_detected_via_consecutive_events() {
        let sm = status_map();
        let events = vec![
            event(Some("todo"), "in_progress", 1000),
            event(Some("in_progress"), "done", 2000),
            event(Some("done"), "in_progress", 3000),
        ];
        assert!(has_rework(&events, &sm));
    }

    #[test]
    fn no_rework_without_completed_to_started() {
        let sm = status_map();
        let events = vec![
            event(Some("todo"), "in_progress", 1000),
            event(Some("in_progress"), "done", 2000),
        ];
        assert!(!has_rework(&events, &sm));
    }
}
