use std::{fs, path::Path};
pub(super) struct StubApi {
    pub(super) sha: String,
    pub(super) missing_branch: Option<&'static str>,
    pub(super) error: Option<crate::core::github_download::GithubApiError>,
    /// Also serve `scripts/run.sh`, executable, as the production adapter
    /// lands a `100755` file (its modes restored from the trees answer).
    pub(super) executable_script: bool,
}

impl StubApi {
    pub(super) fn serving(sha: &str) -> Self {
        StubApi {
            sha: sha.to_string(),
            missing_branch: None,
            error: None,
            executable_script: false,
        }
    }

    pub(super) fn failing(status: u16) -> Self {
        StubApi {
            sha: "0".repeat(40),
            missing_branch: None,
            error: Some(crate::core::github_download::GithubApiError {
                status,
                reset_minutes: None,
                url: "stub".to_string(),
            }),
            executable_script: false,
        }
    }
}

impl crate::core::git_acquisition::GithubApi for StubApi {
    fn matching_refs(
        &self,
        _: &crate::core::git_acquisition::GithubRepo,
        _: &str,
    ) -> anyhow::Result<Vec<String>> {
        Ok(vec!["main".into(), "feature/x".into()])
    }

    fn branch_sha(
        &self,
        coords: &crate::core::git_acquisition::GithubCoords,
    ) -> anyhow::Result<String> {
        if self.missing_branch == Some(coords.branch.as_str()) {
            return Err(crate::core::github_download::GithubApiError {
                status: 404,
                reset_minutes: None,
                url: "stub".into(),
            }
            .into());
        }
        Ok(self.sha.clone())
    }

    fn download_directory(
        &self,
        _coords: &crate::core::git_acquisition::GithubCoords,
        dest: &Path,
        _cancel: Option<&crate::core::cancel_token::CancelToken>,
    ) -> anyhow::Result<()> {
        if let Some(err) = &self.error {
            return Err(anyhow::Error::new(err.clone()));
        }
        fs::create_dir_all(dest)?;
        fs::write(
            dest.join("SKILL.md"),
            "---\nname: fast-path-skill\ndescription: served by the API\n---\n",
        )?;
        if self.executable_script {
            let script = dest.join("scripts/run.sh");
            fs::create_dir_all(dest.join("scripts"))?;
            fs::write(&script, "#!/bin/sh\necho run\n")?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&script, fs::Permissions::from_mode(0o755))?;
            }
        }
        Ok(())
    }
}
