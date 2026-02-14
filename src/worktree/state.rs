use crate::error::Result;
use crate::utils::git::run_git_command;
use std::path::PathBuf;

/// Represents a git worktree entry parsed from porcelain output
#[derive(Debug, Clone, PartialEq)]
pub struct WorktreeEntry {
    pub path: PathBuf,
    pub head: String,           // HEAD commit SHA
    pub branch: Option<String>, // None if detached HEAD
    pub is_bare: bool,
    pub is_detached: bool,
    pub locked: Option<String>, // Lock reason if locked (Some("") if locked without reason)
}

/// Parse git worktree list --porcelain output into WorktreeEntry structs
fn parse_porcelain_output(output: &str) -> Vec<WorktreeEntry> {
    let mut entries = Vec::new();

    // Split on double newlines to get worktree blocks
    for block in output.split("\n\n") {
        if block.trim().is_empty() {
            continue;
        }

        let mut path = None;
        let mut head = None;
        let mut branch = None;
        let mut is_bare = false;
        let mut is_detached = false;
        let mut locked = None;

        for line in block.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some(p) = line.strip_prefix("worktree ") {
                path = Some(PathBuf::from(p));
            } else if let Some(h) = line.strip_prefix("HEAD ") {
                head = Some(h.to_string());
            } else if let Some(b) = line.strip_prefix("branch ") {
                // Strip refs/heads/ prefix to get clean branch name
                let branch_name = b.strip_prefix("refs/heads/").unwrap_or(b);
                branch = Some(branch_name.to_string());
            } else if line == "bare" {
                is_bare = true;
            } else if line == "detached" {
                is_detached = true;
            } else if let Some(reason) = line.strip_prefix("locked ") {
                locked = Some(reason.to_string());
            } else if line == "locked" {
                locked = Some("".to_string());
            }
        }

        // Only add entry if we have at least path and head
        if let (Some(path), Some(head)) = (path, head) {
            entries.push(WorktreeEntry {
                path,
                head,
                branch,
                is_bare,
                is_detached,
                locked,
            });
        }
    }

    entries
}

/// Query git worktree list and return parsed entries
pub fn list_worktrees() -> Result<Vec<WorktreeEntry>> {
    let output_str = run_git_command(&["worktree", "list", "--porcelain"], "list worktrees")?;
    Ok(parse_porcelain_output(&output_str))
}

/// Filter worktree entries to find locked ones
pub fn find_locked_worktrees(entries: &[WorktreeEntry]) -> Vec<&WorktreeEntry> {
    entries.iter().filter(|e| e.locked.is_some()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_worktree() {
        let input = "worktree /home/user/project
HEAD abc123def456
branch refs/heads/main
";
        let result = parse_porcelain_output(input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("/home/user/project"));
        assert_eq!(result[0].head, "abc123def456");
        assert_eq!(result[0].branch, Some("main".to_string()));
        assert!(!result[0].is_bare);
        assert!(!result[0].is_detached);
        assert_eq!(result[0].locked, None);
    }

    #[test]
    fn test_parse_multiple_worktrees() {
        let input = "worktree /home/user/project
HEAD abc123def456
branch refs/heads/main

worktree /home/user/project-feature
HEAD def789abc012
branch refs/heads/feature/auth
";
        let result = parse_porcelain_output(input);
        assert_eq!(result.len(), 2);

        assert_eq!(result[0].path, PathBuf::from("/home/user/project"));
        assert_eq!(result[0].branch, Some("main".to_string()));

        assert_eq!(result[1].path, PathBuf::from("/home/user/project-feature"));
        assert_eq!(result[1].branch, Some("feature/auth".to_string()));
    }

    #[test]
    fn test_parse_detached_head() {
        let input = "worktree /home/user/project
HEAD abc123def456
detached
";
        let result = parse_porcelain_output(input);
        assert_eq!(result.len(), 1);
        assert!(result[0].is_detached);
        assert_eq!(result[0].branch, None);
    }

    #[test]
    fn test_parse_bare_worktree() {
        let input = "worktree /home/user/project.git
HEAD abc123def456
bare
";
        let result = parse_porcelain_output(input);
        assert_eq!(result.len(), 1);
        assert!(result[0].is_bare);
    }

    #[test]
    fn test_parse_locked_with_reason() {
        let input = "worktree /home/user/project-feature
HEAD def789abc012
branch refs/heads/feature
locked working on something
";
        let result = parse_porcelain_output(input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].locked, Some("working on something".to_string()));
    }

    #[test]
    fn test_parse_locked_without_reason() {
        let input = "worktree /home/user/project-feature
HEAD def789abc012
branch refs/heads/feature
locked
";
        let result = parse_porcelain_output(input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].locked, Some("".to_string()));
    }

    #[test]
    fn test_parse_empty_output() {
        let result = parse_porcelain_output("");
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_branch_name_extraction() {
        let input = "worktree /home/user/project
HEAD abc123def456
branch refs/heads/feature/deep/nested
";
        let result = parse_porcelain_output(input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].branch, Some("feature/deep/nested".to_string()));
    }

    #[test]
    fn test_find_locked_empty() {
        let entries = vec![WorktreeEntry {
            path: PathBuf::from("/test"),
            head: "abc123".to_string(),
            branch: Some("main".to_string()),
            is_bare: false,
            is_detached: false,
            locked: None,
        }];
        let result = find_locked_worktrees(&entries);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_find_locked_some() {
        let entries = vec![
            WorktreeEntry {
                path: PathBuf::from("/test1"),
                head: "abc123".to_string(),
                branch: Some("main".to_string()),
                is_bare: false,
                is_detached: false,
                locked: None,
            },
            WorktreeEntry {
                path: PathBuf::from("/test2"),
                head: "def456".to_string(),
                branch: Some("feature".to_string()),
                is_bare: false,
                is_detached: false,
                locked: Some("in use".to_string()),
            },
            WorktreeEntry {
                path: PathBuf::from("/test3"),
                head: "ghi789".to_string(),
                branch: Some("hotfix".to_string()),
                is_bare: false,
                is_detached: false,
                locked: Some("".to_string()),
            },
        ];
        let result = find_locked_worktrees(&entries);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].path, PathBuf::from("/test2"));
        assert_eq!(result[1].path, PathBuf::from("/test3"));
    }
}
