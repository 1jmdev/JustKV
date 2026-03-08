pub(super) const FLAG_G: u16 = 1 << 0;
pub(super) const FLAG_DOLLAR: u16 = 1 << 1;
pub(super) const FLAG_L: u16 = 1 << 2;
pub(super) const FLAG_S: u16 = 1 << 3;
pub(super) const FLAG_H: u16 = 1 << 4;
pub(super) const FLAG_Z: u16 = 1 << 5;
pub(super) const FLAG_X: u16 = 1 << 6;
pub(super) const FLAG_KEYEVENT: u16 = 1 << 7;
pub(super) const FLAG_KEYSPACE: u16 = 1 << 8;
pub(super) const FLAG_A: u16 = 1 << 9;

pub(super) fn flag_to_mask(flag: u8) -> Option<u16> {
    let _trace = profiler::scope("server::pubsub::flag_to_mask");
    match flag {
        b'g' => Some(FLAG_G),
        b'$' => Some(FLAG_DOLLAR),
        b'l' => Some(FLAG_L),
        b's' => Some(FLAG_S),
        b'h' => Some(FLAG_H),
        b'z' => Some(FLAG_Z),
        b'x' => Some(FLAG_X),
        b'e' => Some(FLAG_KEYEVENT),
        b'K' => Some(FLAG_KEYSPACE),
        b'E' => Some(FLAG_KEYEVENT),
        b'A' => Some(FLAG_A),
        _ => None,
    }
}

pub(super) fn flags_to_mask(flags: &[u8]) -> Result<u16, ()> {
    let _trace = profiler::scope("server::pubsub::flags_to_mask");
    let mut mask = 0u16;
    for &flag in flags {
        let bit = flag_to_mask(flag).ok_or(())?;
        mask |= bit;
    }
    Ok(mask)
}

pub(super) fn mask_to_flags(mask: u16) -> Vec<u8> {
    let _trace = profiler::scope("server::pubsub::mask_to_flags");
    let mut out = Vec::new();
    if mask & FLAG_A != 0 {
        out.push(b'A');
    }
    if mask & FLAG_G != 0 {
        out.push(b'g');
    }
    if mask & FLAG_DOLLAR != 0 {
        out.push(b'$');
    }
    if mask & FLAG_L != 0 {
        out.push(b'l');
    }
    if mask & FLAG_S != 0 {
        out.push(b's');
    }
    if mask & FLAG_H != 0 {
        out.push(b'h');
    }
    if mask & FLAG_Z != 0 {
        out.push(b'z');
    }
    if mask & FLAG_X != 0 {
        out.push(b'x');
    }
    if mask & FLAG_KEYSPACE != 0 {
        out.push(b'K');
    }
    if mask & FLAG_KEYEVENT != 0 {
        out.push(b'E');
    }
    out
}

pub(super) fn notifications_enabled(mask: u16, class: u8) -> bool {
    let _trace = profiler::scope("server::pubsub::notifications_enabled");
    (mask & FLAG_A) != 0 || (flag_to_mask(class).is_some_and(|bit| (mask & bit) != 0))
}

pub(super) fn notifications_enabled_keyspace(mask: u16) -> bool {
    let _trace = profiler::scope("server::pubsub::notifications_enabled_keyspace");
    (mask & FLAG_KEYSPACE) != 0
}

pub(super) fn notifications_enabled_keyevent(mask: u16) -> bool {
    let _trace = profiler::scope("server::pubsub::notifications_enabled_keyevent");
    (mask & FLAG_KEYEVENT) != 0
}
