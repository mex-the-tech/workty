use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct GitRepo {
    pub root: PathBuf,
    pub common_dir: PathBuf,
}

impl GitRepo {
    pub fn discover(start_path: Option<&Path>) -> Result<Self> {
        let working_directory = start_path
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let root = git_rev_parse(&working_directory, &["--show-toplevel"])?;
        let common_dir = git_rev_parse(&working_directory, &["--git-common-dir"])?;

        let root = PathBuf::from(root.trim());
        let common_dir_str = common_dir.trim();

        let common_dir = if Path::new(common_dir_str).is_absolute() {
            PathBuf::from(common_dir_str)
        } else {
            root.join(common_dir_str)
        };

        Ok(Self {
            root: root.canonicalize().unwrap_or(root),
            common_dir: common_dir.canonicalize().unwrap_or(common_dir),
        })
    }

    pub fn run_git(&self, args: &[&str]) -> Result<String> {
        run_git_command(Some(&self.root), args)
    }

    #[allow(dead_code)]
    pub fn run_git_in(&self, worktree_path: &Path, args: &[&str]) -> Result<String> {
        run_git_command(Some(worktree_path), args)
    }

    pub fn origin_url(&self) -> Option<String> {
        self.run_git(&["remote", "get-url", "origin"])
            .ok()
            .map(|s| s.trim().to_string())
    }

    /// Attempts to detect the default branch of the repository.
    ///
    /// Strategy:
    /// 1. Check the `HEAD` file in the common git directory (works for bare repos/worktrees).
    /// 2. Fallback to checking for existence of "main".
    /// 3. Fallback to checking for existence of "master".
    pub fn default_branch(&self) -> Option<String> {
        // 1. Try to read HEAD from the common git directory
        // This usually points to the default branch if we are in a bare repo or the main worktree
        let head_path = self.common_dir.join("HEAD");
        if let Ok(contents) = std::fs::read_to_string(head_path) {
            if let Some(ref_name) = contents.strip_prefix("ref: refs/heads/") {
                return Some(ref_name.trim().to_string());
            }
        }

        // 2. Fallbacks
        const FALLBACK_BRANCHES: [&str; 2] = ["main", "master"];
        for branch in FALLBACK_BRANCHES {
            if branch_exists(self, branch) {
                return Some(branch.to_string());
            }
        }

        None
    }
}

fn git_rev_parse(working_directory: &Path, args: &[&str]) -> Result<String> {
    let mut cmd_args = vec!["rev-parse"];
    cmd_args.extend(args);
    run_git_command(Some(working_directory), &cmd_args)
}

pub fn run_git_command(working_directory: Option<&Path>, args: &[&str]) -> Result<String> {
    let mut cmd = Command::new("git");
    if let Some(directory) = working_directory {
        cmd.current_dir(directory);
    }
    cmd.args(args);

    let output = cmd.output().context("Failed to execute git command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "git {} failed: {}",
            args.first().unwrap_or(&""),
            stderr.trim()
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn is_git_installed() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn is_in_git_repo(path: &Path) -> bool {
    Command::new("git")
        .current_dir(path)
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn branch_exists(repo: &GitRepo, branch: &str) -> bool {
    repo.run_git(&["rev-parse", "--verify", &format!("refs/heads/{}", branch)])
        .is_ok()
}

pub fn is_ancestor(repo: &GitRepo, ancestor: &str, descendant: &str) -> Result<bool> {
    let result = Command::new("git")
        .current_dir(&repo.root)
        .args(["merge-base", "--is-ancestor", ancestor, descendant])
        .output()
        .context("Failed to check ancestry")?;
    Ok(result.status.success())
}
