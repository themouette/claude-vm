use crate::error::Result;
use crate::version;

pub fn execute(check_updates: bool) -> Result<()> {
    let current = version::VERSION;
    println!("{} {}", version::PKG_NAME, current);

    if check_updates {
        println!("\nChecking for updates...");
        match check_latest_version()? {
            Some(latest) if latest != current => {
                println!("New version available: {}", latest);
                println!("Run 'claude-vm update' to upgrade");
            }
            Some(_) => println!("You're on the latest version"),
            None => println!("Unable to check for updates"),
        }
    }

    Ok(())
}

fn check_latest_version() -> Result<Option<String>> {
    // Use GitHub API to fetch latest release version
    let _url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        version::REPO_OWNER,
        version::REPO_NAME
    );

    // Use self_update's backend to get the latest version
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
