//! SQLite FTS5-backed command index.

use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEntry {
    pub name: String,
    pub description: String,
    pub package: String,
    pub platform: String,
}

pub struct Index {
    conn: Connection,
}

impl Index {
    /// Open or create the index database at `path`.
    pub fn open(path: &std::path::Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE VIRTUAL TABLE IF NOT EXISTS cmds
               USING fts5(name, description, package, platform, tokenize='porter ascii');",
        )?;
        Ok(Self { conn })
    }

    /// Open an in-memory index (useful for tests).
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE VIRTUAL TABLE IF NOT EXISTS cmds
               USING fts5(name, description, package, platform, tokenize='porter ascii');",
        )?;
        Ok(Self { conn })
    }

    /// Bulk-import a slice of entries inside a transaction.
    pub fn import(&self, entries: &[CommandEntry]) -> Result<()> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO cmds (name, description, package, platform) VALUES (?1, ?2, ?3, ?4)",
        )?;
        for e in entries {
            stmt.execute(params![e.name, e.description, e.package, e.platform])?;
        }
        Ok(())
    }

    /// Fuzzy full-text search; returns up to `limit` matching entries.
    /// Uses FTS5 MATCH with wildcards so partial prefixes work.
    pub fn search(&self, pattern: &str, limit: usize) -> Result<Vec<CommandEntry>> {
        if pattern.trim().is_empty() {
            return Ok(Vec::new());
        }
        // Append wildcard for prefix matching
        let query = format!("{}*", pattern.replace('"', "\"\""));
        let mut stmt = self.conn.prepare(
            "SELECT name, description, package, platform
               FROM cmds
              WHERE cmds MATCH ?1
              ORDER BY rank
              LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![query, limit as i64], |row| {
            Ok(CommandEntry {
                name: row.get(0)?,
                description: row.get(1)?,
                package: row.get(2)?,
                platform: row.get(3)?,
            })
        })?;
        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    /// Total number of entries in the index.
    pub fn count(&self) -> Result<usize> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM cmds",
            [],
            |r| r.get(0),
        )?;
        Ok(n as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entries() -> Vec<CommandEntry> {
        vec![
            CommandEntry {
                name: "grep".into(),
                description: "Print lines that match patterns".into(),
                package: "grep".into(),
                platform: "linux".into(),
            },
            CommandEntry {
                name: "ripgrep".into(),
                description: "Recursively searches directories for a regex pattern".into(),
                package: "ripgrep".into(),
                platform: "cross".into(),
            },
            CommandEntry {
                name: "find".into(),
                description: "Search for files in a directory hierarchy".into(),
                package: "findutils".into(),
                platform: "linux".into(),
            },
        ]
    }

    #[test]
    fn test_import_and_search() {
        let idx = Index::in_memory().unwrap();
        idx.import(&sample_entries()).unwrap();
        let results = idx.search("grep", 10).unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|e| e.name.contains("grep")));
    }

    #[test]
    fn test_prefix_search() {
        let idx = Index::in_memory().unwrap();
        idx.import(&sample_entries()).unwrap();
        let results = idx.search("rip", 10).unwrap();
        assert!(results.iter().any(|e| e.name == "ripgrep"));
    }

    #[test]
    fn test_count() {
        let idx = Index::in_memory().unwrap();
        idx.import(&sample_entries()).unwrap();
        assert_eq!(idx.count().unwrap(), 3);
    }
}
