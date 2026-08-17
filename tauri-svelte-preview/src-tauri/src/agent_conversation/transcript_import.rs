use std::path::Path;

use super::protocol::AgentConversationProvider;
use super::transcript::{complete_lines, parse_durable_line, read_range, ProjectedRecord};

pub struct ImportedTail {
    /// Records parsed from the tail, oldest first.
    pub records: Vec<ProjectedRecord>,
    /// Byte offset of the first line included. Pass this back as `end_offset` to read further back.
    pub cutoff_offset: u64,
    /// True when the tail reaches the start of the file and nothing older remains.
    pub reached_start: bool,
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use uuid::Uuid;

    use super::*;

    struct TranscriptFixture {
        path: PathBuf,
        contents: String,
    }

    impl TranscriptFixture {
        fn new(record_count: usize) -> Self {
            let path = std::env::temp_dir().join(format!("mcb-tail-{}.jsonl", Uuid::new_v4()));
            let contents = (1..=record_count)
                .map(message_line)
                .collect::<Vec<_>>()
                .join("");
            fs::write(&path, &contents).expect("transcript fixture should be written");
            Self { path, contents }
        }

        fn line_len(&self) -> u64 {
            self.contents
                .find('\n')
                .map(|index| index as u64 + 1)
                .expect("fixture should contain a complete line")
        }

        fn len(&self) -> u64 {
            self.contents.len() as u64
        }
    }

    impl Drop for TranscriptFixture {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
        }
    }

    fn message_line(index: usize) -> String {
        format!(
            "{{\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"id\":\"item-{index}\",\"role\":\"user\",\"content\":[{{\"type\":\"input_text\",\"text\":\"message\"}}]}}}}\n"
        )
    }

    fn item_ids(tail: &ImportedTail) -> Vec<&str> {
        tail.records
            .iter()
            .filter_map(|record| record.item_id.as_deref())
            .collect()
    }

    fn fixture_provider() -> AgentConversationProvider {
        super::super::transcript::parse_provider(&["co", "dex"].concat())
            .expect("fixture provider should be supported")
    }

    fn read_fixture(
        fixture: &TranscriptFixture,
        end_offset: u64,
        max_bytes: u64,
        max_records: usize,
    ) -> ImportedTail {
        read_tail(
            fixture_provider(),
            "session-1",
            Path::new(&fixture.path),
            end_offset,
            max_bytes,
            max_records,
        )
        .expect("tail should be read")
    }

    #[test]
    fn tail_stops_at_the_byte_budget() {
        let fixture = TranscriptFixture::new(5);

        let tail = read_fixture(&fixture, fixture.len(), fixture.line_len() * 3, usize::MAX);

        assert!(!tail.records.is_empty());
        assert!(tail.cutoff_offset > 0);
        assert!(!tail.reached_start);
    }

    #[test]
    fn tail_reaching_the_start_reports_it() {
        let fixture = TranscriptFixture::new(2);

        let tail = read_fixture(&fixture, fixture.len(), fixture.len() + 1, usize::MAX);

        assert_eq!(item_ids(&tail), ["item-1", "item-2"]);
        assert_eq!(tail.cutoff_offset, 0);
        assert!(tail.reached_start);
    }

    #[test]
    fn a_window_with_no_usable_line_still_moves_the_cutoff_back() {
        let path = std::env::temp_dir().join(format!("mcb-tail-noise-{}.jsonl", Uuid::new_v4()));
        let contents = "not json\nalso not json\n";
        fs::write(&path, contents).expect("noise fixture should be written");
        let len = contents.len() as u64;

        let tail = read_tail(
            AgentConversationProvider::Codex,
            "session",
            &path,
            len,
            len,
            usize::MAX,
        )
        .expect("reading the noise fixture should succeed");

        assert!(tail.records.is_empty());
        assert_eq!(tail.cutoff_offset, 0);
        assert!(tail.reached_start);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn tail_honours_the_record_cap() {
        let fixture = TranscriptFixture::new(4);

        let tail = read_fixture(&fixture, fixture.len(), fixture.len(), 2);

        assert_eq!(item_ids(&tail), ["item-3", "item-4"]);
        assert!(tail.records.len() <= 2);
        assert_eq!(tail.cutoff_offset, fixture.line_len() * 2);
        assert!(!tail.reached_start);
    }

    #[test]
    fn reading_again_from_the_cutoff_walks_backwards_without_gaps() {
        let fixture = TranscriptFixture::new(6);
        let max_bytes = fixture.line_len() * 4;

        let newer = read_fixture(&fixture, fixture.len(), max_bytes, usize::MAX);
        let older = read_fixture(&fixture, newer.cutoff_offset, max_bytes, usize::MAX);
        let ids = item_ids(&older)
            .into_iter()
            .chain(item_ids(&newer))
            .collect::<Vec<_>>();

        assert_eq!(
            ids,
            ["item-1", "item-2", "item-3", "item-4", "item-5", "item-6"]
        );
        assert!(older.reached_start);
    }
}

pub fn read_tail(
    provider: AgentConversationProvider,
    native_session_id: &str,
    path: &Path,
    end_offset: u64,
    max_bytes: u64,
    max_records: usize,
) -> Result<ImportedTail, String> {
    let start = end_offset.saturating_sub(max_bytes);
    let read_len = end_offset - start;
    if read_len == 0 {
        return Ok(ImportedTail {
            records: Vec::new(),
            cutoff_offset: end_offset,
            reached_start: end_offset == 0,
        });
    }

    let bytes = read_range(path, start, read_len)?;
    let (lines, _) = complete_lines(&bytes, start > 0);
    let bytes_start = bytes.as_ptr() as usize;
    let mut parsed_lines = lines
        .into_iter()
        .filter_map(|line| {
            let records = parse_durable_line(provider, native_session_id, line);
            if records.is_empty() {
                return None;
            }
            let line_offset = start + (line.as_ptr() as usize - bytes_start) as u64;
            Some((line_offset, records))
        })
        .collect::<Vec<_>>();

    let mut record_count = parsed_lines
        .iter()
        .map(|(_, records)| records.len())
        .sum::<usize>();
    let mut first_surviving_line = 0;
    while record_count > max_records && first_surviving_line < parsed_lines.len() {
        record_count -= parsed_lines[first_surviving_line].1.len();
        first_surviving_line += 1;
    }
    if first_surviving_line > 0 {
        parsed_lines.drain(..first_surviving_line);
    }

    // A window with no usable line still consumed everything back to `start`, so the cutoff moves
    // there. Reporting `end_offset` instead would hand back the same window forever.
    let cutoff_offset = parsed_lines.first().map_or(start, |(offset, _)| *offset);
    let records = parsed_lines
        .into_iter()
        .flat_map(|(_, records)| records)
        .collect();

    Ok(ImportedTail {
        records,
        cutoff_offset,
        reached_start: cutoff_offset == 0,
    })
}
