// SPDX-License-Identifier: GPL-3.0-only
use fm1_app::runner::{Error, Observation, Row, State};
#[test]
fn state_uses_existing_236_bytes_and_reset_preserves_run_lifetime_fields() {
    assert_eq!(core::mem::size_of::<State>(), 236);
    assert_eq!(core::mem::offset_of!(State, row), 108);
    assert_eq!(core::mem::size_of::<Row>(), 8);
    let mut state = State {
        best: 123,
        rows_born: 456,
        rng: 789,
        beat: 32.0,
        lane: 7,
        banner: [b'!'; 20],
        ..State::default()
    };
    state.reset();
    assert_eq!(
        (state.best, state.rows_born, state.rng, state.lane),
        (123, 456, 789, 7)
    );
    assert_eq!(state.start_beat, 32.0);
    assert_eq!(&state.banner[..12], b"KICK RUNNER\0");
    assert_eq!(state.banner[19], b'!');
    assert_eq!(state.ships, 3);
    assert!(state.row.iter().all(|r| *r == Row::default()));
}
#[test]
fn captured_start_timestamps_wrap_without_changing_long_lived_counters() {
    let mut state = State {
        best: 99,
        rows_born: 71,
        ..State::default()
    };
    state.start(
        Observation {
            now: u32::MAX,
            bars: 8,
            ..Observation::default()
        },
        [101, 102, u32::MAX],
    );
    assert_eq!(
        (state.s16_ms, state.start_ms, state.banner_until),
        (101, 102, 2499)
    );
    assert_eq!(state.beat, 2.0);
    assert_eq!((state.best, state.rows_born), (99, 71));
    assert_eq!(state.rng, u32::MAX.wrapping_mul(2_654_435_761) | 1);
}
#[test]
fn walls_take_precedence_then_last_ship_records_best_and_gates_wrap_score() {
    let mut state = State {
        on: 1,
        lane: 2,
        ships: 1,
        beat: 4.0,
        score: 111,
        best: 40,
        ..State::default()
    };
    state.row[0] = Row {
        beat: 4.0,
        wall: 4,
        gate: 4,
        ..Row::default()
    };
    assert_eq!(state.resolve(0, 1234), Ok(false));
    assert_eq!(
        (state.over, state.ships, state.best, state.over_ms),
        (1, 0, 111, 1234)
    );
    assert_eq!(state.row[0].taken, 0);
    state.reset();
    state.layer = 5;
    state.score = u32::MAX - 49;
    state.row[1] = Row {
        beat: 4.0,
        gate: 4,
        ..Row::default()
    };
    assert_eq!(state.resolve(1, 1234), Ok(false));
    assert_eq!(state.score, 50); // 60 + full-track bonus40, unsigned C wrapping.
    assert_eq!((state.gates, state.row[1].taken), (1, 1));
    let before = state;
    assert_eq!(state.resolve(16, 0), Err(Error::Index));
    assert_eq!(state, before);
}
#[test]
fn band_clock_uses_wrapping_ms_and_rejects_nan_before_mutation() {
    let mut state = State {
        on: 1,
        ships: 3,
        s16: 4,
        s16_ms: u32::MAX - 99,
        step_ms: 125,
        beat: 1.0,
        ..State::default()
    };
    state
        .advance(Observation {
            now: 25,
            bars: 5,
            playing: 1,
            kick: 0.0,
        })
        .unwrap();
    assert_eq!((state.s16, state.s16_ms, state.step_ms), (5, 25, 125));
    assert_eq!(state.beat, 1.25);
    let before = state;
    assert_eq!(
        state.advance(Observation {
            kick: f32::NAN,
            ..Observation::default()
        }),
        Err(Error::State)
    );
    assert_eq!(state, before);
}
#[test]
fn steering_uses_detent_sign_masks_have_checked_ids_and_leds_keep_thresholds() {
    let mut state = State {
        on: 1,
        ships: 3,
        ..State::default()
    };
    state.steer(i32::MIN, false, false).unwrap();
    assert_eq!(state.lane, 7);
    state.steer(i32::MAX, true, true).unwrap();
    assert_eq!(state.lane, 0);
    assert_eq!(state.masks(12, &[1, 2, 5, 4, 6]), Ok([0, 0x76]));
    state.lp_until = 1.0;
    assert_eq!(state.masks(12, &[1, 2, 5, 4, 6]), Ok([4096, 0x76]));
    assert_eq!(state.masks(32, &[1, 2, 5, 4, 6]), Err(Error::Index));
    let white = [0, 2, 4, 6, 7, 9, 11, 12];
    let black = [1, 3, 5, 8, 10, 13, 15, 17, 20, 22, 25];
    state.row[0] = Row {
        beat: 1.0,
        wall: 4,
        ..Row::default()
    };
    assert_eq!(
        state.leds(true, 6, [10, 0], &[0.35, 0.351], &white, &black),
        Ok([1024, 1 | 8 | 16])
    );
    assert_eq!(
        state.leds(true, 0, [10, 0], &[0.35, 0.351], &white, &black),
        Ok([1024, 1 | 8])
    );
}
#[test]
fn full_width_game_over_scores_use_bounded_text_and_keep_strip_canaries() {
    use fm1_app::{
        graphics::{Canvas, Font, State as CanvasState},
        runner_draw,
    };
    let advance = [1; 64];
    let width = [1; 64];
    let offset = [0; 64];
    let font = Font::new(1, 0, 32, 95, &advance, &width, &offset, &[0xf0]).unwrap();
    let mut pixels = [0x55aa; 240 * 120 + 2];
    let mut canvas = Canvas::new(
        &mut pixels[1..240 * 120 + 1],
        CanvasState::begin(240, 120, 240 * 120, 0).unwrap(),
    )
    .unwrap();
    let game = State {
        on: 1,
        over: 1,
        ships: 3,
        score: u32::MAX,
        best: u32::MAX,
        ..State::default()
    };
    runner_draw::draw(
        &mut canvas,
        &game,
        1000,
        true,
        [0x1234, 0xffff, 0x4567, 0x789a],
        [&font, &font, &font],
    )
    .unwrap();
    assert!(canvas.writes() > 0);
    assert_eq!(pixels[0], 0x55aa);
    assert_eq!(pixels[240 * 120 + 1], 0x55aa);
    // 6+10+7+10 bytes, centered at x120; every synthetic glyph is one pixel.
    assert!(pixels[1 + 104 * 240 + 104..1 + 104 * 240 + 137]
        .iter()
        .all(|p| *p == 0xffff));
}
#[test]
fn hostile_draw_state_is_rejected_before_raster_work() {
    use fm1_app::{
        graphics::{Canvas, Error as CanvasError, Font, State as CanvasState},
        runner_draw,
    };
    let table = [1; 64];
    let offset = [0; 64];
    let font = Font::new(1, 0, 32, 95, &table, &table, &offset, &[0xf0]).unwrap();
    let mut pixels = [0x55aa; 16];
    let mut canvas = Canvas::new(&mut pixels, CanvasState::begin(4, 4, 16, 0).unwrap()).unwrap();
    let game = State {
        view: 1e30,
        ..State::default()
    };
    assert_eq!(
        runner_draw::draw(&mut canvas, &game, 0, false, [0; 4], [&font, &font, &font]),
        Err(CanvasError::Canvas)
    );
    assert_eq!(canvas.writes(), 0);
    assert_eq!(pixels, [0x55aa; 16]);
}
#[test]
fn restart_gate_and_actions_preserve_exact_hold_edges_and_900ms_boundary() {
    let mut game = State {
        on: 1,
        over: 1,
        over_ms: u32::MAX - 99,
        ..State::default()
    };
    assert!(!game.restart_candidate(4, 4));
    assert!(game.restart_candidate(4, 0));
    assert!(!game.restart(800));
    assert!(game.restart(801));
    assert_eq!(game.over, 0);
    let masks = [1 << 10, 1 << 6, 1 << 4, 1 << 1];
    assert_eq!(State::buttons(1 << 10, 1 << 10, masks), 1);
    assert_eq!(State::buttons(1 << 4, 1 << 4, masks), 0);
    assert_eq!(State::buttons(1 << 4, (1 << 4) | (1 << 1), masks), 2);
    assert_eq!(State::buttons((1 << 10) | (1 << 6), 0, masks), 3);
}
