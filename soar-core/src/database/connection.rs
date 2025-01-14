use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use rusqlite::Connection;

use crate::error::SoarError;

use super::{
    models::RemotePackageMetadata, repository::PackageRepository, statements::DbStatements,
};

type Result<T> = std::result::Result<T, SoarError>;

pub struct Database {
    pub conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let conn = Connection::open(path)?;
        let conn = Arc::new(Mutex::new(conn));
        Ok(Database { conn })
    }

    pub fn new_multi<P: AsRef<Path>>(paths: &[P]) -> Result<Self> {
        let conn = Connection::open(&paths[0])?;
        for (idx, path) in paths.iter().enumerate().skip(1) {
            let path = path.as_ref();
            conn.execute(
                &format!("ATTACH DATABASE '{}' AS shard{}", path.display(), idx),
                [],
            )?;
        }
        let conn = Arc::new(Mutex::new(conn));
        Ok(Database { conn })
    }

    pub fn from_json_metadata(
        &self,
        metadata: RemotePackageMetadata,
        repo_name: &str,
    ) -> Result<()> {
        let mut guard = self.conn.lock().unwrap();
        let _: String = guard.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;

        let tx = guard.transaction()?;
        {
            let statements = DbStatements::new(&tx)?;
            let mut repo = PackageRepository::new(&tx, statements, repo_name);
            repo.import_packages(&metadata)?;
        }
        tx.commit()?;
        Ok(())
    }
}
