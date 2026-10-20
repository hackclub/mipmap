use crate::{
    env,
    error::{ApiError, reject},
    http::CLIENT,
};
use reqwest::{StatusCode, Url};
use serde::Deserialize;

#[derive(Clone, Copy)]
enum Host {
    GitHub,
    GitLab,
    Codeberg,
}

const HOSTS: [Host; 3] = [Host::GitHub, Host::GitLab, Host::Codeberg];
const GITLAB_ROUTES: [&str; 18] = [
    "-", "badges", "blame", "blob", "builds", "commits", "create", "create_dir", "edit", "files",
    "find_file", "new", "preview", "raw", "refs", "tree", "update", "wikis",
];

impl Host {
    fn domain(self) -> &'static str {
        match self {
            Host::GitHub => "github.com",
            Host::GitLab => "gitlab.com",
            Host::Codeberg => "codeberg.org",
        }
    }
}

pub struct Repo {
    host: Host,
    path: String,
}

impl Repo {
    pub fn url(&self) -> String {
        format!("https://{}/{}", self.host.domain(), self.path)
    }

    pub fn key(&self) -> String {
        format!("{}/{}", self.host.domain(), self.path).to_lowercase()
    }

    pub fn root(&self) -> String {
        let owner_and_name: Vec<&str> = self.path.splitn(3, '/').take(2).collect();
        format!("{}/{}", self.host.domain(), owner_and_name.join("/")).to_lowercase()
    }

    pub fn overlaps(&self, other: &Repo) -> bool {
        let (key, other) = (self.key(), other.key());
        key == other || key.starts_with(&format!("{other}/")) || other.starts_with(&format!("{key}/"))
    }
}

pub fn parse(link: &str) -> Result<Repo, ApiError> {
    let link = link.trim();
    if link.is_empty() {
        return Err(reject("Add a repository link first"));
    }
    let url = Url::parse(link)
        .or_else(|_| Url::parse(&format!("https://{link}")))
        .map_err(|_| reject("The repository link isn't a valid URL"))?;
    let host = url
        .host_str()
        .map(|host| host.trim_start_matches("www."))
        .and_then(|host| HOSTS.into_iter().find(|known| known.domain() == host))
        .ok_or_else(|| reject("The repository has to be on GitHub, GitLab or Codeberg"))?;
    let mut segments: Vec<&str> = url
        .path_segments()
        .into_iter()
        .flatten()
        .filter(|segment| !segment.is_empty())
        .collect();
    match host {
        Host::GitLab => {
            if let Some(end) = segments
                .iter()
                .skip(2)
                .position(|segment| GITLAB_ROUTES.contains(segment))
            {
                segments.truncate(end + 2);
            }
        }
        _ => segments.truncate(2),
    }
    let valid = |segment: &str| {
        segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    };
    if segments.len() < 2 || !segments.iter().all(|segment| valid(segment)) {
        return Err(reject("The repository link should point at a repository"));
    }
    if let Some(last) = segments.last_mut() {
        *last = last.strip_suffix(".git").unwrap_or(last);
    }
    Ok(Repo {
        host,
        path: segments.join("/"),
    })
}

pub async fn has_readme(repo: &Repo) -> anyhow::Result<bool> {
    let request = match repo.host {
        Host::GitHub => {
            let request = CLIENT
                .get(format!("https://api.github.com/repos/{}/readme", repo.path))
                .header("Accept", "application/vnd.github+json");
            match env::var("GITHUB_TOKEN") {
                Some(token) => request.bearer_auth(token),
                None => request,
            }
        }
        Host::GitLab => CLIENT.get(format!(
            "https://gitlab.com/api/v4/projects/{}",
            repo.path.replace('/', "%2F")
        )),
        Host::Codeberg => CLIENT.get(format!(
            "https://codeberg.org/api/v1/repos/{}/contents",
            repo.path
        )),
    };
    let response = request.send().await?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(false);
    }
    let response = response.error_for_status()?;
    Ok(match repo.host {
        Host::GitHub => true,
        Host::GitLab => {
            #[derive(Deserialize)]
            struct Project {
                readme_url: Option<String>,
            }
            response.json::<Project>().await?.readme_url.is_some()
        }
        Host::Codeberg => {
            #[derive(Deserialize)]
            struct Entry {
                name: String,
            }
            response
                .json::<Vec<Entry>>()
                .await?
                .iter()
                .any(|entry| entry.name.to_lowercase().starts_with("readme"))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(link: &str) -> Repo {
        parse(link).ok().unwrap()
    }

    #[test]
    fn overlaps() {
        let base = repo("https://gitlab.com/a/b");
        for same in ["gitlab.com/a/b/tree/main", "https://gitlab.com/A/B/x", "gitlab.com/a/b/-/blob/main/x", "gitlab.com/a/b.git"] {
            assert!(repo(same).overlaps(&base), "{same}");
            assert!(base.overlaps(&repo(same)), "{same}");
        }
        for other in ["gitlab.com/a/bc", "gitlab.com/a/c", "github.com/a/b"] {
            assert!(!repo(other).overlaps(&base), "{other}");
        }
        assert_eq!(repo("gitlab.com/a/b/c").root(), "gitlab.com/a/b");
        assert!(repo("github.com/a/b/tree/main").overlaps(&repo("https://github.com/A/B")));
        assert!(repo("gitlab.com/a/b/tree/main").overlaps(&repo("gitlab.com/a/b/blob/main/x")));
        assert_eq!(repo("gitlab.com/a/b/c/raw/main/x").key(), "gitlab.com/a/b/c");
    }
}
