pub mod migrations;

use crate::error::AppResult;
use rusqlite::Connection;
use std::path::Path;

fn configure(conn: &Connection) -> AppResult<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    Ok(())
}

pub fn open(path: &Path) -> AppResult<Connection> {
    let mut conn = Connection::open(path)?;
    configure(&conn)?;
    migrations::migrate(&mut conn)?;
    Ok(conn)
}

#[cfg(test)]
pub fn open_memory() -> AppResult<Connection> {
    let mut conn = Connection::open_in_memory()?;
    configure(&conn)?;
    migrations::migrate(&mut conn)?;
    Ok(conn)
}
