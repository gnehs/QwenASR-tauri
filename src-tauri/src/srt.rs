use crate::models::TranscriptSegment;

pub fn format_time(ms: u64) -> String {
    let hours = ms / 3_600_000;
    let minutes = (ms % 3_600_000) / 60_000;
    let seconds = (ms % 60_000) / 1_000;
    let millis = ms % 1_000;
    format!("{hours:02}:{minutes:02}:{seconds:02},{millis:03}")
}

pub fn render(segments: &[TranscriptSegment]) -> String {
    let mut output = String::new();
    let mut index = 1usize;

    for segment in segments {
        let text = segment.text.trim();
        if text.is_empty() {
            continue;
        }

        output.push_str(&index.to_string());
        output.push('\n');
        output.push_str(&format_time(segment.start_ms));
        output.push_str(" --> ");
        output.push_str(&format_time(segment.end_ms));
        output.push('\n');
        output.push_str(text);
        output.push_str("\n\n");
        index += 1;
    }

    output
}
