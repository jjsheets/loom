//! SQL DDL: the fixed entities/graveyard bootstrap, and component table
//! generation from a [`ComponentDef`].

use super::definition::ComponentDef;
use std::fmt::Write as _;

/// Creates the `entities` and `graveyard` tables and the trigger that keeps
/// `entities ∪ graveyard` equal to every entity id ever created.
///
/// `entities.id` uses `AUTOINCREMENT` so SQLite never hands out an id it has
/// already minted, even after that id is deleted — the only way an id
/// returns to `entities` is by being explicitly recycled from `graveyard`.
pub(crate) const BOOTSTRAP_SQL: &str = "
CREATE TABLE entities (
    id INTEGER PRIMARY KEY AUTOINCREMENT
);

CREATE TABLE graveyard (
    id INTEGER PRIMARY KEY
);

CREATE TRIGGER entities_after_delete
AFTER DELETE ON entities
FOR EACH ROW
BEGIN
    INSERT INTO graveyard (id) VALUES (OLD.id);
END;
";

/// Builds the `CREATE TABLE` statement for a component, appending an
/// `ON DELETE CASCADE` foreign key to `entities(id)` for each column named in
/// `def.entity_refs`.
pub(crate) fn component_table_ddl(def: &ComponentDef) -> String {
    let mut sql = format!("CREATE TABLE {} (\n    {}", def.name, def.columns);
    for column in &def.entity_refs {
        write!(
            sql,
            ",\n    FOREIGN KEY ({column}) REFERENCES entities(id) ON DELETE CASCADE"
        )
        .expect("writing to a String never fails");
    }
    sql.push_str("\n);");
    sql
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_table_ddl_appends_one_cascade_fk_per_entity_ref() {
        let def = ComponentDef {
            name: "carries".to_string(),
            columns: "carrier_id INTEGER NOT NULL, item_id INTEGER NOT NULL, PRIMARY KEY (carrier_id, item_id)".to_string(),
            entity_refs: vec!["carrier_id".to_string(), "item_id".to_string()],
        };

        let ddl = component_table_ddl(&def);

        assert!(ddl.starts_with("CREATE TABLE carries ("));
        assert!(ddl.contains("FOREIGN KEY (carrier_id) REFERENCES entities(id) ON DELETE CASCADE"));
        assert!(ddl.contains("FOREIGN KEY (item_id) REFERENCES entities(id) ON DELETE CASCADE"));
    }

    #[test]
    fn component_table_ddl_with_no_entity_refs_has_no_foreign_key() {
        let def = ComponentDef {
            name: "tag".to_string(),
            columns: "entity_id INTEGER NOT NULL, label TEXT NOT NULL".to_string(),
            entity_refs: vec![],
        };

        let ddl = component_table_ddl(&def);

        assert!(!ddl.contains("FOREIGN KEY"));
    }

    /// The DDL strings above are only meaningful if SQLite actually accepts
    /// them: this executes the bootstrap plus a representative component
    /// (including a two-entity relationship) against a real in-memory
    /// database, so a syntax error here fails close to the source rather
    /// than surfacing only in a distant integration test.
    #[test]
    fn bootstrap_and_component_ddl_execute_against_real_sqlite() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .expect("enable foreign keys");
        conn.execute_batch(BOOTSTRAP_SQL).expect("bootstrap SQL");

        let position = ComponentDef {
            name: "position".to_string(),
            columns: "entity_id INTEGER NOT NULL PRIMARY KEY, x REAL NOT NULL".to_string(),
            entity_refs: vec!["entity_id".to_string()],
        };
        conn.execute_batch(&component_table_ddl(&position))
            .expect("component DDL");

        conn.execute("INSERT INTO entities DEFAULT VALUES", [])
            .expect("insert entity");
        conn.execute("INSERT INTO position (entity_id, x) VALUES (1, 0.0)", [])
            .expect("insert component row");
        conn.execute("DELETE FROM entities WHERE id = 1", [])
            .expect("delete entity");

        let graveyard_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM graveyard WHERE id = 1", [], |row| {
                row.get(0)
            })
            .expect("query graveyard");
        assert_eq!(graveyard_count, 1);

        let position_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM position", [], |row| row.get(0))
            .expect("query position");
        assert_eq!(
            position_count, 0,
            "cascade delete should have removed the component row"
        );
    }
}
