use crate::utils::{Error, ListOpt, Listable, Result, INTERNAL_ERR, TERM_OUT};
use cargo_metadata::{Metadata, PackageId};
use console::style;
use semver::Version;
use serde::Serialize;
use serde_json::Value;
use std::{cmp::max, path::PathBuf};

#[derive(Serialize, Debug, Clone, Ord, Eq, PartialOrd, PartialEq)]
pub struct Pkg {
    #[serde(skip)]
    pub id: PackageId,
    pub name: String,
    pub version: Version,
    pub location: PathBuf,
    #[serde(skip)]
    pub path: String,
    pub private: bool,
    pub independent: bool,
}

impl Listable for Vec<Pkg> {
    fn list(&self, list: ListOpt) -> Result {
        if list.json {
            return self.json();
        }

        if self.is_empty() {
            return Ok(());
        }

        let first = self.iter().map(|x| x.name.len()).max().expect(INTERNAL_ERR);
        let second = self
            .iter()
            .map(|x| x.version.to_string().len() + 1)
            .max()
            .expect(INTERNAL_ERR);
        let third = self
            .iter()
            .map(|x| max(1, x.path.len()))
            .max()
            .expect(INTERNAL_ERR);

        for pkg in self {
            TERM_OUT.write_str(&pkg.name)?;
            let mut width = first - pkg.name.len();

            if list.long {
                let path = if pkg.path.is_empty() { "." } else { &pkg.path };

                TERM_OUT.write_str(&format!(
                    "{:f$} {}{:s$} {}",
                    "",
                    style(format!("v{}", pkg.version)).green(),
                    "",
                    style(path).black().bright(),
                    f = width,
                    s = second - pkg.version.to_string().len() - 1,
                ))?;

                width = third - pkg.path.len();
            }

            if list.all && pkg.private {
                TERM_OUT.write_str(&format!(
                    "{:w$} ({})",
                    "",
                    style("PRIVATE").red(),
                    w = width
                ))?;
            }

            TERM_OUT.write_line("")?;
        }

        Ok(())
    }
}

fn is_independent(metadata: &Value) -> bool {
    if let Value::Object(v) = metadata {
        if let Some(Value::Object(v)) = v.get("workspaces") {
            if let Some(Value::Bool(v)) = v.get("independent") {
                return *v;
            }
        }
    }

    false
}

pub fn get_pkgs(metadata: &Metadata, all: bool) -> Result<Vec<Pkg>> {
    let mut pkgs = vec![];

    for id in &metadata.workspace_members {
        if let Some(pkg) = metadata.packages.iter().find(|x| x.id == *id) {
            let private =
                pkg.publish.is_some() && pkg.publish.as_ref().expect(INTERNAL_ERR).is_empty();

            if !all && private {
                continue;
            }

            let loc = pkg.manifest_path.strip_prefix(&metadata.workspace_root);

            if loc.is_err() {
                return Err(Error::PackageNotInWorkspace {
                    id: pkg.id.repr.clone(),
                    ws: metadata.workspace_root.to_string_lossy().to_string(),
                });
            }

            let loc = loc.expect(INTERNAL_ERR).to_string_lossy();
            let loc = loc
                .trim_end_matches("Cargo.toml")
                .trim_end_matches("/")
                .trim_end_matches("\\");

            pkgs.push(Pkg {
                id: pkg.id.clone(),
                name: pkg.name.clone(),
                version: pkg.version.clone(),
                location: metadata.workspace_root.join(loc),
                path: loc.to_string(),
                private,
                independent: is_independent(&pkg.metadata),
            });
        } else {
            Error::PackageNotFound {
                id: id.repr.clone(),
            }
            .print_err()?;
        }
    }

    if pkgs.is_empty() {
        return Err(Error::EmptyWorkspace);
    }

    pkgs.sort();
    Ok(pkgs)
}
