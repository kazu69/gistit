extern crate reqwest;
extern crate exitcode;
extern crate git2;
extern crate emojicons;

#[macro_use]
extern crate clap;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

use std::env;
use std::env::set_current_dir;
use std::io;
use std::process;
use std::path::PathBuf;
use std::fs;

use clap::{Arg, App};

use git2::build::RepoBuilder;

use emojicons::EmojiFormatter;

pub mod gist;
pub mod utils;

const AUTHOR: &str = "kazu69";
const DESCRIPTION: &str = r#"
Commit various files gists from the command line
"#;
const USAGE: &str = r#"
gistit [-f] <file_path>
"#;
const HELPMESSAGE: &str = r#"
           Commit various files to gist

           USAGE: gistit <opts>

           Options:
           -h, --help        Dispay this message
           -v, --version     Display version info
           -f, --file        Gist commit file name
           -d, --description Gist description
           -p, --public      Gist public or secret
           -n, --hostname    Gist hostname setting (Default github.com)
"#;

fn file_copy(file: &PathBuf) -> Result<u64, std::io::Error> {
    let filename = file.file_name().unwrap();
    fs::copy(file, filename)
}

fn delete_file(path: &PathBuf) -> io::Result<()> {
    fs::remove_dir_all(path)
}

fn main() { 
    let args = App::new("gitsru")
                .version(crate_version!())
                .version_short("v")
                .author(AUTHOR)
                .about(DESCRIPTION)
                .usage(USAGE)
                .help(HELPMESSAGE)
                .help_short("h")

                .arg(Arg::with_name("file")
                 .short("f")
                 .long("file")
                 .multiple(true)
                 .takes_value(true)
                 .help("Commit File Path."))

                .arg(Arg::with_name("description")
                 .short("d")
                 .long("description")
                 .takes_value(true)
                 .help("Gist description"))

                .arg(Arg::with_name("public")
                 .short("p")
                 .long("public")
                 .help("Gist public"))

                .arg(Arg::with_name("hostname")
                 .short("n")
                 .long("hostname")
                 .takes_value(true)
                 .help("Gist API Host name"))

                .get_matches();

        let username = match env::var("GITHUB_USERNSME") {
            Ok(username) => username,
            Err(_) => {
                eprintln!("{} Github username and password required.", format!("{}", EmojiFormatter(":x:")));
                process::exit(exitcode::DATAERR);
            }
        };

        let token = match env::var("GITHUB_TOKEN") {
           Ok(password) => password,
           Err(_) => {
               eprintln!("{} Github username and token required.", format!("{}", EmojiFormatter(":x:")));
               process::exit(exitcode::DATAERR);
           }
        };

        let mut gist_builder = gist::GistBuilder::new(&username, &token);
        let mut request_builder = gist::ApiRequestBuilder::new(username.to_string());

        let description = args.value_of("description").unwrap_or("");

        if description.len() > 0 {
            request_builder.with_description(description);
        }

        let hostname = args.value_of("hostname").unwrap_or("");

        if hostname.len() > 0 {
            gist_builder.with_host(hostname.to_string());
        }

        let public = args.is_present("public");
    
        let files = args.values_of("file").unwrap().collect::<Vec<_>>();
        let mut filepaths = vec![];
        if files.len() > 0 {
            for file in files.iter() {
                let filepath = utils::get_file_path(&file);
                filepaths.push(filepath);
            };
        } else {
            return
        }

        let request_body = request_builder.with_description(description)
                                    .with_public(public)
                                    .get_body();

        let gist: gist::Gist = gist_builder.finalize();
        let app_dir: PathBuf = gist.create_work_dir();
        let response = gist.create_repo(&request_body)
                            .map_err(|err| eprintln!("{} Error {}", format!("{}", EmojiFormatter(":x:")), err));

        if response.is_err() {
            process::exit(exitcode::DATAERR);
        }

        let json: gist::ApiResponse = response.unwrap().json().unwrap();
        let working_dir = set_current_dir(&app_dir).map_err(|err| eprintln!("{} Error {}", format!("{}", EmojiFormatter(":x:")), err));

        if working_dir.is_err() {
            process::exit(exitcode::DATAERR);
        }

        let path: PathBuf = utils::get_file_path(&json.id);
        let repository = RepoBuilder::new()
                                    .clone(&json.git_pull_url, &path).map_err(|err| eprintln!("{} Error {}", format!("{}", EmojiFormatter(":x:")), err));
        if repository.is_err() {
            process::exit(exitcode::DATAERR);
        }

        let repository = repository.unwrap();
        let working_dir = set_current_dir(path)
                            .map_err(|err| eprintln!("{} Error {}", format!("{}", EmojiFormatter(":x:")), err));
        if working_dir.is_err() {
            process::exit(exitcode::DATAERR);
        }    

        for file in filepaths {
            let copyed = file_copy(&file);
            if copyed.is_err() {
                let err = &copyed.unwrap();
                eprintln!("{} Error change dir faild {:?}", format!("{}", EmojiFormatter(":x:")), &err);
            }
        }

        if fs::remove_file("gistitfile").is_err() {
            eprintln!("{} Error remove file", format!("{}", EmojiFormatter(":x:")));
        }

        let commited = gist.add_and_commit_all(&repository, &json.id).map_err(|e| eprintln!("Error {}" ,e));
        if commited.is_err() {
            process::exit(exitcode::DATAERR);
        }

        if set_current_dir(&app_dir).is_err() {
            eprintln!("{} Error {}!", format!("{}", EmojiFormatter(":x:")), "change dir");
            process::exit(exitcode::DATAERR);
        }

        let clone_dir = utils::get_file_path(&json.id);

        if delete_file(&clone_dir).is_err() {
            eprintln!("{} Error {}!", format!("{}", EmojiFormatter(":x:")), "delete dir");
            process::exit(exitcode::DATAERR);
        }

        println!("{} Create Gist. Open URL {}", format!("{}", EmojiFormatter(":rocket:")), json.html_url);
}
