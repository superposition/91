#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
export MIRIFLAGS=-Zmiri-strict-provenance
cargo +nightly-2026-08-28 miri test --locked -p fm1-jam-bridge --features hal-test retained_dma_root_pointer_remains_valid_between_call_scoped_updates
cargo +nightly-2026-08-28 miri test --locked -p fm1-jam-bridge --features hal-test foreign_uninitialized_load_is_initialized_only_after_verified_success
cargo +nightly-2026-08-28 miri test --locked -p fm1-jam-bridge --features hal-test retained_uart_root_survives_scoped_parser_calls_without_dma_references
for test in retained_ep0_root_and_uninitialized_destination_need_no_mutable_dma_reference retained_foreign_roots_keep_busy_payload_length_and_packet_bounds retained_raw_queue_roots_and_uninitialized_delivery_are_scoped;do
    cargo +nightly-2026-08-28 miri test --locked -p fm1-jam-bridge --features hal-test "$test"
done
cargo +nightly-2026-08-28 miri test --locked -p fm1-jam-bridge --features hal-test host_tx_retains_raw_source_through_busy_then_reuses_only_after_retirement
cargo +nightly-2026-08-28 miri test --locked -p fm1-jam-bridge --features hal-test raw_control_initializes_only_completed_uninitialized_output_prefix_and_seals_unknown
cargo +nightly-2026-08-28 miri test --locked -p fm1-hal --test usb_host
cargo +nightly-2026-08-28 miri test --locked -p fm1-hal --test usb_host_control
cargo +nightly-2026-08-28 miri test --locked -p fm1-jam-bridge --features hal-test retained_roots_survive_a_scoped_producer_reentered_only_by_its_consumer
cargo +nightly-2026-08-28 miri test --locked -p fm1-hal --test usb_endpoint
for test in copied_configuration_initializes_every_result_byte_including_reserved retained_rx_and_queue_roots_survive_scoped_calls_without_foreign_views;do
    cargo +nightly-2026-08-28 miri test --locked -p fm1-jam-bridge --features hal-test "$test"
done
