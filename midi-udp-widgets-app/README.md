![CAW Logo](../assets/logo.png)

# caw_midi_udp_widgets_app

[![Version](https://img.shields.io/crates/v/caw_midi_udp_widgets_app.svg)](https://crates.io/crates/caw_midi_udp_widgets_app)
[![Documentation](https://docs.rs/caw_midi_udp_widgets_app/badge.svg)](https://docs.rs/caw_midi_udp_widgets_app)
[![test](https://github.com/gridbugs/caw/actions/workflows/test.yml/badge.svg)](https://github.com/gridbugs/caw/actions/workflows/test.yml)
[![dependency status](https://deps.rs/repo/github/gridbugs/caw/status.svg)](https://deps.rs/repo/github/gridbugs/caw)

App for launching widgets that communicate with a caw synthesizer by sending
midi commands over UDP. Sometimes it can be useful to pop up a graphical window
and manually interact with a widget. This app can open windows containing
widgets that can act as UDP clients, sending MIDI commands to a UDP server,
such as is contained in the
[caw_midi_udp](https://crates.io/crates/caw_midi_udp).

Part of the [CAW Synthesizer Framework](..).
