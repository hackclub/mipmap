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
            if let Some(end) = segments.iter().position(|segment| *segment == "-") {
                segments.truncate(end);
            }
        }
        _ => segments.truncate(2),
    }
    if segments.len() < 2 {
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
