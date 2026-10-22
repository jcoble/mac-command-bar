use super::*;

fn parse_codex(lines: &[String], last_total: &mut Option<String>) -> Vec<UsageEvent> {
    parse_usage_events_for_source_with_context(
        lines,
        "codex",
        "local",
        "codex-jsonl",
        "/tmp/rollout.jsonl",
        &UsageSourceContext::default(),
        last_total,
    )
}

fn token_count(total: u64, last: u64) -> String {
    format!(
        r#"{{"type":"event_msg","payload":{{"type":"token_count","info":{{"total_token_usage":{{"input_tokens":{total},"output_tokens":1}},"last_token_usage":{{"input_tokens":{last},"output_tokens":1}}}}}}}}"#
    )
}

fn input_sum(events: &[UsageEvent]) -> u64 {
    events
        .iter()
        .map(|event| event.input_tokens + event.cache_read_tokens)
        .sum()
}

#[test]
fn codex_token_count_records_become_non_zero_events() {
    let lines = vec![
        r#"{"type":"session_meta","payload":{"id":"session-2","cwd":"/tmp/project/workspace"}}"#.to_string(),
        r#"{"type":"turn_context","payload":{"model":"gpt-5.6-sol"}}"#.to_string(),
        r#"{"type":"event_msg","timestamp":"2026-08-08T12:34:56Z","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":20,"cached_input_tokens":5,"output_tokens":9,"reasoning_output_tokens":4}}}}"#.to_string(),
    ];
    let events = parse_codex(&lines, &mut None);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].owned_id.as_deref(), Some("session-2"));
    assert_eq!(events[0].input_tokens, 15);
    assert_eq!(events[0].cache_read_tokens, 5);
    assert_eq!(events[0].output_tokens, 9);
    assert_eq!(events[0].reasoning_tokens, 4);
    assert_eq!(events[0].model, "gpt-5.6-sol");
}

#[test]
fn codex_repeated_total_token_usage_counts_once() {
    let mut last_total = None;
    let events = parse_codex(
        &[
            token_count(100, 100),
            token_count(100, 100),
            token_count(150, 50),
        ],
        &mut last_total,
    );
    assert_eq!(events.len(), 2);
    assert_eq!(input_sum(&events), 150);
    assert_eq!(
        last_total,
        Some(r#"{"input_tokens":150,"output_tokens":1}"#.to_string())
    );
}

#[test]
fn codex_repeated_total_keeps_the_first_when_last_usage_differs() {
    let events = parse_codex(&[token_count(100, 100), token_count(100, 30)], &mut None);
    assert_eq!(events.len(), 1);
    assert_eq!(input_sum(&events), 100);
}

#[test]
fn codex_repeated_total_split_across_reads_counts_once() {
    let mut last_total = None;
    let first = parse_codex(&[token_count(100, 100)], &mut last_total);
    let second = parse_codex(
        &[token_count(100, 100), token_count(150, 50)],
        &mut last_total,
    );
    assert_eq!(input_sum(&first) + input_sum(&second), 150);
}

/// Token counts from a real Codex session (paths and text removed) that
/// repeats an unchanged total twice, once with a different last usage.
#[test]
fn codex_real_transcript_sums_to_its_final_total() {
    let lines = include_str!("codex-repeated-totals.jsonl")
        .lines()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let events = parse_codex(&lines, &mut None);
    assert_eq!(events.len(), 72);
    assert_eq!(input_sum(&events), 8_431_424);
    assert_eq!(
        events.iter().map(|event| event.output_tokens).sum::<u64>(),
        57_560
    );
}
