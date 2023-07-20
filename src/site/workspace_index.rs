use crate::config::Config;
use crate::data::workspaces::WorkspaceData;
use crate::errors::Result;
use crate::site::link::determine_path;
use camino::Utf8PathBuf;
use indexmap::IndexMap;
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct WorkspaceIndexContext {
    pub members: IndexMap<String, WorkspaceIndexMember>,
}

#[derive(Serialize, Debug)]
pub struct WorkspaceIndexMember {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub repository: Option<String>,
    pub logo: Option<Utf8PathBuf>,
}

impl WorkspaceIndexContext {
    pub fn new(members: &Vec<WorkspaceData>, workspace_config: &Config) -> Result<Self> {
        let mut map = IndexMap::new();
        for member in members {
            let logo = if member.config.styles.logo.is_some() {
                Some(Self::find_logo_path(member, workspace_config)?)
            } else {
                None
            };
            let context = WorkspaceIndexMember {
                name: member.config.project.name.clone(),
                slug: member.slug.clone(),
                description: member.config.project.description.clone(),
                repository: member.config.project.repository.clone(),
                logo,
            };
            map.insert(member.slug.clone(), context);
        }

        Ok(Self { members: map })
    }

    fn find_logo_path(member: &WorkspaceData, workspace_config: &Config) -> Result<Utf8PathBuf> {
        let logo_url = member.config.styles.logo.clone().unwrap();
        let root_path = Utf8PathBuf::from_path_buf(std::env::current_dir()?.canonicalize()?)
            .unwrap_or(Utf8PathBuf::new());
        if logo_url.starts_with("http") {
            // Lifted from axoasset. Expose it there?
            let mut filename = url::Url::parse(&logo_url)
                .unwrap()
                .path()
                .to_string()
                .replace('/', "_");
            filename.remove(0);
            let mut dp = Utf8PathBuf::from(&member.slug);
            dp.push(filename);
            Ok(dp)
        } else {
            determine_path(root_path, &Some(&member.slug), logo_url)
        }
    }
}
