pub fn hash_from_string(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

pub fn hash_to_string(hash: &[u8]) -> String {
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}
