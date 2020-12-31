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
    pub config: Option<Config>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_transport")]
    pub transport: Transport,
    pub ssh: Option<SshConfig>,
}
fn default_transport() -> Transport { Transport::Ssh }

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SshConfig {
    pub user: String,
    pub password: String,
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
