use std::collections::{HashMap, HashSet};

/// 간선 `(from, to)`을 추가했을 때 순환이 발생하는지 검사한다.
///
/// 기존 간선 목록에 `(from, to)`를 추가한 뒤 `from`에서 시작하는
/// 사이클이 존재하면 `true`를 반환한다.
#[must_use]
pub fn detect_cycle(edges: &[(String, String)], from: &str, to: &str) -> bool {
    let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();
    for (src, dst) in edges {
        graph.entry(src.as_str()).or_default().push(dst.as_str());
    }
    graph.entry(from).or_default().push(to);

    let mut visited = HashSet::new();
    let mut stack = vec![to];
    while let Some(node) = stack.pop() {
        if node == from {
            return true;
        }
        if visited.insert(node)
            && let Some(neighbors) = graph.get(node)
        {
            stack.extend(neighbors.iter());
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    // Q7: 순환 없음 → false
    #[test]
    fn test_detect_cycle_no_cycle() {
        let edges = vec![
            ("A".to_string(), "B".to_string()),
            ("B".to_string(), "C".to_string()),
        ];
        assert!(!detect_cycle(&edges, "D", "A"));
    }

    // Q8: 직접 순환 (A→B→A) → true
    #[test]
    fn test_detect_cycle_direct() {
        let edges = vec![("A".to_string(), "B".to_string())];
        assert!(detect_cycle(&edges, "B", "A"));
    }

    // Q9: 간접 순환 (A→B→C→A) → true
    #[test]
    fn test_detect_cycle_indirect() {
        let edges = vec![
            ("A".to_string(), "B".to_string()),
            ("B".to_string(), "C".to_string()),
        ];
        assert!(detect_cycle(&edges, "C", "A"));
    }

    // Q10: 빈 그래프에서 순환 없음
    #[test]
    fn test_detect_cycle_empty_graph() {
        let edges: Vec<(String, String)> = vec![];
        assert!(!detect_cycle(&edges, "A", "B"));
    }

    // 다이아몬드 그래프에서 재방문 노드 처리 (visited.insert false 분기)
    #[test]
    fn test_detect_cycle_diamond_no_cycle() {
        // A→B, A→C, B→D, C→D — E→A 추가 시 순환 없음
        // DFS: A→B,C → B→D, C→D — D가 두 번 스택에 들어가 두 번째는 visited.insert false
        let edges = vec![
            ("A".to_string(), "B".to_string()),
            ("A".to_string(), "C".to_string()),
            ("B".to_string(), "D".to_string()),
            ("C".to_string(), "D".to_string()),
        ];
        assert!(!detect_cycle(&edges, "E", "A"));
    }
}
