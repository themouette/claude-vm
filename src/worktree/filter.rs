//! Composable filter functions for worktree entries
//!
//! This module provides simple, composable filter functions that work with
//! iterators of WorktreeEntry references. Each filter can be chained to build
//! complex filtering logic.

use crate::worktree::state::WorktreeEntry;

/// Filter worktrees to only those whose branch is merged into the base branch.
///
/// Worktrees without a branch (detached HEAD) are excluded from the results.
///
/// # Example
/// ```ignore
/// let iter = worktrees.iter();
/// let merged = filter_merged(iter, &merged_branches);
/// ```
pub fn filter_merged<'a>(
    worktrees: impl Iterator<Item = &'a WorktreeEntry> + 'a,
    merged_branches: &'a [String],
) -> impl Iterator<Item = &'a WorktreeEntry> + 'a {
    worktrees.filter(move |w| {
        w.branch
            .as_ref()
            .map(|b| merged_branches.contains(b))
            .unwrap_or(false)
    })
}

/// Filter worktrees to only those that are locked.
///
/// # Example
/// ```ignore
/// let iter = worktrees.iter();
/// let locked = filter_locked(iter);
/// ```
pub fn filter_locked<'a>(
    worktrees: impl Iterator<Item = &'a WorktreeEntry> + 'a,
) -> impl Iterator<Item = &'a WorktreeEntry> + 'a {
    worktrees.filter(|w| w.locked.is_some())
}

/// Filter worktrees to only those with detached HEAD.
///
/// # Example
/// ```ignore
/// let iter = worktrees.iter();
/// let detached = filter_detached(iter);
/// ```
pub fn filter_detached<'a>(
    worktrees: impl Iterator<Item = &'a WorktreeEntry> + 'a,
) -> impl Iterator<Item = &'a WorktreeEntry> + 'a {
    worktrees.filter(|w| w.is_detached)
}

/// Exclude locked worktrees (inverse of filter_locked).
///
/// This is useful when you want to skip locked worktrees from processing.
///
/// # Example
/// ```ignore
/// let iter = worktrees.iter();
/// let unlocked = exclude_locked(iter);
/// ```
pub fn exclude_locked<'a>(
    worktrees: impl Iterator<Item = &'a WorktreeEntry> + 'a,
) -> impl Iterator<Item = &'a WorktreeEntry> + 'a {
    worktrees.filter(|w| w.locked.is_none())
}

/// Skip the main worktree (first entry in the list).
///
/// Git worktree list always returns the main worktree first, and many operations
/// need to exclude it from processing.
///
/// # Example
/// ```ignore
/// let iter = worktrees.iter();
/// let non_main = skip_main(iter);
/// ```
pub fn skip_main<'a>(
    worktrees: impl Iterator<Item = &'a WorktreeEntry> + 'a,
) -> impl Iterator<Item = &'a WorktreeEntry> + 'a {
    worktrees.skip(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_worktree(
        path: &str,
        branch: Option<&str>,
        locked: bool,
        detached: bool,
    ) -> WorktreeEntry {
        WorktreeEntry {
            path: PathBuf::from(path),
            head: "abc123".to_string(),
            branch: branch.map(|s| s.to_string()),
            is_bare: false,
            is_detached: detached,
            locked: if locked { Some("".to_string()) } else { None },
        }
    }

    #[test]
    fn test_filter_merged_includes_merged_branches() {
        let worktrees = [
            create_test_worktree("/main", Some("main"), false, false),
            create_test_worktree("/feature-1", Some("feature-1"), false, false),
            create_test_worktree("/feature-2", Some("feature-2"), false, false),
        ];
        let merged_branches = vec!["main".to_string(), "feature-1".to_string()];

        let result: Vec<_> = filter_merged(worktrees.iter(), &merged_branches).collect();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].branch, Some("main".to_string()));
        assert_eq!(result[1].branch, Some("feature-1".to_string()));
    }

    #[test]
    fn test_filter_merged_excludes_detached() {
        let worktrees = [
            create_test_worktree("/main", Some("main"), false, false),
            create_test_worktree("/detached", None, false, true),
        ];
        let merged_branches = vec!["main".to_string()];

        let result: Vec<_> = filter_merged(worktrees.iter(), &merged_branches).collect();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].branch, Some("main".to_string()));
    }

    #[test]
    fn test_filter_merged_empty_list() {
        let worktrees = [create_test_worktree(
            "/feature-1",
            Some("feature-1"),
            false,
            false,
        )];
        let merged_branches: Vec<String> = vec![];

        let result: Vec<_> = filter_merged(worktrees.iter(), &merged_branches).collect();

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_filter_locked_includes_only_locked() {
        let worktrees = [
            create_test_worktree("/main", Some("main"), false, false),
            create_test_worktree("/locked-1", Some("feature-1"), true, false),
            create_test_worktree("/unlocked", Some("feature-2"), false, false),
            create_test_worktree("/locked-2", Some("feature-3"), true, false),
        ];

        let result: Vec<_> = filter_locked(worktrees.iter()).collect();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].path, PathBuf::from("/locked-1"));
        assert_eq!(result[1].path, PathBuf::from("/locked-2"));
    }

    #[test]
    fn test_filter_locked_empty_when_none_locked() {
        let worktrees = [
            create_test_worktree("/main", Some("main"), false, false),
            create_test_worktree("/feature", Some("feature"), false, false),
        ];

        let result: Vec<_> = filter_locked(worktrees.iter()).collect();

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_filter_detached_includes_only_detached() {
        let worktrees = [
            create_test_worktree("/main", Some("main"), false, false),
            create_test_worktree("/detached-1", None, false, true),
            create_test_worktree("/feature", Some("feature"), false, false),
            create_test_worktree("/detached-2", None, false, true),
        ];

        let result: Vec<_> = filter_detached(worktrees.iter()).collect();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].path, PathBuf::from("/detached-1"));
        assert_eq!(result[1].path, PathBuf::from("/detached-2"));
    }

    #[test]
    fn test_filter_detached_empty_when_none_detached() {
        let worktrees = [
            create_test_worktree("/main", Some("main"), false, false),
            create_test_worktree("/feature", Some("feature"), false, false),
        ];

        let result: Vec<_> = filter_detached(worktrees.iter()).collect();

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_exclude_locked_excludes_locked() {
        let worktrees = [
            create_test_worktree("/main", Some("main"), false, false),
            create_test_worktree("/locked", Some("feature-1"), true, false),
            create_test_worktree("/unlocked", Some("feature-2"), false, false),
        ];

        let result: Vec<_> = exclude_locked(worktrees.iter()).collect();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].path, PathBuf::from("/main"));
        assert_eq!(result[1].path, PathBuf::from("/unlocked"));
    }

    #[test]
    fn test_exclude_locked_all_when_none_locked() {
        let worktrees = [
            create_test_worktree("/main", Some("main"), false, false),
            create_test_worktree("/feature", Some("feature"), false, false),
        ];

        let result: Vec<_> = exclude_locked(worktrees.iter()).collect();

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_skip_main_removes_first_entry() {
        let worktrees = [
            create_test_worktree("/main", Some("main"), false, false),
            create_test_worktree("/feature-1", Some("feature-1"), false, false),
            create_test_worktree("/feature-2", Some("feature-2"), false, false),
        ];

        let result: Vec<_> = skip_main(worktrees.iter()).collect();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].path, PathBuf::from("/feature-1"));
        assert_eq!(result[1].path, PathBuf::from("/feature-2"));
    }

    #[test]
    fn test_skip_main_empty_when_only_main() {
        let worktrees = [create_test_worktree("/main", Some("main"), false, false)];

        let result: Vec<_> = skip_main(worktrees.iter()).collect();

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_chaining_skip_main_and_filter_merged() {
        let worktrees = [
            create_test_worktree("/main", Some("main"), false, false),
            create_test_worktree("/merged-1", Some("feature-1"), false, false),
            create_test_worktree("/unmerged", Some("feature-2"), false, false),
            create_test_worktree("/merged-2", Some("feature-3"), false, false),
        ];
        let merged_branches = vec![
            "main".to_string(),
            "feature-1".to_string(),
            "feature-3".to_string(),
        ];

        // Chain filters: skip main, then filter merged
        let iter = skip_main(worktrees.iter());
        let result: Vec<_> = filter_merged(iter, &merged_branches).collect();

        // Should exclude main (skipped) and feature-2 (not merged)
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].branch, Some("feature-1".to_string()));
        assert_eq!(result[1].branch, Some("feature-3".to_string()));
    }

    #[test]
    fn test_chaining_all_filters() {
        let worktrees = [
            create_test_worktree("/main", Some("main"), false, false),
            create_test_worktree("/merged-unlocked", Some("feature-1"), false, false),
            create_test_worktree("/merged-locked", Some("feature-2"), true, false),
            create_test_worktree("/unmerged", Some("feature-3"), false, false),
        ];
        let merged_branches = vec![
            "main".to_string(),
            "feature-1".to_string(),
            "feature-2".to_string(),
        ];

        // Chain: skip main -> filter merged -> exclude locked
        let iter = skip_main(worktrees.iter());
        let iter = filter_merged(iter, &merged_branches);
        let result: Vec<_> = exclude_locked(iter).collect();

        // Should only have merged-unlocked
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].branch, Some("feature-1".to_string()));
    }

    #[test]
    fn test_chaining_preserves_main_when_not_merged() {
        // This tests the bug fix: when main is not in the merged list,
        // skip_main should happen BEFORE filtering, not after
        let worktrees = [
            create_test_worktree("/main", Some("main"), false, false),
            create_test_worktree("/merged", Some("feature-1"), false, false),
            create_test_worktree("/unmerged", Some("feature-2"), false, false),
        ];
        // Main is NOT in the merged list
        let merged_branches = vec!["feature-1".to_string()];

        // Correct order: skip main FIRST, then filter
        let iter = skip_main(worktrees.iter());
        let result: Vec<_> = filter_merged(iter, &merged_branches).collect();

        // Should have only feature-1
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].branch, Some("feature-1".to_string()));
    }
}
