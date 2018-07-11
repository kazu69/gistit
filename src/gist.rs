use std::path::{Path, PathBuf};
use std::fs::DirBuilder;

use serde_json;

use reqwest;
use reqwest::Client;
use reqwest::header::Headers;

use git2;
use git2::{Repository, ObjectType, RemoteCallbacks, PushOptions, Config};

use utils;

#[derive(Deserialize, Debug)]
pub struct ApiResponse {    
    pub id: String,
    pub commits_url: String,
    pub forks_url: String,
    pub git_pull_url: String,
    pub git_push_url: String,
    pub html_url: String,
    pub public: bool,
}

#[derive(Debug)]
pub struct Gist {
    host: String,
    user: String,
    token: String,
    sshuser: String,
    workdir: String,
    files: Vec<PathBuf>,
}

impl Gist {
    pub fn api_uri(&self) -> String {
        let uri = format!("https://api.{}/gists", &self.host);
        uri
    }

    pub fn create_repo(&self, body: &serde_json::Value) -> reqwest::Result<reqwest::Response> {
        let url = self.api_uri();
        let client = Client::new();
        let header_token = ["token", &self.token].join(" ");
        let mut headers = Headers::new();
        headers.set_raw("Authorization", header_token.as_str());
        let response = client.post(&url).headers(headers).json(&body).send();

        response
    }

    pub fn add_and_commit_all(&self, repository: &Repository, gist_id: &str) -> Result<(), git2::Error> {
        match self.add_all_to_index(repository, "") {
            Ok(_oid) => {
                let pushed = self.push_reository(repository, gist_id);
                pushed
            },
            Err(err) => Err(err)
        }
    }

    fn add_all_to_index(&self, repo: &Repository, message: &str) -> Result<git2::Oid, git2::Error> {
        let mut index = repo.index()?;
        let callback = &mut |path: &Path, _matched_spec: &[u8]| -> i32 {
             let status = repo.status_file(path).unwrap();
             let ret = if status.contains(git2::Status::WT_MODIFIED) ||
                         status.contains(git2::Status::WT_NEW) ||
                         status.contains(git2::Status::WT_DELETED)  {
                    0
                } else {
                    1
                };

            ret
        };

        let add_all_callback = Some(callback as &mut git2::IndexMatchedPath);
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, add_all_callback)?;
        index.write()?;
        let oid = index.write_tree()?;
        let signature = repo.signature()?;
        let parent_commit = self.find_last_commit(&repo)?;
        let tree = repo.find_tree(oid)?;
        let result = repo.commit(Some("HEAD"),
                        &signature,
                        &signature,
                        message,
                        &tree,
                        &[&parent_commit]
                    );
        if result.is_err() {
            return Err(git2::Error::from_str("Commit faild"));
        }

        result
    }

    fn sshurl(&self, gist_id: &str) -> String {
        let url = format!("{}@gist.github.com:{}.git", self.sshuser, gist_id);
        url
    }

    fn push_reository(&self, repo: &Repository, gist_id: &str) -> Result<(), git2::Error> {
        let ssh_url = self.sshurl(gist_id);
        let git_config: PathBuf = utils::get_file_path(".git/config");
        let mut cfg = Config::open(&git_config).unwrap();
        cfg.remove("remote.origin.url")?;

        let mut remote = repo.remote("origin", &ssh_url)?;
        let mut callbacks = RemoteCallbacks::new();
        cfg.set_str("remote.origin.url", &ssh_url)?;
        callbacks.credentials(|_url, username, allowed| {
            let credentials = if allowed.contains(git2::CredentialType::SSH_KEY) {
                let user = username.unwrap();
                git2::Cred::ssh_key_from_agent(user)
            } else {
                return Err(git2::Error::from_str("No authentication available. Please make sure you have the correct access rights"))
            };

            if credentials.is_err() {
                return Err(git2::Error::from_str("Failed to authenticate. Please make sure you have the correct access rights"))
            }

            credentials
        });
        let mut opts = PushOptions::new();
        opts.remote_callbacks(callbacks);
        remote.push(&["refs/heads/master:refs/heads/master"], Some(&mut opts))
    }

    pub fn create_work_dir(&self) -> PathBuf {
        let home_dir = utils::get_home_dir();
        let mut app_dir: PathBuf = PathBuf::from(&home_dir);
        app_dir.push(&self.workdir);

        if Path::new(&app_dir).exists() {
            return  app_dir;
        } else {
            DirBuilder::new().recursive(true).create(&app_dir).unwrap();
        }
        app_dir
    }

    pub fn find_last_commit<'a>(&self, repo: &'a Repository) -> Result<git2::Commit<'a>, git2::Error> {
      let obj = repo.head()?
                    .resolve()?
                    .peel(ObjectType::Commit)?;

      obj.into_commit()
          .map_err(|_| git2::Error::from_str("Couldn't find commit"))
    }
}

#[derive(Debug, Clone)]
pub struct GistBuilder {
    host: String,
    user: String,
    token: String,
    sshuser: String,
    workdir: String,
    files: Vec<PathBuf>,
}

impl GistBuilder {
    pub fn new(username: &str, token: &str) -> GistBuilder {
        GistBuilder {
            host: "github.com".to_string(),
            user: username.to_string(),
            token: token.to_string(),
            sshuser: "git".to_string(),
            workdir: ".gistit".to_string(),
            files: vec![],
        }
    }

    pub fn with_host(&mut self, host: String) -> &mut Self {
        self.host = host;
        self
    }

    pub fn with_files(&mut self, files: Vec<PathBuf>) -> &mut Self {
        self.files = files;
        self
    }

    pub fn finalize(&self) -> Gist {
        Gist {
            host: self.host.clone(),
            user: self.user.clone(),
            token: self.token.clone(),
            sshuser: self.sshuser.clone(),
            workdir: self.workdir.clone(),
            files: self.files.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApiRequestBuilder {
    username: String,
    description: String,
    public: bool,
}

impl ApiRequestBuilder {
    pub fn new(username: String) -> ApiRequestBuilder {
        ApiRequestBuilder {
            username: username,
            description: "".to_string(),
            public: false,
        }
    }

    pub fn with_description(&mut self, description: &str) ->  &mut Self {
        self.description = description.to_string();
        self
    }

    pub fn with_public(&mut self, public: bool) ->  &mut Self {
        self.public = public;
        self
    }

    pub fn get_body(&self) -> serde_json::Value {
        let json: serde_json::Value = json!({
           "user": self.username,
           "description": self.description,
           "public": self.public,
           "files": {
               "gistitfile": {
                   "content": "Hello gistit"
               }
           }
        });

        json
    }
}
