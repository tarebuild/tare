use super::*;
const MAX_EXACT_INTEGER: u64 = (1u64 << 53) - 1;

pub fn canonical_repository(value: &str) -> &str {
    let value = value.trim_end_matches('/');
    value.strip_suffix(".git").unwrap_or(value).trim_end_matches('/')
}

pub fn canonicalize_source(source: &mut Value) {
    if let Some(repository) = source["repository"].as_str() {
        let repository = canonical_repository(repository).to_owned();
        source["repository"] = Value::String(repository);
    }
}

pub fn document(value: &Value) -> Value {
    match value {
        Value::Number(number) if number.as_u64().is_some_and(|n| n > MAX_EXACT_INTEGER) => Value::String(number.to_string()),
        Value::Array(values) => Value::Array(values.iter().map(document).collect()),
        Value::Object(values) => Value::Object(values.iter().map(|(key,value)| (key.clone(),document(value))).collect()),
        _ => value.clone(),
    }
}
pub fn integer(value: &Value) -> Option<u64> {
    value.as_u64().or_else(||value.as_str()?.parse().ok())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repository_suffixes_have_one_canonical_form() {
        let expected = "https://github.com/solana-foundation/anchor";
        for value in [expected.to_owned(), format!("{expected}/"), format!("{expected}///"), format!("{expected}.git"), format!("{expected}.git/")] {
            assert_eq!(canonical_repository(&value), expected);
        }
        assert_eq!(canonical_repository("https://example.com/other.git/repo/"), "https://example.com/other.git/repo");
    }
    #[test]
    fn source_normalization_preserves_other_provenance() {
        let mut source = json!({"repository":"https://example.com/repo.git/","commit":"abc"});
        canonicalize_source(&mut source);
        assert_eq!(source, json!({"repository":"https://example.com/repo","commit":"abc"}));
        canonicalize_source(&mut source);
        assert_eq!(source["repository"], "https://example.com/repo");
    }
    #[test]
    fn exact_u64_document_roundtrip() {
        let wire=document(&json!({"lamports":9007199254740992u64,"rentEpoch":u64::MAX,"slot":42,"nested":[u64::MAX]}));
        assert_eq!(wire["rentEpoch"],"18446744073709551615");
        assert_eq!(wire["slot"],42);
        assert_eq!(integer(&wire["lamports"]),Some(9007199254740992));
    }
}
