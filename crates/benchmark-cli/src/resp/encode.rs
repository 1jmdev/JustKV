#[derive(Clone, Debug)]
pub enum ExpectedResponse {
    Simple(&'static str),
    Bulk(Option<Vec<u8>>),
    Array(Vec<ExpectedResponse>),
}

pub fn encode_resp_parts(parts: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::with_capacity(parts.iter().map(|part| part.len() + 16).sum::<usize>() + 16);
    out.push(b'*');
    append_u64(&mut out, parts.len() as u64);
    out.extend_from_slice(b"\r\n");

    for part in parts {
        out.push(b'$');
        append_u64(&mut out, part.len() as u64);
        out.extend_from_slice(b"\r\n");
        out.extend_from_slice(part);
        out.extend_from_slice(b"\r\n");
    }
    out
}

pub fn encode_expected_response(expected: &ExpectedResponse) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    append_expected_response(&mut out, expected)?;
    Some(out)
}

fn append_expected_response(out: &mut Vec<u8>, expected: &ExpectedResponse) -> Option<()> {
    match expected {
        ExpectedResponse::Simple(value) => {
            out.push(b'+');
            out.extend_from_slice(value.as_bytes());
            out.extend_from_slice(b"\r\n");
        }
        ExpectedResponse::Bulk(None) => out.extend_from_slice(b"$-1\r\n"),
        ExpectedResponse::Bulk(Some(value)) => {
            out.push(b'$');
            append_u64(out, value.len() as u64);
            out.extend_from_slice(b"\r\n");
            out.extend_from_slice(value);
            out.extend_from_slice(b"\r\n");
        }
        ExpectedResponse::Array(items) => {
            out.push(b'*');
            append_u64(out, items.len() as u64);
            out.extend_from_slice(b"\r\n");
            for item in items {
                append_expected_response(out, item)?;
            }
        }
    }
    Some(())
}

pub(crate) fn append_u64(out: &mut Vec<u8>, value: u64) {
    let mut tmp = itoa::Buffer::new();
    out.extend_from_slice(tmp.format(value).as_bytes());
}
