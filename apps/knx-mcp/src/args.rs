//! Command-line parsing: which project files to serve, under which aliases.
//!
//! The command line is the server's whole configuration (ADR-0090). Tools
//! address projects only by the aliases given here and never take a path,
//! so the server cannot be talked into reading any other file.

use std::path::PathBuf;

/// The usage text, printed for `--help` and after every argument error.
pub const USAGE: &str = "\
usage: knx-mcp --project <alias>=<path.knxdb> [--project <alias>=<path.knxdb> ...]
               [--product-db <path> | --no-product-db]
       knx-mcp --help | --version

Serves the named KNXBench project files read-only to an MCP client over
stdio. Files are never written: an older project or product database is
migrated in memory only. No bus access, no network listener.

  --project <alias>=<path>  a saved .knxdb project; repeatable. The alias
                            (1-32 of A-Z a-z 0-9 _ -) is how tools name it.
  --product-db <path>       product database to explain parameters with;
                            default: the installed one, when present.
  --no-product-db           use no product database at all.";

/// Longest accepted alias, in bytes.
pub const MAX_ALIAS_LEN: usize = 32;

/// One `--project` argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectArg {
    pub alias: String,
    pub path: PathBuf,
}

/// Where the product database comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProductDbChoice {
    /// `knx_productdb::default_path()`, used only if the file exists.
    Default,
    /// An explicit path, which must open.
    Path(PathBuf),
    /// `--no-product-db`.
    Disabled,
}

/// A complete, validated configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub projects: Vec<ProjectArg>,
    pub product_db: ProductDbChoice,
}

/// What the command line asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Invocation {
    Serve(Config),
    Help,
    Version,
}

/// Parses the arguments after the program name.
pub fn parse(args: &[String]) -> Result<Invocation, String> {
    let mut projects: Vec<ProjectArg> = Vec::new();
    let mut product_db: Option<ProductDbChoice> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => return Ok(Invocation::Help),
            "--version" | "-V" => return Ok(Invocation::Version),
            "--project" => {
                let value = args.get(i + 1).ok_or("--project needs <alias>=<path>")?;
                let project = parse_project(value)?;
                if projects.iter().any(|p| p.alias == project.alias) {
                    return Err(format!("alias {:?} is given twice", project.alias));
                }
                projects.push(project);
                i += 2;
            }
            "--product-db" => {
                let value = args.get(i + 1).ok_or("--product-db needs a path")?;
                if product_db.is_some() {
                    return Err("--product-db/--no-product-db may be given only once".into());
                }
                product_db = Some(ProductDbChoice::Path(PathBuf::from(value)));
                i += 2;
            }
            "--no-product-db" => {
                if product_db.is_some() {
                    return Err("--product-db/--no-product-db may be given only once".into());
                }
                product_db = Some(ProductDbChoice::Disabled);
                i += 1;
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    if projects.is_empty() {
        return Err("at least one --project <alias>=<path> is required".into());
    }
    Ok(Invocation::Serve(Config {
        projects,
        product_db: product_db.unwrap_or(ProductDbChoice::Default),
    }))
}

fn parse_project(value: &str) -> Result<ProjectArg, String> {
    let (alias, path) = value
        .split_once('=')
        .ok_or_else(|| format!("--project {value:?}: expected <alias>=<path>"))?;
    validate_alias(alias)?;
    if path.is_empty() {
        return Err(format!("--project {value:?}: the path is empty"));
    }
    Ok(ProjectArg {
        alias: alias.to_string(),
        path: PathBuf::from(path),
    })
}

/// An alias is 1 to [`MAX_ALIAS_LEN`] ASCII letters, digits, `_` or `-`.
pub fn validate_alias(alias: &str) -> Result<(), String> {
    let well_formed = !alias.is_empty()
        && alias.len() <= MAX_ALIAS_LEN
        && alias
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if well_formed {
        Ok(())
    } else {
        Err(format!(
            "alias {alias:?} must be 1-{MAX_ALIAS_LEN} characters of A-Z a-z 0-9 _ -"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn projects_and_default_product_db() {
        let parsed = parse(&args(&[
            "--project",
            "home=/p/home.knxdb",
            "--project",
            "old=a=b",
        ]));
        let Ok(Invocation::Serve(config)) = parsed else {
            panic!("expected Serve, got {parsed:?}");
        };
        assert_eq!(config.projects[0].alias, "home");
        assert_eq!(config.projects[0].path, PathBuf::from("/p/home.knxdb"));
        // Only the first `=` separates; the rest belongs to the path.
        assert_eq!(config.projects[1].path, PathBuf::from("a=b"));
        assert_eq!(config.product_db, ProductDbChoice::Default);
    }

    #[test]
    fn product_db_choices() {
        let explicit = parse(&args(&["--project", "a=x", "--product-db", "/db"])).unwrap();
        assert!(matches!(
            explicit,
            Invocation::Serve(Config {
                product_db: ProductDbChoice::Path(_),
                ..
            })
        ));
        let disabled = parse(&args(&["--project", "a=x", "--no-product-db"])).unwrap();
        assert!(matches!(
            disabled,
            Invocation::Serve(Config {
                product_db: ProductDbChoice::Disabled,
                ..
            })
        ));
        assert!(parse(&args(&[
            "--project",
            "a=x",
            "--no-product-db",
            "--product-db",
            "y"
        ]))
        .is_err());
    }

    #[test]
    fn refusals() {
        assert!(parse(&args(&[])).is_err(), "no project");
        assert!(parse(&args(&["--project"])).is_err(), "missing value");
        assert!(parse(&args(&["--project", "nopath"])).is_err(), "no =");
        assert!(parse(&args(&["--project", "a="])).is_err(), "empty path");
        assert!(parse(&args(&["--project", "=x"])).is_err(), "empty alias");
        assert!(
            parse(&args(&["--project", "a b=x"])).is_err(),
            "space in alias"
        );
        assert!(
            parse(&args(&["--project", "../x=x"])).is_err(),
            "path-like alias"
        );
        assert!(
            parse(&args(&["--project", "a=x", "--project", "a=y"])).is_err(),
            "duplicate"
        );
        assert!(
            parse(&args(&["--project", "a=x", "--bus"])).is_err(),
            "unknown flag"
        );
        let long = format!("{}=x", "a".repeat(MAX_ALIAS_LEN + 1));
        assert!(
            parse(&args(&["--project", &long])).is_err(),
            "alias too long"
        );
    }

    #[test]
    fn help_and_version_win() {
        assert_eq!(parse(&args(&["--help"])).unwrap(), Invocation::Help);
        assert_eq!(
            parse(&args(&["--project", "a=x", "--version"])).unwrap(),
            Invocation::Version
        );
    }
}
