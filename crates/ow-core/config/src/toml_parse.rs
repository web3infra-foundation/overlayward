use crate::model::ContainerConfig;

pub fn parse_toml(input: &str) -> Result<ContainerConfig, toml::de::Error> {
    toml::from_str(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_toml() {
        let toml_str = r#"
image = "alpine:latest"

[process]
args = ["/bin/echo", "hello"]
"#;
        let config = parse_toml(toml_str).unwrap();
        assert_eq!(config.image, "alpine:latest");
    }
}
