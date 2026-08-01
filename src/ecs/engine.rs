//! The public entry point: loads a game definition and runs it against a
//! fresh, in-memory SQLite database.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension};

use super::definition::{GameDefinition, Phase, SystemDef};
use super::error::EcsError;
use super::schema::{BOOTSTRAP_SQL, component_table_ddl};

/// A running game: an in-memory SQLite database bootstrapped from a game
/// definition, plus the `update`-phase systems left to run on every tick.
///
/// # Examples
///
/// ```
/// use loom::ecs::Engine;
///
/// let yaml = r#"
/// components:
///   - name: position
///     columns: "entity_id INTEGER NOT NULL PRIMARY KEY, x REAL NOT NULL"
///     entity_refs: [entity_id]
/// systems:
///   - name: seed
///     phase: new_game
///     sql:
///       - "INSERT INTO entities DEFAULT VALUES"
///       - "INSERT INTO position (entity_id, x) VALUES (1, 0.0)"
///   - name: advance
///     phase: update
///     sql:
///       - "UPDATE position SET x = x + 1.0"
/// "#;
///
/// let mut engine = Engine::from_yaml_str(yaml).expect("valid definition");
///
/// // new_game systems have already run once, before any update() call.
/// let seeded: f64 = engine
///     .connection()
///     .query_row("SELECT x FROM position WHERE entity_id = 1", [], |row| row.get(0))
///     .expect("seeded row exists");
/// assert_eq!(seeded, 0.0);
///
/// engine.update().expect("advance system runs");
/// engine.update().expect("advance system runs");
///
/// let after_two_ticks: f64 = engine
///     .connection()
///     .query_row("SELECT x FROM position WHERE entity_id = 1", [], |row| row.get(0))
///     .expect("row still exists");
/// assert_eq!(after_two_ticks, 2.0);
///
/// // Despawning an entity cascades its components and recycles its id.
/// engine.despawn_entity(1).expect("despawn entity 1");
/// let recycled_id = engine.spawn_entity().expect("spawn a new entity");
/// assert_eq!(recycled_id, 1);
/// ```
pub struct Engine {
    /// The in-memory SQLite connection this engine's entities, components,
    /// and systems live in.
    conn: Connection,
    /// The systems to run on every [`Engine::update`] call, in declaration
    /// order.
    update_systems: Vec<SystemDef>,
}

impl Engine {
    /// Reads `path`, then behaves exactly like [`Engine::from_yaml_str`].
    ///
    /// # Errors
    ///
    /// Returns an error if `path` cannot be read, its contents are not a
    /// valid game definition, or any bootstrap/component/system SQL fails.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, EcsError> {
        let yaml = std::fs::read_to_string(path)?;
        Self::from_yaml_str(&yaml)
    }

    /// Parses `yaml` as a game definition, bootstraps a fresh in-memory
    /// database (`entities`, `graveyard`, and every component table), runs
    /// every `new_game`-phase system once, and retains the `update`-phase
    /// systems for later [`Engine::update`] calls.
    ///
    /// # Errors
    ///
    /// Returns an error if `yaml` is not a valid game definition, or any
    /// bootstrap/component/system SQL fails. A failing `new_game` system
    /// leaves no `Engine` behind: construction fails outright rather than
    /// returning a half-initialized instance.
    pub fn from_yaml_str(yaml: &str) -> Result<Self, EcsError> {
        let definition: GameDefinition = serde_yaml_ng::from_str(yaml)?;

        let mut conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        conn.execute_batch(BOOTSTRAP_SQL)?;

        for component in &definition.components {
            conn.execute_batch(&component_table_ddl(component))?;
        }

        let mut update_systems = Vec::new();
        for system in definition.systems {
            match system.phase {
                Phase::NewGame => Self::run_system(&mut conn, &system)?,
                Phase::Update => update_systems.push(system),
            }
        }

        Ok(Self {
            conn,
            update_systems,
        })
    }

    /// Runs every `update`-phase system once, in declaration order, each as
    /// its own transaction.
    ///
    /// # Errors
    ///
    /// Returns an error if any system's SQL fails. That system's own
    /// transaction rolls back, but systems that already ran earlier in this
    /// call are not undone.
    pub fn update(&mut self) -> Result<(), EcsError> {
        for system in &self.update_systems {
            Self::run_system(&mut self.conn, system)?;
        }
        Ok(())
    }

    /// Creates a new entity, recycling the smallest id in `graveyard` if one
    /// exists (leaving it with no leftover component rows, since those were
    /// already cascade-deleted when it was despawned), otherwise minting a
    /// fresh, never-before-used id.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying SQL fails.
    pub fn spawn_entity(&mut self) -> Result<i64, EcsError> {
        let tx = self.conn.transaction()?;

        let recycled: Option<i64> = tx
            .query_row("SELECT id FROM graveyard ORDER BY id LIMIT 1", [], |row| {
                row.get(0)
            })
            .optional()?;

        let id = match recycled {
            Some(id) => {
                tx.execute("DELETE FROM graveyard WHERE id = ?1", [id])?;
                tx.execute("INSERT INTO entities (id) VALUES (?1)", [id])?;
                id
            }
            None => tx.query_row(
                "INSERT INTO entities DEFAULT VALUES RETURNING id",
                [],
                |row| row.get(0),
            )?,
        };

        tx.commit()?;
        Ok(id)
    }

    /// Deletes an entity, cascading to every component row that references
    /// it and moving its id into `graveyard`.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying SQL fails.
    pub fn despawn_entity(&mut self, id: i64) -> Result<(), EcsError> {
        self.conn
            .execute("DELETE FROM entities WHERE id = ?1", [id])?;
        Ok(())
    }

    /// The underlying SQLite connection, for inspecting state directly or
    /// running ad hoc queries (primarily useful in tests).
    #[must_use]
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Runs `system`'s SQL statements together as one transaction.
    ///
    /// A SQL failure is reported as [`EcsError::System`] carrying the
    /// system's name, so a broken system in a large YAML file is easy to
    /// locate.
    fn run_system(conn: &mut Connection, system: &SystemDef) -> Result<(), EcsError> {
        let tx = conn.transaction()?;
        tx.execute_batch(&system.sql.join(";\n"))
            .map_err(|source| EcsError::System {
                name: system.name.clone(),
                source,
            })?;
        tx.commit()?;
        Ok(())
    }
}
