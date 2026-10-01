//! Checks the resolved workspace graph for forbidden cross-layer dependencies.

use std::collections::{BTreeMap, BTreeSet};

/// A package dependency graph, keyed by package name.
#[derive(Debug, Default)]
pub struct DepGraph {
    pub edges: BTreeMap<String, Vec<String>>,
}

/// One forbidden package reachable from a root package.
#[derive(Debug, PartialEq, Eq)]
pub struct Violation {
    pub root: String,
    pub forbidden: String,
    /// The shortest dependency path from root to the forbidden package,
    /// including both ends. Printed so the reader can see how it got in.
    pub path: Vec<String>,
}

/// Breadth-first search from `root`, reporting every forbidden package that is
/// reachable, with the shortest path to it.
///
/// Breadth-first rather than depth-first so the reported path is the shortest
/// one, which is the most useful thing to show someone who has to remove it.
pub fn forbidden_reachable(graph: &DepGraph, root: &str, forbidden: &[&str]) -> Vec<Violation> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut queue: std::collections::VecDeque<Vec<String>> = std::collections::VecDeque::new();
    let mut violations = Vec::new();

    seen.insert(root);
    queue.push_back(vec![root.to_string()]);

    while let Some(path) = queue.pop_front() {
        let current = path.last().expect("path is never empty");

        if forbidden.contains(&current.as_str()) && current != root {
            violations.push(Violation {
                root: root.to_string(),
                forbidden: current.clone(),
                path: path.clone(),
            });
            continue;
        }

        let Some(deps) = graph.edges.get(current) else {
            continue;
        };
        for dep in deps {
            if seen.insert(dep.as_str()) {
                let mut next = path.clone();
                next.push(dep.clone());
                queue.push_back(next);
            }
        }
    }

    violations
}

/// Packages `knx-core` must never reach. Spec section 3.1, rule 1. `axum`
/// and `tower` are here alongside `tokio` so an accidental HTTP dependency
/// (e.g. from the web/Docker deployment target's `knx-server`) creeping
/// into a domain crate gets caught by this gate too.
pub const CORE_FORBIDDEN: &[&str] = &[
    "serde_json",
    "quick-xml",
    "rusqlite",
    "tokio",
    "axum",
    "tower",
];

/// Packages `knx-secure` must never reach (ADR-0008, `ARCHITECTURE.md` §9).
/// `knx-core` is forbidden so no type path can carry key material into the
/// project model; `serde` is forbidden so no `knx-secure` type can gain a
/// `Serialize`/`Deserialize` impl by way of a dependency that brings the
/// derive macros along, whether `knx-secure` depends on it directly or a
/// future dependency of `knx-secure`'s pulls it in transitively — the
/// resolved graph this check walks does not distinguish the two, which is
/// the point.
pub const SECURE_FORBIDDEN: &[&str] = &["knx-core", "serde"];

/// Build the resolved dependency graph of the whole workspace, including
/// transitive third-party dependencies.
pub fn workspace_graph(root: &std::path::Path) -> Result<DepGraph, String> {
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(root.join("Cargo.toml"))
        .current_dir(root)
        .exec()
        .map_err(|e| format!("cargo metadata failed: {e}"))?;

    let name_of: std::collections::HashMap<_, _> = metadata
        .packages
        .iter()
        .map(|p| (p.id.clone(), p.name.clone()))
        .collect();

    let resolve = metadata
        .resolve
        .ok_or_else(|| "cargo metadata returned no resolve graph".to_string())?;

    let mut edges = BTreeMap::new();
    for node in resolve.nodes {
        let Some(name) = name_of.get(&node.id) else {
            continue;
        };
        let deps = node
            .deps
            .iter()
            .filter_map(|d| name_of.get(&d.pkg).cloned())
            .collect();
        edges.insert(name.clone(), deps);
    }

    for root in [
        "knx-core",
        "knx-etsproj",
        "knx-productdb",
        "knx-projection",
        "knx-csv",
        "knx-report",
        "knx-diff",
        "knx-secure",
    ] {
        if !metadata
            .packages
            .iter()
            .any(|package| package.name == root && metadata.workspace_members.contains(&package.id))
            || !edges.contains_key(root)
        {
            return Err(format!(
                "missing layering root {root} in the resolved target graph"
            ));
        }
    }
    Ok(DepGraph { edges })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(edges: &[(&str, &[&str])]) -> DepGraph {
        DepGraph {
            edges: edges
                .iter()
                .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
                .collect(),
        }
    }

    #[test]
    fn clean_graph_has_no_violations() {
        let g = graph(&[("knx-core", &["thiserror"]), ("thiserror", &[])]);
        assert_eq!(forbidden_reachable(&g, "knx-core", &["rusqlite"]), vec![]);
    }

    #[test]
    fn direct_forbidden_dependency_is_reported_with_path() {
        let g = graph(&[("knx-core", &["rusqlite"]), ("rusqlite", &[])]);
        let found = forbidden_reachable(&g, "knx-core", &["rusqlite"]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].forbidden, "rusqlite");
        assert_eq!(found[0].path, vec!["knx-core", "rusqlite"]);
    }

    #[test]
    fn transitive_forbidden_dependency_is_reported_with_full_path() {
        let g = graph(&[
            ("knx-core", &["fancy-parser"]),
            ("fancy-parser", &["quick-xml"]),
            ("quick-xml", &[]),
        ]);
        let found = forbidden_reachable(&g, "knx-core", &["quick-xml"]);
        assert_eq!(found[0].path, vec!["knx-core", "fancy-parser", "quick-xml"]);
    }

    #[test]
    fn dependency_cycles_do_not_hang() {
        let g = graph(&[("a", &["b"]), ("b", &["a"])]);
        assert_eq!(forbidden_reachable(&g, "a", &["tokio"]), vec![]);
    }
}
