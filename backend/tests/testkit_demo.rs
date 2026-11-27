//! Testkit demo: how B3/B4 test against the frozen contracts WITHOUT B1's
//! and B2's real implementations. This is the pattern to copy — see
//! `src/game/README.md` and `src/rooms/README.md` §Testing.
//!
//! Runs green today against the fakes; the same shape of tests will run
//! against the real engine/registry when those modules land (B4's merge gate).

use rechess_backend::game::testkit::{starting_snapshot, FakeEngine};
use rechess_backend::game::ChessEngine as _;
use rechess_backend::game::{GameError, MoveRequest, PlayerColor};
use rechess_backend::protocol::ErrorCode;
use rechess_backend::rooms::testkit::{FakeHandle, FakeRegistry};
use rechess_backend::rooms::{
    CommandOutcome, JoinRoomError, ResumeError, RoomClosed, RoomCommand, RoomEvent, RoomEventKind,
    RoomHandle as _, RoomRegistry,
};

/// B1's contract: the fake satisfies it, so will the real engine.
#[tokio::test]
async fn fake_engine_scripts_results_and_records_calls() {
    let mut engine = FakeEngine::new(starting_snapshot());

    // Success path: apply a move, get a snapshot back.
    let snapshot = engine
        .apply_move(MoveRequest {
            from: "e2".into(),
            to: "e4".into(),
            promotion: None,
        })
        .expect("fake accepts by default");
    assert!(snapshot.fen.contains('r'), "snapshot carries a FEN");
    assert_eq!(engine.moves.len(), 1, "move was recorded");

    // Failure path: script the next error.
    engine.fail_next_with(GameError::WrongTurn);
    let err = engine
        .apply_move(MoveRequest {
            from: "e2".into(),
            to: "e4".into(),
            promotion: None,
        })
        .expect_err("scripted failure");
    assert_eq!(err, GameError::WrongTurn);

    // Lifecycle is recorded too.
    engine.resign(PlayerColor::Black).expect("resign ok");
    assert!(engine.calls.contains(&"resign:Black".to_string()));
    engine.reset_for_rematch();
    assert_eq!(engine.reset_count, 1);
}

/// B2's contract: the fake registry serves create/join/resume/lookup.
#[tokio::test]
async fn fake_registry_serves_the_room_lifecycle() {
    let registry = FakeRegistry::new("TESTCODE1");

    let created = registry.create_room("Host").await.expect("create ok");
    assert_eq!(created.room_code, "TESTCODE1");
    assert_eq!(created.credentials.color, PlayerColor::White);
    assert_eq!(
        created.room.status,
        rechess_backend::protocol::RoomStatus::Waiting
    );

    // Wrong code → NotFound; wrong token → InvalidToken (mirrors B3's wire errors).
    assert!(matches!(
        registry.join_room("WRONGCOD", "Guest").await,
        Err(JoinRoomError::NotFound)
    ));
    assert!(matches!(
        registry.resume_room("TESTCODE1", "bad-token").await,
        Err(ResumeError::InvalidToken)
    ));

    let joined = registry
        .join_room("TESTCODE1", "Guest")
        .await
        .expect("join ok");
    assert_eq!(joined.credentials.color, PlayerColor::Black);

    let resumed = registry
        .resume_room("TESTCODE1", &created.credentials.player_token)
        .await
        .expect("resume ok");
    assert_eq!(resumed.color, PlayerColor::White);
    assert_eq!(resumed.snapshot.room_code, "TESTCODE1");

    assert!(registry.lookup("TESTCODE1").await.is_some());
    assert!(registry.lookup("NOPE0000").await.is_none());
}

/// B2's contract: the fake handle records commands and emits events.
#[tokio::test]
async fn fake_handle_records_commands_and_emits_events() {
    let handle = FakeHandle::new();
    let mut events = handle.subscribe();

    let outcome = handle
        .send(RoomCommand::Move {
            player: "host".into(),
            from: "e2".into(),
            to: "e4".into(),
            promotion: None,
            expected_revision: 0,
        })
        .await
        .expect("send ok");
    assert!(matches!(outcome, CommandOutcome::Accepted { revision: 0 }));
    assert_eq!(handle.received().len(), 1, "command was recorded");

    // Scripted rejection (what B3 turns into game.move_rejected / server.error).
    handle.push_outcome(CommandOutcome::Rejected {
        error: ErrorCode::NotYourTurn,
        message: "not your turn".into(),
        revision: 3,
    });
    let outcome = handle
        .send(RoomCommand::Move {
            player: "host".into(),
            from: "d2".into(),
            to: "d4".into(),
            promotion: None,
            expected_revision: 3,
        })
        .await
        .expect("send ok");
    assert!(matches!(
        outcome,
        CommandOutcome::Rejected {
            error: ErrorCode::NotYourTurn,
            ..
        }
    ));

    // Events reach subscribers; MoveRejected is TARGETED (never broadcast).
    handle.emit(RoomEvent::to(
        "host".into(),
        RoomEventKind::MoveRejected {
            player_id: "host".into(),
            error: ErrorCode::IllegalMove,
            message: "illegal".into(),
            revision: 3,
            snapshot: None,
        },
    ));
    let event = events.recv().await.expect("event delivered");
    assert_eq!(event.target.as_deref(), Some("host"));

    // A closed room rejects further commands.
    handle.close();
    assert!(matches!(
        handle
            .send(RoomCommand::Leave {
                player: "host".into()
            })
            .await,
        Err(RoomClosed)
    ));
}

/// B3's consumption pattern: code written GENERICALLY over the registry
/// works unchanged against fakes (tests) and B2's real registry (prod).
#[tokio::test]
async fn generic_code_works_against_any_registry() {
    async fn create_and_subscribe<R: RoomRegistry>(
        registry: &R,
        nickname: &str,
    ) -> (R::Handle, bool) {
        let created = registry.create_room(nickname).await.expect("create ok");
        let events = created.handle.subscribe();
        let alive = events.is_empty(); // receiver created, queue empty
        (created.handle, alive)
    }

    let registry = FakeRegistry::new("TESTCODE1");
    let (handle, alive) = create_and_subscribe(&registry, "Host").await;
    assert!(alive);
    let outcome = handle
        .send(RoomCommand::Leave {
            player: "host".into(),
        })
        .await;
    assert!(outcome.is_ok());
}
