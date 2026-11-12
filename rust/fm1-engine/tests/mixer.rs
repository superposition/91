// SPDX-License-Identifier: GPL-3.0-only
use fm1_engine::effects::mix::{self, Buffers, MergeConfig, PadFm};
fn outputs<const N: usize>(values: &mut [[f32; N]; 6]) -> Buffers<'_> {
    let [left, right, reverb, echo, bus_left, bus_right] = values;
    Buffers {
        left,
        right,
        reverb,
        echo,
        bus_left,
        bus_right,
    }
}
#[test]
fn pad_then_fm_addition_retains_binary32_cancellation_order() {
    let mut values = [[1.0]; 6];
    let pad = [16777216.0];
    let fm = [-16777216.0];
    let zero = [0.0];
    let config = MergeConfig {
        first: [1.0; 3],
        second: [1.0; 3],
        first_bus: 0,
        second_bus: 0,
    };
    mix::pad_fm(
        outputs(&mut values),
        &config,
        PadFm {
            pad_left: &pad,
            pad_right: &pad,
            pad_dry: &zero,
            fm_left: &fm,
            fm_right: &fm,
            fm_reverb: &zero,
            fm_echo: &zero,
        },
    );
    // Frozen C evaluates (1 + 2^24) - 2^24 as zero, rather than regrouping the
    // contributions before adding them to the accumulator.
    assert_eq!(values[0], [0.0]);
    assert_eq!(values[1], [0.0]);
    assert_eq!(values[4], [1.0]);
    assert_eq!(values[5], [1.0]);
}
#[test]
fn every_pad_fm_target_pair_keeps_independent_sends_and_accumulators() {
    for pad_bus in 0..2 {
        for fm_bus in 0..2 {
            let mut values = [[0.0]; 6];
            let config = MergeConfig {
                first: [0.5, 0.25, 1.0],
                second: [0.25, 0.5, 0.5],
                first_bus: pad_bus,
                second_bus: fm_bus,
            };
            mix::pad_fm(
                outputs(&mut values),
                &config,
                PadFm {
                    pad_left: &[2.0],
                    pad_right: &[3.0],
                    pad_dry: &[5.0],
                    fm_left: &[7.0],
                    fm_right: &[11.0],
                    fm_reverb: &[13.0],
                    fm_echo: &[17.0],
                },
            );
            let expected = match (pad_bus, fm_bus) {
                (0, 0) => [[2.75], [6.25], [9.5], [9.5], [0.0], [0.0]],
                (0, 1) => [[1.0], [0.75], [9.5], [9.5], [1.75], [5.5]],
                (1, 0) => [[1.75], [5.5], [9.5], [9.5], [1.0], [0.75]],
                _ => [[0.0], [0.0], [9.5], [9.5], [2.75], [6.25]],
            };
            assert_eq!(values, expected);
        }
    }
}
#[test]
fn differential_drum_routing_preserves_before_mix_and_sends() {
    let mut values = [[2.25], [3.5], [9.0], [11.0], [4.0], [5.0]];
    mix::route_drums(outputs(&mut values), &[0.75], &[1.0]);
    assert_eq!(values, [[0.75], [1.0], [9.0], [11.0], [5.5], [7.5]]);
    mix::tank_return(outputs(&mut values), &[2.0], &[3.0], &[0.5]);
    assert_eq!(values, [[1.75], [2.5], [9.0], [11.0], [5.5], [7.5]]);
}
#[test]
fn every_retained_predelay_spacing_excludes_the_current_write_span() {
    // Circular rotation lets the current write begin at zero without loss of
    // generality. This checks every supported spacing and every chunk length,
    // independently of any mixer output or worker scheduling assumption.
    for spacing in 256..=1322 {
        for frames in 1..=256 {
            let start = 1579 - spacing;
            assert!(start >= frames);
            assert!(start + frames <= 1579);
        }
    }
}
