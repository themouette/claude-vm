use crate::error::{ClaudeVmError, Result};
use crate::update_check;
use crate::version;
use self_update::cargo_crate_version;

pub fn execute(check_only: bool, target_version: Option<String>, skip_confirm: bool) -> Result<()> {
    if check_only {
        return check_and_display();
    }

    perform_update(target_version, skip_confirm)
}

fn check_and_display() -> Result<()> {
    let current = version::VERSION;
    println!("Current version: {}", current);
    println!("\nChecking for updates...");

    match get_latest_version()? {
        Some(latest) if latest != current => {
            println!("New version available: {}", latest);
            println!(
                "\nChangelog: https://github.com/{}/{}/releases/tag/v{}",
                version::REPO_OWNER,
                version::REPO_NAME,
                latest
            );
            println!("\nRun 'claude-vm update' to upgrade");
        }
        Some(_) => println!("You're already running the latest version"),
        None => println!("Unable to check for updates"),
    }

    Ok(())
}

fn perform_update(target: Option<String>, skip_confirm: bool) -> Result<()> {
    let current = version::VERSION;

    println!("Current version: {}", current);

    // Determine target version: fetch latest if not specified or if "latest" is specified
    let target_version = match target {
        Some(v) if v == "latest" || v == "v" => {
            // Explicit "latest" request
            None
        }
        Some(v) => {
            // Specific version requested - strip 'v' prefix if present
            Some(v.trim_start_matches('v').to_string())
        }
        None => {
            // No version specified - fetch latest
            None
        }
    };

    // Fetch latest version if needed
    let target_version = if target_version.is_none() {
        match get_latest_version()? {
            Some(latest) => {
                if latest == current {
                    println!("You're already running the latest version");
                    return Ok(());
                }
                println!("New version available: {}", latest);
                Some(latest)
            }
            None => {
                return Err(ClaudeVmError::UpdateError(
                    "Unable to fetch latest version".to_string(),
                ));
            }
        }
    } else {
        target_version
    };

    println!("\nDownloading update...");

    let mut update_builder = self_update::backends::github::Update::configure();
    update_builder
        .repo_owner(version::REPO_OWNER)
        .repo_name(version::REPO_NAME)
        .bin_name(version::binary_name())
        .target(&version::current_platform()?)
        .current_version(cargo_crate_version!())
        .show_download_progress(true)
        .no_confirm(skip_confirm);

    // Always set target version to ensure we update to the specific version
    if let Some(version) = target_version {
        update_builder.target_version_tag(&format!("v{}", version));
    }

    let status = match update_builder.build()?.update() {
        Ok(status) => status,
        Err(e) => {
            // Check if it's a permission error
            let err_string = e.to_string();
            if err_string.contains("Permission denied") || err_string.contains("EACCES") {
                return Err(ClaudeVmError::PermissionDenied(
                    "Cannot replace binary. Try running with sudo: sudo claude-vm update"
                        .to_string(),
                ));
            }
            return Err(ClaudeVmError::from(e));
        }
    };

    println!("\nSuccessfully updated to version {}", status.version());

    // Clear the version check cache so next check will be fresh
    update_check::clear_cache();

    Ok(())
}

pub fn get_latest_version() -> Result<Option<String>> {
    match self_update::backends::github::ReleaseList::configure()
        .repo_owner(version::REPO_OWNER)
        .repo_name(version::REPO_NAME)
        .build()
    {
        Ok(releases) => match releases.fetch() {
            Ok(releases) => {
                if let Some(release) = releases.first() {
                    // Remove 'v' prefix if present
                    let version = release.version.trim_start_matches('v').to_string();
                    Ok(Some(version))
                } else {
                    Ok(None)
                }
            }
            Err(_) => Ok(None),
        },
        Err(_) => Ok(None),
    }
}
