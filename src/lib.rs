use std::fs;
use std::path::{Path, PathBuf};
use std::process;

mod cmd;
mod config;
mod output;
mod repo;

use config::{Config, Tree};
use output::*;

use repo::{clone_repo, detect_remote_type, init_repo, open_repo, Remote, Repo};

fn path_as_string(path: &Path) -> String {
    path.to_path_buf().into_os_string().into_string().unwrap()
}

fn env_home() -> PathBuf {
    match std::env::var("HOME") {
        Ok(path) => Path::new(&path).to_path_buf(),
        Err(e) => {
            print_error(&format!("Unable to read HOME: {}", e));
            process::exit(1);
        }
    }
}

fn expand_path(path: &Path) -> PathBuf {
    fn home_dir() -> Option<PathBuf> {
        Some(env_home())
    }

    let expanded_path = match shellexpand::full_with_context(
        &path_as_string(path),
        home_dir,
        |name| -> Result<Option<String>, &'static str> {
            match name {
                "HOME" => Ok(Some(path_as_string(home_dir().unwrap().as_path()))),
                _ => Ok(None),
            }
        },
    ) {
        Ok(std::borrow::Cow::Borrowed(path)) => path.to_owned(),
        Ok(std::borrow::Cow::Owned(path)) => path,
        Err(e) => {
            print_error(&format!("Unable to expand root: {}", e));
            process::exit(1);
        }
    };

    Path::new(&expanded_path).to_path_buf()
}

fn sync_trees(config: Config) {
    for tree in config.trees {
        let repos = tree.repos.unwrap_or_default();

        let root_path = expand_path(Path::new(&tree.root));

        for repo in &repos {
            let repo_path = root_path.join(&repo.name);

            let mut repo_handle = None;

            if repo_path.exists() {
                repo_handle = Some(open_repo(&repo_path).unwrap_or_else(|error| {
                    print_repo_error(&repo.name, &format!("Opening repository failed: {}", error));
                    process::exit(1);
                }));
            } else {
                match &repo.remotes {
                    None => {
                        print_repo_action(
                            &repo.name,
                            "Repository does not have remotes configured, initializing new",
                        );
                        repo_handle = match init_repo(&repo_path) {
                            Ok(r) => {
                                print_repo_success(&repo.name, "Repository created");
                                Some(r)
                            }
                            Err(e) => {
                                print_repo_error(
                                    &repo.name,
                                    &format!("Repository failed during init: {}", e),
                                );
                                None
                            }
                        }
                    }
                    Some(r) => {
                        let first = match r.first() {
                            Some(e) => e,
                            None => {
                                panic!("Repos is an empty array. This is a bug");
                            }
                        };

                        match clone_repo(first, &repo_path) {
                            Ok(_) => {
                                print_repo_success(&repo.name, "Repository successfully cloned");
                            }
                            Err(e) => {
                                print_repo_error(
                                    &repo.name,
                                    &format!("Repository failed during clone: {}", e),
                                );
                                continue;
                            }
                        };
                    }
                }
            }
            if let Some(remotes) = &repo.remotes {
                let repo_handle = repo_handle
                    .unwrap_or_else(|| open_repo(&repo_path).unwrap_or_else(|_| process::exit(1)));

                let current_remotes: Vec<String> = match repo_handle.remotes() {
                    Ok(r) => r,
                    Err(e) => {
                        print_repo_error(
                            &repo.name,
                            &format!("Repository failed during getting the remotes: {}", e),
                        );
                        continue;
                    }
                }
                .iter()
                .flatten()
                .map(|r| r.to_owned())
                .collect();

                for remote in remotes {
                    if !current_remotes.iter().any(|r| *r == remote.name) {
                        print_repo_action(
                            &repo.name,
                            &format!(
                                "Setting up new remote \"{}\" to \"{}\"",
                                &remote.name, &remote.url
                            ),
                        );
                        if let Err(e) = repo_handle.remote(&remote.name, &remote.url) {
                            print_repo_error(
                                &repo.name,
                                &format!("Repository failed during setting the remotes: {}", e),
                            );
                            continue;
                        }
                    } else {
                        let current_remote = repo_handle.find_remote(&remote.name).unwrap();
                        let current_url = match current_remote.url() {
                            Some(url) => url,
                            None => {
                                print_repo_error(&repo.name, &format!("Repository failed during getting of the remote URL for remote \"{}\". This is most likely caused by a non-utf8 remote name", remote.name));
                                continue;
                            }
                        };
                        if remote.url != current_url {
                            print_repo_action(
                                &repo.name,
                                &format!("Updating remote {} to \"{}\"", &remote.name, &remote.url),
                            );
                            if let Err(e) = repo_handle.remote_set_url(&remote.name, &remote.url) {
                                print_repo_error(&repo.name, &format!("Repository failed during setting of the remote URL for remote \"{}\": {}", &remote.name, e));
                                continue;
                            };
                        }
                    }
                }

                for current_remote in &current_remotes {
                    if !remotes.iter().any(|r| &r.name == current_remote) {
                        print_repo_action(
                            &repo.name,
                            &format!("Deleting remote \"{}\"", &current_remote,),
                        );
                        if let Err(e) = repo_handle.remote_delete(current_remote) {
                            print_repo_error(
                                &repo.name,
                                &format!(
                                    "Repository failed during deleting remote \"{}\": {}",
                                    &current_remote, e
                                ),
                            );
                            continue;
                        }
                    }
                }
            }

            print_repo_success(&repo.name, "OK");
        }

        let current_repos = find_repos_without_details(&root_path).unwrap();
        for repo in current_repos {
            let name = path_as_string(repo.strip_prefix(&root_path).unwrap());
            if !repos.iter().any(|r| r.name == name) {
                print_warning(&format!("Found unmanaged repository: {}", name));
            }
        }
    }
}

fn find_repos_without_details(path: &Path) -> Option<Vec<PathBuf>> {
    let mut repos: Vec<PathBuf> = Vec::new();

    let git_dir = path.join(".git");
    if git_dir.exists() {
        repos.push(path.to_path_buf());
    } else {
        match fs::read_dir(path) {
            Ok(contents) => {
                for content in contents {
                    match content {
                        Ok(entry) => {
                            let path = entry.path();
                            if path.is_symlink() {
                                continue;
                            }
                            if path.is_dir() {
                                if let Some(mut r) = find_repos_without_details(&path) {
                                    repos.append(&mut r);
                                };
                            }
                        }
                        Err(e) => {
                            print_error(&format!("Error accessing directory: {}", e));
                            continue;
                        }
                    };
                }
            }
            Err(e) => {
                print_error(&format!("Failed to open \"{}\": {}", &path.display(), &e));
                return None;
            }
        };
    }

    Some(repos)
}

fn find_repos(root: &Path) -> Option<Vec<Repo>> {
    let mut repos: Vec<Repo> = Vec::new();

    for path in find_repos_without_details(root).unwrap() {
        let repo = match open_repo(&path) {
            Ok(r) => r,
            Err(e) => {
                print_error(&format!("Error opening repo {}: {}", path.display(), e));
                return None;
            }
        };

        let remotes = match repo.remotes() {
            Ok(remotes) => {
                let mut results: Vec<Remote> = Vec::new();
                for remote in remotes.iter() {
                    match remote {
                        Some(remote_name) => {
                            match repo.find_remote(remote_name) {
                                Ok(remote) => {
                                    let name = match remote.name() {
                                        Some(name) => name.to_string(),
                                        None => {
                                            print_repo_error(&path_as_string(&path), &format!("Falied getting name of remote \"{}\". This is most likely caused by a non-utf8 remote name", remote_name));
                                            process::exit(1);
                                        }
                                    };
                                    let url = match remote.url() {
                                        Some(url) => url.to_string(),
                                        None => {
                                            print_repo_error(&path_as_string(&path), &format!("Falied getting URL of remote \"{}\". This is most likely caused by a non-utf8 URL", name));
                                            process::exit(1);
                                        }
                                    };
                                    let remote_type = match detect_remote_type(&url) {
                                        Some(t) => t,
                                        None => {
                                            print_repo_error(
                                                &path_as_string(&path),
                                                &format!(
                                                    "Could not detect remote type of \"{}\"",
                                                    &url
                                                ),
                                            );
                                            process::exit(1);
                                        }
                                    };

                                    results.push(Remote {
                                        name,
                                        url,
                                        remote_type,
                                    });
                                }
                                Err(e) => {
                                    print_repo_error(
                                        &path_as_string(&path),
                                        &format!("Error getting remote {}: {}", remote_name, e),
                                    );
                                    process::exit(1);
                                }
                            };
                        }
                        None => {
                            print_repo_error(&path_as_string(&path), "Error getting remote. This is most likely caused by a non-utf8 remote name");
                            process::exit(1);
                        }
                    };
                }
                Some(results)
            }
            Err(e) => {
                print_repo_error(
                    &path_as_string(&path),
                    &format!("Error getting remotes: {}", e),
                );
                process::exit(1);
            }
        };

        repos.push(Repo {
            name: match path == root {
                true => match &root.parent() {
                    Some(parent) => path_as_string(path.strip_prefix(parent).unwrap()),
                    None => {
                        print_error("Getting name of the search root failed. Do you have a git repository in \"/\"?");
                        process::exit(1);
                    },
                }
                false => path_as_string(path.strip_prefix(&root).unwrap()),
            },
            remotes,
        });
    }
    Some(repos)
}

fn find_in_tree(path: &Path) -> Option<Tree> {
    let repos: Vec<Repo> = match find_repos(path) {
        Some(vec) => vec,
        None => Vec::new(),
    };

    let mut root = path.to_path_buf();
    let home = env_home();
    if root.starts_with(&home) {
        // The tilde is not handled differently, it's just a normal path component for `Path`.
        // Therefore we can treat it like that during **output**.
        root = Path::new("~").join(root.strip_prefix(&home).unwrap());
    }

    Some(Tree {
        root: root.into_os_string().into_string().unwrap(),
        repos: Some(repos),
    })
}

pub fn run() {
    let opts = cmd::parse();

    match opts.subcmd {
        cmd::SubCommand::Sync(sync) => {
            let config = match config::read_config(&sync.config) {
                Ok(c) => c,
                Err(e) => {
                    print_error(&e);
                    process::exit(1);
                }
            };
            sync_trees(config);
        }
        cmd::SubCommand::Find(find) => {
            let path = Path::new(&find.path);
            if !path.exists() {
                print_error(&format!("Path \"{}\" does not exist", path.display()));
                process::exit(1);
            }
            let path = &path.canonicalize().unwrap();
            if !path.is_dir() {
                print_error(&format!("Path \"{}\" is not a directory", path.display()));
                process::exit(1);
            }

            let config = Config {
                trees: vec![find_in_tree(path).unwrap()],
            };

            let toml = toml::to_string(&config).unwrap();

            print!("{}", toml);
        }
    }
}
