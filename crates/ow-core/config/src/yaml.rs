use crate::model::ContainerConfig;

pub fn parse_yaml(input: &str) -> Result<ContainerConfig, serde_yaml::Error> {
    serde_yaml::from_str(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_yaml() {
        let yaml = r#"
image: alpine:latest
process:
  args: ["/bin/echo", "hello"]
"#;
        let config = parse_yaml(yaml).unwrap();
        assert_eq!(config.image, "alpine:latest");
        assert_eq!(config.process.args, vec!["/bin/echo", "hello"]);
    }

    #[test]
    fn parse_full_yaml() {
        let yaml = r#"
image: alpine:latest
hostname: mycontainer
process:
  args: ["/bin/sh", "-c", "echo hello"]
  env:
    - PATH=/usr/bin
  working_dir: /app
resources:
  cpu_quota_us: 100000
  cpu_period_us: 100000
  memory_max: 268435456
network:
  enabled: true
"#;
        let config = parse_yaml(yaml).unwrap();
        assert_eq!(config.hostname, Some("mycontainer".into()));
        assert_eq!(config.resources.as_ref().unwrap().memory_max, Some(268435456));
        assert!(config.network.as_ref().unwrap().enabled);
    }
}
