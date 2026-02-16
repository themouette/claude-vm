use crate::config::ResourceCheckMode;
use crate::error::{ClaudeVmError, Result};
use std::io::{self, Write};

/// Handle resource warning based on the configured mode
pub fn handle_resource_warning(
    mode: &ResourceCheckMode,
    warning_message: &str,
    force: bool,
) -> Result<()> {
    // If --force-resources flag is set, always proceed
    if force {
        return Ok(());
    }

    match mode {
        ResourceCheckMode::Warn => {
            // Just print warning to stderr and continue
            eprintln!("{}", warning_message);
            Ok(())
        }
        ResourceCheckMode::Ask => {
            // Print warning and prompt user
            eprintln!("{}", warning_message);
            print!("Proceed anyway? [y/N]: ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim().to_lowercase();

            if input == "y" || input == "yes" {
                Ok(())
            } else {
                Err(ClaudeVmError::ResourceOverprovisioned {
                    message: "User declined to proceed with overprovisioned resources".to_string(),
                    details: warning_message.to_string(),
                })
            }
        }
        ResourceCheckMode::Prevent => {
            // Always error
            Err(ClaudeVmError::ResourceOverprovisioned {
                message: "Resource thresholds exceeded".to_string(),
                details: warning_message.to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_force_flag_bypasses_all_modes() {
        let warning = "Test warning";

        // Should succeed with force flag regardless of mode
        assert!(handle_resource_warning(&ResourceCheckMode::Ask, warning, true).is_ok());
        assert!(handle_resource_warning(&ResourceCheckMode::Warn, warning, true).is_ok());
        assert!(handle_resource_warning(&ResourceCheckMode::Prevent, warning, true).is_ok());
    }

    #[test]
    fn test_warn_mode_always_succeeds() {
        let warning = "Test warning";
        let result = handle_resource_warning(&ResourceCheckMode::Warn, warning, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_prevent_mode_always_fails() {
        let warning = "Test warning";
        let result = handle_resource_warning(&ResourceCheckMode::Prevent, warning, false);
        assert!(result.is_err());
        if let Err(ClaudeVmError::ResourceOverprovisioned { message, details }) = result {
            assert!(message.contains("Resource thresholds exceeded"));
            assert_eq!(details, warning);
        }
    }
}
