use crate::RehashingMap;

fn bytes(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

#[test]
fn batch_lookup_matches_single_lookup() {
    let mut map = RehashingMap::new();
    map.insert(bytes("alpha"), 1u32);
    map.insert(bytes("beta"), 2u32);
    map.insert(bytes("gamma"), 3u32);
    map.insert(bytes("delta"), 4u32);
    map.insert(bytes("epsilon"), 5u32);

    let keys: [&[u8]; 6] = [
        b"alpha", b"missing", b"gamma", b"delta", b"nope", b"epsilon",
    ];
    let expected = keys.map(|key| map.get(key).copied());
    let actual = map.get_batch(&keys).map(|value| value.copied());

    assert_eq!(actual, expected);
}

#[test]
fn handles_same_hash_different_lengths() {
    let mut map = RehashingMap::new();
    map.insert(bytes("a"), 1u32);
    map.insert(bytes("aa"), 2u32);
    map.insert(bytes("aaa"), 3u32);

    assert_eq!(map.get(b"a" as &[u8]), Some(&1));
    assert_eq!(map.get(b"aa" as &[u8]), Some(&2));
    assert_eq!(map.get(b"aaa" as &[u8]), Some(&3));
    assert_eq!(map.get(b"aaaa" as &[u8]), None);
}

#[test]
fn remove_keeps_remaining_chain_reachable() {
    let mut map = RehashingMap::new();
    let items = [
        ("k0", 0u32),
        ("k1", 1),
        ("k2", 2),
        ("k3", 3),
        ("k4", 4),
        ("k5", 5),
        ("k6", 6),
        ("k7", 7),
    ];

    for (key, value) in items {
        map.insert(bytes(key), value);
    }

    assert_eq!(map.remove(b"k3" as &[u8]), Some(3));
    assert_eq!(map.remove(b"k0" as &[u8]), Some(0));

    for (key, value) in [
        ("k1", 1u32),
        ("k2", 2),
        ("k4", 4),
        ("k5", 5),
        ("k6", 6),
        ("k7", 7),
    ] {
        assert_eq!(map.get(key.as_bytes()), Some(&value));
    }
}

#[test]
fn growth_uses_incremental_rehashing() {
    let mut map = RehashingMap::new();

    for i in 0..65u32 {
        map.insert(i.to_le_bytes().to_vec(), i);
    }

    assert!(map.old_table.is_some());
    assert_eq!(map.rehash_cursor, 0);

    for i in 0..65u32 {
        assert_eq!(map.get(&i.to_le_bytes()), Some(&i));
    }

    while map.old_table.is_some() {
        map.insert(
            format!("extra-{i}", i = map.len()).into_bytes(),
            map.len() as u32,
        );
    }

    for i in 0..65u32 {
        assert_eq!(map.get(&i.to_le_bytes()), Some(&i));
    }
}
