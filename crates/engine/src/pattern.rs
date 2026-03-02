pub(super) fn wildcard_match(pattern: &[u8], text: &[u8]) -> bool {
    let _trace = profiler::scope("engine::pattern::wildcard_match");
    let mut pi = 0;
    let mut ti = 0;
    let mut star = None;
    let mut star_match = 0;

    while ti < text.len() {
        if pi < pattern.len() && (pattern[pi] == text[ti] || pattern[pi] == b'?') {
            pi += 1;
            ti += 1;
            continue;
        }

        if pi < pattern.len() && pattern[pi] == b'*' {
            star = Some(pi);
            pi += 1;
            star_match = ti;
            continue;
        }

        match star {
            Some(position) => {
                pi = position + 1;
                star_match += 1;
                ti = star_match;
            }
            None => return false,
        }
    }

    while pi < pattern.len() && pattern[pi] == b'*' {
        pi += 1;
    }

    pi == pattern.len()
}
