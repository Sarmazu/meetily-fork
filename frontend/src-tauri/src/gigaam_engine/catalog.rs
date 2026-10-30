use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Deserialize)]
pub struct FileSpec {
    pub path: String,
    pub size: u64,
    pub sha256: String,
    pub url: String,
}
#[derive(Clone)]
pub struct Package {
    pub name: String,
    pub files: Vec<FileSpec>,
}
impl Package {
    pub fn size(&self) -> u64 {
        self.files.iter().map(|f| f.size).sum()
    }
}
#[derive(Clone, Serialize, Debug, PartialEq)]
pub enum Status {
    Missing,
    Downloading,
    Available,
    Corrupted,
    Error,
}
#[derive(Clone, Serialize)]
pub struct ModelInfo {
    pub name: String,
    pub size_bytes: u64,
    pub status: Status,
    pub error: Option<String>,
}

pub fn catalog() -> Result<Vec<Package>, String> {
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tools/gigaam-model-contract/manifest.json"
    ))
    .map_err(|e| e.to_string())?;
    let files: Vec<FileSpec> =
        serde_json::from_value(value["files"].clone()).map_err(|e| e.to_string())?;
    ["v3_ctc", "v3_rnnt", "v3_e2e_ctc", "v3_e2e_rnnt"]
        .iter()
        .map(|id| {
            let paths = value["models"][*id]["files"]
                .as_array()
                .ok_or("Missing package")?;
            let mut names = HashSet::new();
            let mut selected = Vec::new();
            for path in paths {
                let path = path.as_str().ok_or("Invalid path")?;
                let name = path.strip_prefix("models/").ok_or("Unsafe package path")?;
                if name.is_empty()
                    || name.contains(['/', '\\', ':'])
                    || name == "."
                    || name == ".."
                    || !names.insert(name.to_ascii_lowercase())
                {
                    return Err("Unsafe or duplicate package filename".into());
                }
                let file = files
                    .iter()
                    .find(|f| f.path == path)
                    .ok_or("Missing file contract")?
                    .clone();
                if file.sha256.len() != 64 || !file.sha256.bytes().all(|c| c.is_ascii_hexdigit()) {
                    return Err("Invalid digest".into());
                }
                selected.push(file);
            }
            Ok(Package {
                name: id.to_string(),
                files: selected,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_manifest_contains_only_four_complete_fp32_packages() {
        let catalog = super::catalog().unwrap();
        assert_eq!(catalog.len(), 4);
        for package in catalog {
            assert!(package.size() > 1_000_000);
            assert!(package
                .files
                .iter()
                .any(|f| f.path.ends_with("LICENSE.txt")));
            assert!(package
                .files
                .iter()
                .any(|f| f.path.ends_with("config.json")));
            assert!(package.files.iter().all(|f| f.path.starts_with("models/")
                && f.url.contains("322c3b29492673eb7d0b434bfa9dfb8653e34d02")));
        }
    }
}
