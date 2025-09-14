use crate::window::Window;
use caw_computer_keyboard::Key;
use caw_keyboard::Note;
use midly::{MidiMessage, num::u7};
use sdl2::{keyboard::Scancode, pixels::Color};
use std::{collections::HashMap, time::Instant};

const WIDTH_PX: u32 = 128;
const HEIGHT_PX: u32 = 128;

struct KeyMappings(HashMap<Key, Note>);

impl KeyMappings {
    fn note_from_scancode(&self, scancode: Scancode) -> Option<Note> {
        caw_sdl2::sdl2_scancode_to_key(scancode)
            .and_then(|key| self.0.get(&key).cloned())
    }
}

pub struct ComputerKeyboard {
    window: Window,
    key_mappings: KeyMappings,
}

const SPACEBAR_CONTROLLER: u7 = u7::from_int_lossy(0);

impl ComputerKeyboard {
    pub fn new(title: Option<&str>, start_note: Note) -> anyhow::Result<Self> {
        let window = Window::new(title, WIDTH_PX, HEIGHT_PX)?;
        let key_mappings = KeyMappings(
            caw_computer_keyboard::opinionated_note_by_key(start_note)
                .into_iter()
                .collect(),
        );
        Ok(Self {
            window,
            key_mappings,
        })
    }

    fn handle_events(&mut self, buf: &mut Vec<MidiMessage>) {
        for event in self.window.event_pump.poll_iter() {
            use sdl2::event::Event;
            match event {
                Event::Quit { .. } => std::process::exit(0),
                Event::KeyDown {
                    scancode: Some(Scancode::Space),
                    ..
                } => buf.push(MidiMessage::Controller {
                    controller: SPACEBAR_CONTROLLER,
                    value: u7::max_value(),
                }),
                Event::KeyUp {
                    scancode: Some(Scancode::Space),
                    ..
                } => buf.push(MidiMessage::Controller {
                    controller: SPACEBAR_CONTROLLER,
                    value: 0.into(),
                }),

                Event::KeyDown {
                    scancode: Some(scancode),
                    ..
                } => {
                    if let Some(note) =
                        self.key_mappings.note_from_scancode(scancode)
                    {
                        buf.push(MidiMessage::NoteOn {
                            key: note.to_midi_index().into(),
                            vel: u7::max_value(),
                        });
                    }
                }
                Event::KeyUp {
                    scancode: Some(scancode),
                    ..
                } => {
                    if let Some(note) =
                        self.key_mappings.note_from_scancode(scancode)
                    {
                        buf.push(MidiMessage::NoteOff {
                            key: note.to_midi_index().into(),
                            vel: u7::max_value(),
                        });
                    }
                }
                _ => (),
            }
        }
    }

    fn render(&mut self) -> anyhow::Result<()> {
        self.window.canvas.set_draw_color(Color::BLACK);
        self.window.canvas.clear();
        self.window.render_title()?;
        self.window.canvas.present();
        Ok(())
    }

    pub fn tick(&mut self, buf: &mut Vec<MidiMessage>) -> anyhow::Result<()> {
        buf.clear();
        self.window.wait_until_next_frame();
        self.handle_events(buf);
        self.render()?;
        self.window.prev_tick_complete = Instant::now();
        Ok(())
    }
}
