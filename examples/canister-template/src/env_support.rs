//! `examples/canister-template` keeps env var parsing and JS injection helpers here.
//! Splitting these helpers keeps the canister endpoint file below the line limit.

use ic_edge_store::{EdgeStore, StableEdgeStore};

pub(super) fn read_env_names(store: &StableEdgeStore) -> Vec<String> {
    store
        .get_kv("__env_names")
        .ok()
        .flatten()
        .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
        .unwrap_or_default()
        .lines()
        .filter(|name| valid_env_name(name))
        .map(str::to_string)
        .collect()
}

pub(super) fn insert_env_name(names: &str, name: &str) -> String {
    let mut names = names
        .lines()
        .filter(|entry| valid_env_name(entry))
        .map(str::to_string)
        .collect::<Vec<_>>();
    if !names.iter().any(|entry| entry == name) {
        names.push(name.to_string());
    }
    names.sort();
    names.dedup();
    names.join("\n")
}

pub(super) fn valid_env_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

pub(super) fn env_assignment(name: &str, value: &str) -> String {
    let name = serde_json::to_string(name).expect("env names are serializable");
    let value = serde_json::to_string(value).expect("env values are serializable");
    format!("globalThis.process.env[{name}] = {value};")
}

#[cfg(test)]
mod tests {
    use super::{env_assignment, insert_env_name, valid_env_name};

    #[test]
    fn env_name_allows_uppercase_digits_and_underscore() {
        assert!(valid_env_name("OPENAI_API_KEY"));
        assert!(valid_env_name("UPSTASH_REDIS_REST_URL2"));
        assert!(!valid_env_name(""));
        assert!(!valid_env_name("OpenAI"));
        assert!(!valid_env_name("TOKEN-NAME"));
    }

    #[test]
    fn env_index_is_sorted_unique_and_filtered() {
        assert_eq!(insert_env_name("B\nbad-name\nA\n", "C"), "A\nB\nC");
        assert_eq!(insert_env_name("A\nB", "A"), "A\nB".to_string());
    }

    #[test]
    fn env_assignment_uses_json_escaping() {
        assert_eq!(
            env_assignment("OPENAI_API_KEY", "line\n\"quoted\""),
            "globalThis.process.env[\"OPENAI_API_KEY\"] = \"line\\n\\\"quoted\\\"\";"
        );
    }
}
