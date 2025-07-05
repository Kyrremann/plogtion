use git2::{Cred, PushOptions, RemoteCallbacks, Repository, Signature, build::RepoBuilder};
use log::info;
use std::path::Path;

const REPO_PATH: &str = "./plog";

pub async fn clone_repository(token: &str) -> Result<Repository, String> {
    // Clean up the temporary directory if it exists
    if Path::new(REPO_PATH).exists() {
        std::fs::remove_dir_all(REPO_PATH)
            .map_err(|e| format!("Failed to clean up temporary directory: {}", e))?;
    }

    let url = "https://github.com/Kyrremann/plog.git";
    let mut fetch_options = git2::FetchOptions::new();
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_, _, _| Cred::userpass_plaintext(token, ""));
    fetch_options.remote_callbacks(callbacks);

    let repo = RepoBuilder::new()
        .fetch_options(fetch_options)
        .clone(url, Path::new(REPO_PATH))
        .map_err(|e| format!("Failed to clone repository with RepoBuilder: {}", e))?;

    info!("Repository cloned successfully");
    Ok(repo)
}

pub async fn commit_and_push(
    repo: Repository,
    token: &str,
    file_path: &str,
    message: &str,
) -> Result<String, String> {
    // Ensure file to commit exists
    let file_to_commit = Path::new(REPO_PATH).join(file_path);
    if !file_to_commit.exists() {
        return Err(format!("File to commit does not exist: {}", file_path));
    }

    // Adding the change to the index
    let mut index = repo
        .index()
        .map_err(|e| format!("Failed to get repository index: {}", e))?;
    index
        .add_path(Path::new(file_path))
        .map_err(|e| format!("Failed to add file to index: {}", e))?;
    index
        .write()
        .map_err(|e| format!("Failed to write index: {}", e))?;

    // Creating a commit
    let oid = index
        .write_tree()
        .map_err(|e| format!("Failed to write tree: {}", e))?;
    let author = Signature::now("Plog Bot", "plog-scaleway[bot]@users.noreply.github.com")
        .map_err(|e| format!("Failed to create signature: {}", e))?;
    let tree = repo
        .find_tree(oid)
        .map_err(|e| format!("Failed to find tree: {}", e))?;

    repo.commit(
        Some("HEAD"),
        &author,
        &author,
        message,
        &tree,
        &[&repo
            .head()
            .map_err(|e| format!("Failed to get repository head: {}", e))?
            .peel_to_commit()
            .map_err(|e| format!("Failed to peel to commit: {}", e))?],
    )
    .map_err(|e| format!("Failed to create commit: {}", e))?;

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_, _, _| Cred::userpass_plaintext(token, ""));
    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(callbacks);

    let mut remote = repo
        .find_remote("origin")
        .map_err(|e| format!("Failed to find remote: {}", e))?;

    remote
        .push(
            &["refs/heads/main:refs/heads/main"],
            Some(&mut push_options),
        )
        .map_err(|e| format!("Failed to push changes: {}", e))?;

    info!("Changes pushed successfully");
    Ok("Changes pushed successfully".to_string())
}
