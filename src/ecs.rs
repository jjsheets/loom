//! A from-scratch, SQLite-backed entity-component-system engine.
//!
//! Entities live in two tables, `entities` and `graveyard`, whose union is
//! always every entity id ever created: [`Engine::spawn_entity`] and
//! [`Engine::despawn_entity`] maintain that invariant automatically,
//! recycling the smallest available `graveyard` id before minting a fresh
//! one. Components are ordinary SQLite tables keyed by an entity-id column
//! with `ON DELETE CASCADE`, described in a YAML file alongside the systems
//! (stored SQL, run as transactions) that operate on them. See [`Engine`]
//! for the full lifecycle.
//!
//! Systems are meant to be plain SQL, not host-side loops, so bulk entity
//! creation and deletion are demonstrated below as SQL a game author could
//! paste straight into a system's `sql:` list.
//!
//! ## Bulk despawn
//!
//! A plain multi-row `DELETE` already cascades correctly, since the
//! `AFTER DELETE ... FOR EACH ROW` trigger on `entities` fires once per
//! deleted row — no special bulk support is needed:
//!
//! ```
//! use loom::ecs::Engine;
//!
//! let yaml = r#"
//! components:
//!   - name: tag
//!     columns: "entity_id INTEGER NOT NULL, label TEXT NOT NULL"
//!     entity_refs: [entity_id]
//! systems:
//!   - name: seed
//!     phase: new_game
//!     sql:
//!       - "INSERT INTO entities DEFAULT VALUES"
//!       - "INSERT INTO entities DEFAULT VALUES"
//!       - "INSERT INTO entities DEFAULT VALUES"
//!       - "INSERT INTO tag (entity_id, label) SELECT id, 'x' FROM entities"
//!   - name: bulk_despawn
//!     phase: update
//!     sql:
//!       - "DELETE FROM entities WHERE id IN (SELECT id FROM entities WHERE id <= 2)"
//! "#;
//!
//! let mut engine = Engine::from_yaml_str(yaml).expect("valid definition");
//! engine.update().expect("bulk despawn system runs");
//!
//! let remaining: i64 = engine
//!     .connection()
//!     .query_row("SELECT COUNT(*) FROM entities", [], |row| row.get(0))
//!     .expect("count entities");
//! assert_eq!(remaining, 1);
//!
//! let graveyard: i64 = engine
//!     .connection()
//!     .query_row("SELECT COUNT(*) FROM graveyard", [], |row| row.get(0))
//!     .expect("count graveyard");
//! assert_eq!(graveyard, 2);
//!
//! let tags: i64 = engine
//!     .connection()
//!     .query_row("SELECT COUNT(*) FROM tag", [], |row| row.get(0))
//!     .expect("count tag rows");
//! assert_eq!(tags, 1, "cascade delete should remove the despawned entities' tag rows");
//! ```
//!
//! ## Bulk spawn, preferring graveyard recycling
//!
//! Spawning a batch of entities can recycle `graveyard` ids before minting
//! fresh ones, entirely in SQL, using a recursive CTE to generate the
//! fresh-id shortfall. This example starts with 2 ids in `graveyard` (from
//! an earlier despawn of entities 1 and 2) and bulk-spawns 5, so 2 are
//! recycled and 3 are freshly minted:
//!
//! ```
//! use loom::ecs::Engine;
//!
//! let yaml = r#"
//! components: []
//! systems:
//!   - name: seed
//!     phase: new_game
//!     sql:
//!       - "INSERT INTO entities DEFAULT VALUES"
//!       - "INSERT INTO entities DEFAULT VALUES"
//!       - "INSERT INTO entities DEFAULT VALUES"
//!       - "DELETE FROM entities WHERE id IN (1, 2)"
//!   - name: bulk_spawn
//!     phase: update
//!     sql:
//!       - "CREATE TEMP TABLE reused AS SELECT id FROM graveyard ORDER BY id LIMIT 5"
//!       - "INSERT INTO entities (id) SELECT id FROM reused"
//!       - "DELETE FROM graveyard WHERE id IN (SELECT id FROM reused)"
//!       - "WITH RECURSIVE fresh(n) AS (SELECT 1 UNION ALL SELECT n + 1 FROM fresh WHERE n < (5 - (SELECT COUNT(*) FROM reused))) INSERT INTO entities (id) SELECT NULL FROM fresh WHERE (5 - (SELECT COUNT(*) FROM reused)) > 0"
//!       - "DROP TABLE reused"
//! "#;
//!
//! let mut engine = Engine::from_yaml_str(yaml).expect("valid definition");
//! engine.update().expect("bulk spawn system runs");
//!
//! let total: i64 = engine
//!     .connection()
//!     .query_row("SELECT COUNT(*) FROM entities", [], |row| row.get(0))
//!     .expect("count entities");
//! assert_eq!(total, 6, "1 surviving original entity + 2 recycled + 3 fresh");
//!
//! let graveyard: i64 = engine
//!     .connection()
//!     .query_row("SELECT COUNT(*) FROM graveyard", [], |row| row.get(0))
//!     .expect("count graveyard");
//! assert_eq!(graveyard, 0, "both recycled ids should have left the graveyard");
//!
//! let recycled_present: i64 = engine
//!     .connection()
//!     .query_row("SELECT COUNT(*) FROM entities WHERE id IN (1, 2)", [], |row| row.get(0))
//!     .expect("count recycled ids");
//! assert_eq!(recycled_present, 2, "both graveyard ids should have been reused");
//! ```

mod definition;
mod engine;
mod error;
mod schema;

pub use engine::Engine;
pub use error::EcsError;
