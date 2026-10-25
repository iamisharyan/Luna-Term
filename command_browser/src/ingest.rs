//! Ingest helpers for platform-specific package indexes.
//! Each function returns a Vec<CommandEntry> ready for Index::import().

use crate::index::CommandEntry;

/// Parse a simple tab/newline-delimited manifest:
///   name\tdescription\tpackage
/// Used as a fallback universal format for bundled indexes.
pub fn parse_simple_manifest(data: &str, platform: &str) -> Vec<CommandEntry> {
    data.lines()
        .filter(|l| !l.trim_start().starts_with('#') && !l.is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(3, '\t').collect();
            if parts.len() < 2 { return None; }
            Some(CommandEntry {
                name: parts[0].trim().to_string(),
                description: parts.get(1).unwrap_or(&"").trim().to_string(),
                package: parts.get(2).unwrap_or(&parts[0]).trim().to_string(),
                platform: platform.to_string(),
            })
        })
        .collect()
}

/// Parse a minimal PowerShell cmdlet list (Get-Command output dump).
/// Format: CmdletName\tSynopsis
pub fn parse_powershell_manifest(data: &str) -> Vec<CommandEntry> {
    parse_simple_manifest(data, "windows")
}
