use justkv::protocol::types::RespFrame;

pub fn render(frame: &RespFrame, raw: bool) -> String {
    if raw {
        return render_raw(frame);
    }
    render_human(frame)
}

fn render_raw(frame: &RespFrame) -> String {
    match frame {
        RespFrame::Simple(value) | RespFrame::Error(value) => value.clone(),
        RespFrame::SimpleStatic(value) | RespFrame::ErrorStatic(value) => value.to_string(),
        RespFrame::Integer(value) => value.to_string(),
        RespFrame::Bulk(Some(value)) => String::from_utf8_lossy(value.as_slice()).to_string(),
        RespFrame::Bulk(None) | RespFrame::Array(None) => String::new(),
        RespFrame::BulkValues(values) => values
            .iter()
            .map(|v| String::from_utf8_lossy(v.as_slice()).to_string())
            .collect::<Vec<String>>()
            .join("\n"),
        RespFrame::Array(Some(items)) => items
            .iter()
            .map(render_raw)
            .collect::<Vec<String>>()
            .join("\n"),
        RespFrame::Map(entries) => entries
            .iter()
            .map(|(key, value)| format!("{}\n{}", render_raw(key), render_raw(value)))
            .collect::<Vec<String>>()
            .join("\n"),
        // PreEncoded is a server-side write optimization; the CLI never produces it.
        RespFrame::PreEncoded(_) => unreachable!("PreEncoded is a server-only variant"),
    }
}

fn render_human(frame: &RespFrame) -> String {
    match frame {
        RespFrame::Simple(value) => value.clone(),
        RespFrame::SimpleStatic(value) => value.to_string(),
        RespFrame::Error(value) => format!("(error) {value}"),
        RespFrame::ErrorStatic(value) => format!("(error) {value}"),
        RespFrame::Integer(value) => format!("(integer) {value}"),
        RespFrame::Bulk(None) | RespFrame::Array(None) => "(nil)".to_string(),
        RespFrame::Bulk(Some(value)) => {
            format!("\"{}\"", String::from_utf8_lossy(value.as_slice()))
        }
        RespFrame::BulkValues(values) => {
            if values.is_empty() {
                return "(empty array)".to_string();
            }
            let mut out = String::new();
            for (index, value) in values.iter().enumerate() {
                let rendered = format!("\"{}\"", String::from_utf8_lossy(value.as_slice()));
                out.push_str(&format!("{}) {}\n", index + 1, rendered));
            }
            out.trim_end().to_string()
        }
        RespFrame::Array(Some(items)) => {
            if items.is_empty() {
                return "(empty array)".to_string();
            }
            let mut out = String::new();
            for (index, item) in items.iter().enumerate() {
                let rendered = render_human(item);
                let line = rendered.replace('\n', "\n   ");
                out.push_str(&format!("{}) {}\n", index + 1, line));
            }
            out.trim_end().to_string()
        }
        RespFrame::Map(entries) => {
            if entries.is_empty() {
                return "(empty map)".to_string();
            }
            let mut out = String::new();
            for (index, (key, value)) in entries.iter().enumerate() {
                let key_text = render_human(key).replace('\n', "\n   ");
                let value_text = render_human(value).replace('\n', "\n   ");
                out.push_str(&format!("{}) {} => {}\n", index + 1, key_text, value_text));
            }
            out.trim_end().to_string()
        }
        // PreEncoded is a server-side write optimization; the CLI never produces it.
        RespFrame::PreEncoded(_) => unreachable!("PreEncoded is a server-only variant"),
    }
}
