use std::process;

pub struct CliArgs {
    pub profile: Option<String>,
}

pub fn parse_args() -> CliArgs {
    let mut args = std::env::args().skip(1);
    let mut profile = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--profile" => {
                let name = args.next().unwrap_or_else(|| {
                    eprintln!("Error: --profile requires a value");
                    eprintln!("Usage: gosuto [--profile <name>]");
                    process::exit(1);
                });
                if !is_valid_profile_name(&name) {
                    eprintln!(
                        "Error: invalid profile name '{name}'. \
                         Must match [a-zA-Z0-9_-]+, max 64 characters."
                    );
                    process::exit(1);
                }
                profile = Some(name);
            }
            _ => {
                eprintln!("Error: unknown argument '{arg}'");
                eprintln!("Usage: gosuto [--profile <name>]");
                process::exit(1);
            }
        }
    }

    CliArgs { profile }
}

fn is_valid_profile_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_profile_names() {
        assert!(is_valid_profile_name("work"));
        assert!(is_valid_profile_name("user-1"));
        assert!(is_valid_profile_name("test_profile"));
        assert!(is_valid_profile_name("A"));
        assert!(is_valid_profile_name(&"a".repeat(64)));
    }

    #[test]
    fn invalid_profile_names() {
        assert!(!is_valid_profile_name(""));
        assert!(!is_valid_profile_name(&"a".repeat(65)));
        assert!(!is_valid_profile_name("foo/bar"));
        assert!(!is_valid_profile_name("foo\\bar"));
        assert!(!is_valid_profile_name("foo bar"));
        assert!(!is_valid_profile_name("foo.bar"));
    }
}
