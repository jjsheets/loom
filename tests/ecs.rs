//! Black-box integration tests against [`loom::ecs::Engine`]'s public API.

use loom::ecs::{EcsError, Engine};

const BASIC_FIXTURE: &str = "tests/fixtures/basic.yaml";
const BULK_FIXTURE: &str = "tests/fixtures/bulk.yaml";

fn count(engine: &Engine, sql: &str) -> i64 {
    engine
        .connection()
        .query_row(sql, [], |row| row.get(0))
        .expect("count query")
}

// --- Entity lifecycle and the entities/graveyard invariant ---------------

#[test]
fn spawn_entity_mints_increasing_ids_when_graveyard_empty() {
    let mut engine =
        Engine::from_yaml_str("components: []\nsystems: []").expect("valid definition");

    let first = engine.spawn_entity().expect("spawn");
    let second = engine.spawn_entity().expect("spawn");
    let third = engine.spawn_entity().expect("spawn");

    assert_eq!(first, 1);
    assert_eq!(second, 2);
    assert_eq!(third, 3);
}

#[test]
fn despawn_moves_entity_to_graveyard() {
    let mut engine =
        Engine::from_yaml_str("components: []\nsystems: []").expect("valid definition");
    let id = engine.spawn_entity().expect("spawn");

    engine.despawn_entity(id).expect("despawn");

    assert_eq!(count(&engine, "SELECT COUNT(*) FROM entities"), 0);
    assert_eq!(
        count(
            &engine,
            &format!("SELECT COUNT(*) FROM graveyard WHERE id = {id}")
        ),
        1
    );
}

#[test]
fn despawn_then_spawn_recycles_the_same_id() {
    let mut engine =
        Engine::from_yaml_str("components: []\nsystems: []").expect("valid definition");
    let id = engine.spawn_entity().expect("spawn");

    engine.despawn_entity(id).expect("despawn");
    let recycled = engine.spawn_entity().expect("spawn again");

    assert_eq!(recycled, id, "spawn_entity should recycle the despawned id");
    assert_eq!(count(&engine, "SELECT COUNT(*) FROM graveyard"), 0);
}

#[test]
fn despawn_then_spawn_recycled_entity_has_no_leftover_component_rows() {
    let yaml = r#"
components:
  - name: tag
    columns: "entity_id INTEGER NOT NULL, label TEXT NOT NULL"
    entity_refs: [entity_id]
systems: []
"#;
    let mut engine = Engine::from_yaml_str(yaml).expect("valid definition");
    let id = engine.spawn_entity().expect("spawn");
    engine
        .connection()
        .execute(
            &format!("INSERT INTO tag (entity_id, label) VALUES ({id}, 'old life')"),
            [],
        )
        .expect("insert tag row");

    engine.despawn_entity(id).expect("despawn");
    let recycled = engine.spawn_entity().expect("spawn again");

    assert_eq!(recycled, id);
    assert_eq!(
        count(
            &engine,
            &format!("SELECT COUNT(*) FROM tag WHERE entity_id = {id}")
        ),
        0,
        "the recycled entity should start with no rows left over from its previous life"
    );
}

#[test]
fn spawn_recycles_smallest_graveyard_id_first() {
    let mut engine =
        Engine::from_yaml_str("components: []\nsystems: []").expect("valid definition");
    let a = engine.spawn_entity().expect("spawn a");
    let b = engine.spawn_entity().expect("spawn b");
    let c = engine.spawn_entity().expect("spawn c");
    engine.despawn_entity(c).expect("despawn c");
    engine.despawn_entity(a).expect("despawn a");
    engine.despawn_entity(b).expect("despawn b");

    let recycled = engine.spawn_entity().expect("spawn recycled");

    assert_eq!(
        recycled, a,
        "the smallest graveyard id (a) should be recycled first"
    );
}

#[test]
fn recycling_exhaustion_then_mints_fresh_id() {
    let mut engine =
        Engine::from_yaml_str("components: []\nsystems: []").expect("valid definition");
    let a = engine.spawn_entity().expect("spawn a");
    engine.despawn_entity(a).expect("despawn a");

    let recycled = engine.spawn_entity().expect("spawn recycled");
    let fresh = engine.spawn_entity().expect("spawn fresh");

    assert_eq!(recycled, a);
    assert_eq!(
        fresh, 2,
        "graveyard is empty again, so a new id must be minted"
    );
}

#[test]
fn entities_union_graveyard_invariant_holds_across_mixed_operations() {
    let mut engine =
        Engine::from_yaml_str("components: []\nsystems: []").expect("valid definition");
    let mut minted = Vec::new();

    for _ in 0..3 {
        minted.push(engine.spawn_entity().expect("spawn"));
    }
    engine.despawn_entity(minted[1]).expect("despawn");
    minted.push(engine.spawn_entity().expect("spawn recycled"));
    engine.despawn_entity(minted[0]).expect("despawn");
    engine.despawn_entity(minted[2]).expect("despawn");
    minted.push(engine.spawn_entity().expect("spawn"));
    minted.push(engine.spawn_entity().expect("spawn"));

    let union_count: i64 = count(
        &engine,
        "SELECT COUNT(*) FROM (SELECT id FROM entities UNION SELECT id FROM graveyard)",
    );
    let overlap_count: i64 = count(
        &engine,
        "SELECT COUNT(*) FROM entities e JOIN graveyard g ON g.id = e.id",
    );

    let mut unique_minted = minted.clone();
    unique_minted.sort_unstable();
    unique_minted.dedup();

    assert_eq!(overlap_count, 0, "no id should be in both tables at once");
    assert_eq!(
        union_count,
        unique_minted.len() as i64,
        "entities \u{222a} graveyard must equal every id ever minted"
    );
}

// --- Component cascade delete and per-entity cardinality ------------------

#[test]
fn despawn_cascades_single_entity_ref_component() {
    let mut engine = Engine::new(BASIC_FIXTURE).expect("valid fixture");
    assert_eq!(
        count(&engine, "SELECT COUNT(*) FROM position WHERE entity_id = 1"),
        1
    );

    engine.despawn_entity(1).expect("despawn entity 1");

    assert_eq!(
        count(&engine, "SELECT COUNT(*) FROM position WHERE entity_id = 1"),
        0
    );
    assert_eq!(
        count(&engine, "SELECT COUNT(*) FROM velocity WHERE entity_id = 1"),
        0
    );
}

#[test]
fn despawning_carrier_removes_relationship_row() {
    let mut engine = Engine::new(BASIC_FIXTURE).expect("valid fixture");
    engine
        .connection()
        .execute(
            "INSERT INTO carries (carrier_id, item_id) VALUES (1, 2)",
            [],
        )
        .expect("insert carries row");

    engine.despawn_entity(1).expect("despawn carrier");

    assert_eq!(count(&engine, "SELECT COUNT(*) FROM carries"), 0);
}

#[test]
fn despawning_item_removes_relationship_row() {
    let mut engine = Engine::new(BASIC_FIXTURE).expect("valid fixture");
    engine
        .connection()
        .execute(
            "INSERT INTO carries (carrier_id, item_id) VALUES (1, 2)",
            [],
        )
        .expect("insert carries row");

    engine.despawn_entity(2).expect("despawn item");

    assert_eq!(count(&engine, "SELECT COUNT(*) FROM carries"), 0);
}

#[test]
fn component_with_unique_constraint_rejects_duplicate_insert() {
    let engine = Engine::new(BASIC_FIXTURE).expect("valid fixture");

    let result = engine.connection().execute(
        "INSERT INTO position (entity_id, x, y) VALUES (1, 9.0, 9.0)",
        [],
    );

    assert!(
        result.is_err(),
        "position has a PRIMARY KEY on entity_id, so a duplicate insert must fail"
    );
}

#[test]
fn component_without_unique_constraint_allows_multiple_rows() {
    let engine = Engine::new(BASIC_FIXTURE).expect("valid fixture");

    engine
        .connection()
        .execute("INSERT INTO tag (entity_id, label) VALUES (1, 'first')", [])
        .expect("first tag row");
    engine
        .connection()
        .execute(
            "INSERT INTO tag (entity_id, label) VALUES (1, 'second')",
            [],
        )
        .expect("second tag row for the same entity should be allowed");

    assert_eq!(
        count(&engine, "SELECT COUNT(*) FROM tag WHERE entity_id = 1"),
        2
    );
}

// --- new_game / update lifecycle ------------------------------------------

#[test]
fn new_game_systems_run_exactly_once_and_seed_expected_state() {
    let engine = Engine::new(BASIC_FIXTURE).expect("valid fixture");

    assert_eq!(count(&engine, "SELECT COUNT(*) FROM entities"), 2);
    assert_eq!(count(&engine, "SELECT COUNT(*) FROM position"), 2);
    assert_eq!(count(&engine, "SELECT COUNT(*) FROM velocity"), 2);

    let x: f64 = engine
        .connection()
        .query_row("SELECT x FROM position WHERE entity_id = 1", [], |row| {
            row.get(0)
        })
        .expect("seeded position");
    assert_eq!(x, 0.0);
}

#[test]
fn update_is_not_implicitly_run_during_construction() {
    let engine = Engine::new(BASIC_FIXTURE).expect("valid fixture");

    let x: f64 = engine
        .connection()
        .query_row("SELECT x FROM position WHERE entity_id = 1", [], |row| {
            row.get(0)
        })
        .expect("seeded position");

    assert_eq!(
        x, 0.0,
        "apply_velocity (an update-phase system) must not have run yet"
    );
}

#[test]
fn repeated_update_calls_accumulate_state() {
    let mut engine = Engine::new(BASIC_FIXTURE).expect("valid fixture");

    for _ in 0..3 {
        engine.update().expect("update tick");
    }

    for entity_id in [1, 2] {
        let (x, y): (f64, f64) = engine
            .connection()
            .query_row(
                &format!("SELECT x, y FROM position WHERE entity_id = {entity_id}"),
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("position after ticks");
        assert_eq!(x, 3.0, "entity {entity_id}: x = 0.0 + 3 * dx(1.0)");
        assert_eq!(y, 1.5, "entity {entity_id}: y = 0.0 + 3 * dy(0.5)");
    }
}

#[test]
fn system_using_temp_table_leaves_none_behind_and_runs_repeatedly() {
    let mut engine = Engine::new(BASIC_FIXTURE).expect("valid fixture");

    engine.update().expect("first update");
    assert_eq!(
        count(&engine, "SELECT COUNT(*) FROM sqlite_temp_master"),
        0,
        "apply_velocity's temp table should not survive past its own system"
    );

    engine
        .update()
        .expect("second update should not fail with 'table already exists'");
}

#[test]
fn same_phase_systems_run_in_declaration_order() {
    let yaml = r#"
components:
  - name: state
    columns: "entity_id INTEGER NOT NULL PRIMARY KEY, flag INTEGER NOT NULL DEFAULT 0, moved INTEGER NOT NULL DEFAULT 0"
    entity_refs: [entity_id]
systems:
  - name: seed
    phase: new_game
    sql:
      - "INSERT INTO entities DEFAULT VALUES"
      - "INSERT INTO state (entity_id) VALUES (1)"
  - name: set_flag
    phase: update
    sql:
      - "UPDATE state SET flag = 1 WHERE entity_id = 1"
  - name: move_if_flagged
    phase: update
    sql:
      - "UPDATE state SET moved = 1 WHERE entity_id = 1 AND flag = 1"
"#;
    let mut engine = Engine::from_yaml_str(yaml).expect("valid definition");

    engine.update().expect("update tick");

    let moved: i64 = engine
        .connection()
        .query_row("SELECT moved FROM state WHERE entity_id = 1", [], |row| {
            row.get(0)
        })
        .expect("state row");
    assert_eq!(
        moved, 1,
        "move_if_flagged must run after set_flag within the same update() call"
    );
}

// --- Bulk operations -------------------------------------------------------

#[test]
fn bulk_despawn_cascades_every_row_in_a_large_group() {
    let yaml = r#"
components:
  - name: tag
    columns: "entity_id INTEGER NOT NULL, label TEXT NOT NULL"
    entity_refs: [entity_id]
systems: []
"#;
    let mut engine = Engine::from_yaml_str(yaml).expect("valid definition");
    let ids: Vec<i64> = (0..10)
        .map(|_| engine.spawn_entity().expect("spawn"))
        .collect();
    for id in &ids {
        engine
            .connection()
            .execute(
                &format!("INSERT INTO tag (entity_id, label) VALUES ({id}, 'x')"),
                [],
            )
            .expect("insert tag row");
    }

    engine
        .connection()
        .execute(
            "DELETE FROM entities WHERE id IN (SELECT id FROM entities)",
            [],
        )
        .expect("bulk despawn");

    assert_eq!(count(&engine, "SELECT COUNT(*) FROM entities"), 0);
    assert_eq!(count(&engine, "SELECT COUNT(*) FROM graveyard"), 10);
    assert_eq!(
        count(&engine, "SELECT COUNT(*) FROM tag"),
        0,
        "cascade delete must remove every tag row for the whole despawned group"
    );
}

#[test]
fn bulk_spawn_recycles_only_up_to_the_requested_count_when_graveyard_has_more() {
    let mut engine = Engine::new(BULK_FIXTURE).expect("valid fixture");

    engine.update().expect("bulk_spawn_recycle_only");

    assert_eq!(count(&engine, "SELECT COUNT(*) FROM entities"), 3);
    assert_eq!(
        count(
            &engine,
            "SELECT COUNT(*) FROM entities WHERE id IN (1, 2, 3)"
        ),
        3,
        "the 3 smallest graveyard ids should have been recycled"
    );
    assert_eq!(
        count(&engine, "SELECT COUNT(*) FROM graveyard"),
        2,
        "2 graveyard ids should be left untouched since only 3 were requested"
    );
}

// --- Error handling ---------------------------------------------------------

#[test]
fn missing_yaml_file_returns_io_error() {
    let result = Engine::new("tests/fixtures/does_not_exist.yaml");

    assert!(matches!(result, Err(EcsError::Io(_))));
}

#[test]
fn invalid_yaml_content_returns_yaml_error() {
    let result = Engine::from_yaml_str("not: [valid");

    assert!(matches!(result, Err(EcsError::Yaml(_))));
}

#[test]
fn invalid_component_sql_returns_sql_error() {
    let yaml = r#"
components:
  - name: broken
    columns: "this is not valid SQL ((("
systems: []
"#;

    let result = Engine::from_yaml_str(yaml);

    assert!(matches!(result, Err(EcsError::Sql(_))));
}

#[test]
fn invalid_new_game_system_sql_leaves_no_engine_and_reports_system_name() {
    let yaml = r#"
components: []
systems:
  - name: broken_system
    phase: new_game
    sql:
      - "SELECT * FROM this_table_does_not_exist"
"#;

    let result = Engine::from_yaml_str(yaml);

    match result {
        Err(EcsError::System { name, .. }) => assert_eq!(name, "broken_system"),
        Err(other) => panic!("expected EcsError::System, got a different error: {other}"),
        Ok(_) => panic!("expected EcsError::System, but construction succeeded"),
    }
}
