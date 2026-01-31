use crate::error::{ClaudeVmError, Result};
use crate::vm::limactl::LimaCtl;

/// Check if a template exists for the given name
pub fn exists(template_name: &str) -> Result<bool> {
    LimaCtl::vm_exists(template_name)
}

/// Verify a template exists, return error if not
pub fn verify(template_name: &str) -> Result<()> {
    if !exists(template_name)? {
        return Err(ClaudeVmError::TemplateNotFound(
            template_name.to_string(),
        ));
    }
    Ok(())
}

/// Delete a template
pub fn delete(template_name: &str) -> Result<()> {
    if exists(template_name)? {
        LimaCtl::delete(template_name, true, true)?;  // Always verbose for user-initiated deletes
    }
    Ok(())
}

/// List all claude-vm templates
pub fn list_all() -> Result<Vec<String>> {
    let vms = LimaCtl::list()?;
    let templates: Vec<String> = vms
        .into_iter()
        .filter(|vm| vm.name.starts_with("claude-tpl_"))
        .map(|vm| vm.name)
        .collect();
    Ok(templates)
}
