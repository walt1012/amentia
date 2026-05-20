use anyhow::{Context, Result};
use pith_memory::MemoryNote;
use rusqlite::params;

use crate::RuntimeStore;

impl RuntimeStore {
  pub fn load_memory_notes(&self, limit: usize) -> Result<Vec<MemoryNote>> {
    let connection = self.open_connection()?;
    let mut statement = connection.prepare(
      "SELECT id, title, body, scope, source, created_at, tags_json
       FROM memory_notes
       ORDER BY created_at DESC, id ASC
       LIMIT ?1",
    )?;

    let rows = statement.query_map([limit as i64], |row| {
      let tags_json: String = row.get(6)?;
      let tags = serde_json::from_str::<Vec<String>>(&tags_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
          tags_json.len(),
          rusqlite::types::Type::Text,
          Box::new(error),
        )
      })?;

      Ok(MemoryNote {
        id: row.get(0)?,
        title: row.get(1)?,
        body: row.get(2)?,
        scope: row.get(3)?,
        source: row.get(4)?,
        created_at: row.get(5)?,
        tags,
      })
    })?;

    rows
      .collect::<std::result::Result<Vec<_>, _>>()
      .context("failed to load memory notes")
  }

  pub fn save_memory_note(&self, note: &MemoryNote) -> Result<()> {
    let connection = self.open_connection()?;
    let tags_json =
      serde_json::to_string(&note.tags).context("failed to serialize memory note tags")?;
    connection.execute(
      "INSERT INTO memory_notes (
        id,
        title,
        body,
        scope,
        source,
        created_at,
        tags_json
      ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
      ON CONFLICT(id) DO UPDATE SET
        title = excluded.title,
        body = excluded.body,
        scope = excluded.scope,
        source = excluded.source,
        created_at = excluded.created_at,
        tags_json = excluded.tags_json",
      params![
        &note.id,
        &note.title,
        &note.body,
        &note.scope,
        &note.source,
        note.created_at,
        tags_json,
      ],
    )?;

    Ok(())
  }

  pub fn next_memory_sequence(&self) -> Result<usize> {
    let connection = self.open_connection()?;
    let mut statement = connection.prepare("SELECT id FROM memory_notes ORDER BY id ASC")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;

    let max_sequence = rows
      .collect::<std::result::Result<Vec<_>, _>>()?
      .into_iter()
      .filter_map(|id| {
        id.strip_prefix("memory-")
          .and_then(|suffix| suffix.parse::<usize>().ok())
      })
      .max()
      .unwrap_or(0);

    Ok(max_sequence + 1)
  }
}
