use std::collections::HashMap;
use std::path::Path;

use axoasset::LocalAsset;
use camino::Utf8Path;

use crate::config::Config;
use crate::data::Context;
use crate::errors::*;
use crate::message::{Message, MessageType};

pub mod artifacts;
pub mod icons;
pub mod layout;
use layout::{css, javascript, Layout};
pub mod link;
pub mod markdown;
pub mod page;
use page::Page;
pub mod changelog;

#[derive(Debug)]
pub struct Site {
    pages: Vec<Page>,
}

impl Site {
    pub fn build(config: &Config) -> Result<Site> {
        Self::clean_dist_dir(&config.dist_dir)?;

        let mut pages = vec![];
        let layout_template = Layout::new(config)?;

        if let Some(files) = &config.additional_pages {
            let mut additional_pages =
                Self::build_additional_pages(files, &layout_template, config)?;
            pages.append(&mut additional_pages);
        }

        let mut index = Page::index(&layout_template, config)?;

        if Self::needs_context(config) {
            match &config.repository {
                Some(repo_url) => {
                let context = Context::new(repo_url, config.artifacts.cargo_dist)?;
                if config.artifacts.has_some() {
                    index = Page::index_with_artifacts(&context, &layout_template, config)?;
                    if context.latest_dist_release.is_some()
                        || config.artifacts.package_managers.is_some()
                    {
                        let body = artifacts::page(&context, config)?;
                        let artifacts_page = Page::new_from_contents(
                            body,
                            "artifacts.html",
                            &layout_template,
                            config,
                        );
                        pages.push(artifacts_page);
                    }
                }
                if config.changelog {
                    let mut changelog_pages = Self::build_changelog_pages(&context, &layout_template, config)?;
                    pages.append(&mut changelog_pages);
                }
            },
            None => Err(OrandaError::Other("You have indicated you want to use features that require a repository context. Please make sure you have a repo listed in your project or oranda config.".to_string()))?
            }
        }

        pages.push(index);
        Ok(Site { pages })
    }

    fn needs_context(config: &Config) -> bool {
        config.artifacts.has_some() || config.changelog
    }

    fn build_additional_pages(
        files: &HashMap<String, String>,
        layout_template: &Layout,
        config: &Config,
    ) -> Result<Vec<Page>> {
        let mut pages = vec![];
        for file_path in files.values() {
            if page::source::is_markdown(file_path) {
                let additional_page = Page::new_from_file(file_path, layout_template, config)?;
                pages.push(additional_page)
            } else {
                let msg = format!(
                    "File {} in additional pages is not markdown and will be skipped",
                    file_path
                );
                Message::new(MessageType::Warning, &msg).print();
            }
        }
        Ok(pages)
    }

    fn build_changelog_pages(
        context: &Context,
        layout_template: &Layout,
        config: &Config,
    ) -> Result<Vec<Page>> {
        let mut pages = vec![];
        let changelog_html = changelog::build(context, config)?;
        let changelog_page =
            Page::new_from_contents(changelog_html, "changelog.html", layout_template, config);
        let changelog_releases = changelog::build_all(context, config)?;
        pages.push(changelog_page);
        for (name, content) in changelog_releases {
            let page = Page::new_from_contents(
                content,
                &format!("changelog/{}.html", name),
                layout_template,
                config,
            );
            pages.push(page);
        }
        Ok(pages)
    }

    pub fn copy_static(dist_path: &String, static_path: &String) -> Result<()> {
        let mut options = fs_extra::dir::CopyOptions::new();
        options.overwrite = true;
        fs_extra::copy_items(&[static_path], dist_path, &options)?;

        Ok(())
    }

    pub fn write(self, config: &Config) -> Result<()> {
        let dist = &config.dist_dir;
        for page in self.pages {
            // FIXME: We have to do some gymnastics here due to `LocalAsset::write_new_all` taking filename and dest
            // path separately. Hopefully in a future version of axoasset, this only takes one parameter instead of
            // two, and we can just add the page filename to the dest path and pass it in.
            let full_path = Utf8Path::new(&config.dist_dir).join(&page.filename);
            LocalAsset::write_new_all(
                &page.contents,
                full_path.file_name().unwrap(),
                full_path.parent().unwrap().as_str(),
            )?;
        }
        if let Some(book_path) = &config.md_book {
            Self::copy_static(dist, book_path)?;
        }
        if Path::new(&config.static_dir).exists() {
            Self::copy_static(dist, &config.static_dir)?;
        }
        javascript::write_os_script(&config.dist_dir)?;
        if !config.additional_css.is_empty() {
            css::write_additional(&config.additional_css, &config.dist_dir)?;
        }

        Ok(())
    }

    pub fn clean_dist_dir(dist_path: &str) -> Result<()> {
        if Path::new(dist_path).exists() {
            std::fs::remove_dir_all(dist_path)?;
        }
        match std::fs::create_dir_all(dist_path) {
            Ok(_) => Ok(()),
            Err(e) => Err(OrandaError::DistDirCreationError {
                dist_path: dist_path.to_string(),
                details: e,
            }),
        }
    }
}
