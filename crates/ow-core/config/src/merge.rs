use crate::model::ContainerConfig;

pub fn merge(base: &ContainerConfig, override_config: &ContainerConfig) -> ContainerConfig {
    ContainerConfig {
        image: if override_config.image.is_empty() {
            base.image.clone()
        } else {
            override_config.image.clone()
        },
        hostname: override_config.hostname.clone().or_else(|| base.hostname.clone()),
        process: if override_config.process.args.is_empty() {
            base.process.clone()
        } else {
            override_config.process.clone()
        },
        resources: override_config.resources.clone().or_else(|| base.resources.clone()),
        network: override_config.network.clone().or_else(|| base.network.clone()),
        security: override_config.security.clone().or_else(|| base.security.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::yaml::parse_yaml;

    #[test]
    fn override_takes_precedence() {
        let base = parse_yaml("image: alpine:latest\nprocess:\n  args: [\"/bin/sh\"]").unwrap();
        let over = parse_yaml("image: ubuntu:22.04\nprocess:\n  args: [\"/bin/bash\"]").unwrap();
        let merged = merge(&base, &over);
        assert_eq!(merged.image, "ubuntu:22.04");
        assert_eq!(merged.process.args, vec!["/bin/bash"]);
    }

    #[test]
    fn base_fills_gaps() {
        let base = parse_yaml("image: alpine:latest\nhostname: base\nprocess:\n  args: [\"/bin/sh\"]").unwrap();
        let over = parse_yaml("image: \"\"\nprocess:\n  args: []").unwrap();
        let merged = merge(&base, &over);
        assert_eq!(merged.image, "alpine:latest");
        assert_eq!(merged.hostname, Some("base".into()));
    }
}
