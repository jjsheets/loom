//! The YAML shape a game author writes: component tables and systems.

use serde::Deserialize;

/// The full contents of a game engine YAML file: every component table and
/// every system the engine should create and run.
#[derive(Debug, Deserialize)]
pub(crate) struct GameDefinition {
    /// The component tables to create, in declaration order.
    pub(crate) components: Vec<ComponentDef>,
    /// The systems to run, in declaration order.
    pub(crate) systems: Vec<SystemDef>,
}

/// A single component table, described as raw SQL fragments.
#[derive(Debug, Deserialize)]
pub(crate) struct ComponentDef {
    /// The table name.
    pub(crate) name: String,
    /// Literal column and constraint SQL text, spliced into a `CREATE TABLE`
    /// statement as-is.
    pub(crate) columns: String,
    /// Names of the columns in `columns` that reference `entities(id)`; each
    /// gets an `ON DELETE CASCADE` foreign key.
    #[serde(default)]
    pub(crate) entity_refs: Vec<String>,
}

/// When a system runs: once when a new game starts, or every tick.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Phase {
    /// Runs exactly once, when the engine bootstraps a new game.
    NewGame,
    /// Runs every time [`Engine::update`](super::Engine::update) is called.
    Update,
}

/// A single system: a name, the phase it runs in, and the SQL statements it
/// executes together as one transaction.
#[derive(Debug, Deserialize)]
pub(crate) struct SystemDef {
    /// The system's name, used only for identification/debugging.
    pub(crate) name: String,
    /// The phase this system runs in.
    pub(crate) phase: Phase,
    /// The SQL statements to execute, in order, as one transaction.
    pub(crate) sql: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_components_and_systems() {
        let yaml = r#"
components:
  - name: position
    columns: "entity_id INTEGER NOT NULL PRIMARY KEY, x REAL NOT NULL"
    entity_refs: [entity_id]
systems:
  - name: seed
    phase: new_game
    sql:
      - "INSERT INTO entities DEFAULT VALUES"
  - name: tick
    phase: update
    sql:
      - "SELECT 1"
"#;
        let def: GameDefinition = serde_yaml_ng::from_str(yaml).expect("valid definition");

        assert_eq!(def.components.len(), 1);
        assert_eq!(def.components[0].name, "position");
        assert_eq!(
            def.components[0].columns,
            "entity_id INTEGER NOT NULL PRIMARY KEY, x REAL NOT NULL"
        );
        assert_eq!(def.components[0].entity_refs, vec!["entity_id"]);

        assert_eq!(def.systems.len(), 2);
        assert_eq!(def.systems[0].name, "seed");
        assert_eq!(def.systems[0].phase, Phase::NewGame);
        assert_eq!(
            def.systems[0].sql,
            vec!["INSERT INTO entities DEFAULT VALUES"]
        );
        assert_eq!(def.systems[1].name, "tick");
        assert_eq!(def.systems[1].phase, Phase::Update);
    }

    #[test]
    fn entity_refs_defaults_to_empty() {
        let yaml = r#"
components:
  - name: tag
    columns: "entity_id INTEGER NOT NULL, label TEXT NOT NULL"
systems: []
"#;
        let def: GameDefinition = serde_yaml_ng::from_str(yaml).expect("valid definition");

        assert!(def.components[0].entity_refs.is_empty());
    }

    #[test]
    fn rejects_malformed_yaml() {
        let result: Result<GameDefinition, _> = serde_yaml_ng::from_str("not: [valid");
        assert!(result.is_err());
    }
}
