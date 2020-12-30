use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Inventory {
    pub groups: Vec<Group>,
    pub targets: Vec<Target>,
    pub config: Config,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Group {
    pub name: String,
    pub targets: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Target {
    pub name: String,
    pub uri: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub transport: Transport,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Transport {
    Ssh,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_with_transport() {
        let buf = r#"
---
targets: []
groups: []
config:
  transport: ssh"#;
        let _i: Inventory = serde_yaml::from_str(&buf).expect("Failed to deser");
    }
}
