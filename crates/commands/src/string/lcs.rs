use crate::util::{Args, int_error, syntax_error, wrong_args, wrong_type};
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};

pub(crate) fn lcs(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::lcs::lcs");
    if args.len() < 3 {
        return wrong_args("LCS");
    }

    let options = match parse_options(args) {
        Ok(options) => options,
        Err(response) => return response,
    };

    let result = match store.lcs(&args[1], &args[2]) {
        Ok(result) => result,
        Err(_) => return wrong_type(),
    };

    if options.idx {
        let sequence_len = result.sequence.len() as i64;
        let matches = result
            .matches
            .into_iter()
            .filter(|matched| matched.len >= options.min_match_len)
            .map(|matched| {
                let mut entry = Vec::with_capacity(if options.with_match_len { 3 } else { 2 });
                entry.push(RespFrame::Array(Some(vec![
                    RespFrame::Integer(matched.first.0 as i64),
                    RespFrame::Integer(matched.first.1 as i64),
                ])));
                entry.push(RespFrame::Array(Some(vec![
                    RespFrame::Integer(matched.second.0 as i64),
                    RespFrame::Integer(matched.second.1 as i64),
                ])));
                if options.with_match_len {
                    entry.push(RespFrame::Integer(matched.len as i64));
                }
                RespFrame::Array(Some(entry))
            })
            .collect();

        return RespFrame::Map(vec![
            (
                RespFrame::Bulk(Some(BulkData::from_vec(b"matches".to_vec()))),
                RespFrame::Array(Some(matches)),
            ),
            (
                RespFrame::Bulk(Some(BulkData::from_vec(b"len".to_vec()))),
                RespFrame::Integer(sequence_len),
            ),
        ]);
    }

    if options.len_only {
        return RespFrame::Integer(result.sequence.len() as i64);
    }

    RespFrame::Bulk(Some(BulkData::from_vec(result.sequence)))
}

#[derive(Clone, Copy, Default)]
struct LcsOptions {
    len_only: bool,
    idx: bool,
    min_match_len: usize,
    with_match_len: bool,
}

fn parse_options(args: &Args) -> Result<LcsOptions, RespFrame> {
    let mut options = LcsOptions::default();
    let mut index = 3;

    while index < args.len() {
        let option = args[index].as_slice();
        if option.eq_ignore_ascii_case(b"LEN") {
            if options.len_only {
                return Err(syntax_error());
            }
            options.len_only = true;
        } else if option.eq_ignore_ascii_case(b"IDX") {
            if options.idx {
                return Err(syntax_error());
            }
            options.idx = true;
        } else if option.eq_ignore_ascii_case(b"MINMATCHLEN") {
            if options.min_match_len != 0 {
                return Err(syntax_error());
            }
            index += 1;
            if index >= args.len() {
                return Err(syntax_error());
            }
            let Some(value) = crate::util::parse_u64_bytes(&args[index]) else {
                return Err(int_error());
            };
            let Some(value) = usize::try_from(value).ok() else {
                return Err(int_error());
            };
            options.min_match_len = value;
        } else if option.eq_ignore_ascii_case(b"WITHMATCHLEN") {
            if options.with_match_len {
                return Err(syntax_error());
            }
            options.with_match_len = true;
        } else {
            return Err(syntax_error());
        }
        index += 1;
    }

    if !options.idx && (options.min_match_len != 0 || options.with_match_len) {
        return Err(syntax_error());
    }

    Ok(options)
}
