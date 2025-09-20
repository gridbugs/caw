use crate::window::Window;
use anyhow::anyhow;
use caw_computer_keyboard::Key;
use caw_keyboard::Note;
use caw_window_utils::font::{Font, load_font};
use midly::{MidiMessage, num::u7};
use sdl2::{keyboard::Scancode, pixels::Color, rect::Rect};
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

#[derive(Default)]
struct PressedKeys {
    keys: Vec<Note>,
}

impl PressedKeys {
    fn insert(&mut self, note: Note) -> bool {
        if self.keys.contains(&note) {
            false
        } else {
            self.keys.push(note);
            true
        }
    }

    fn remove(&mut self, note: Note) {
        let removed = self
            .keys
            .iter()
            .cloned()
            .filter(|&n| n != note)
            .collect::<Vec<_>>();
        self.keys = removed;
    }
}

pub struct ComputerKeyboard {
    window: Window,
    key_mappings: KeyMappings,
    pressed_keys: PressedKeys,
    font: Font<'static, 'static>,
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
            pressed_keys: Default::default(),
            font: load_font(48)?,
        })
    }

    fn handle_events(&mut self, buf: &mut Vec<MidiMessage>) {
        for event in self.window.event_pump.poll_iter() {
            use sdl2::event::Event;
            Window::handle_event_common(
                event.clone(),
                self.window.title.as_ref(),
            );
            match event {
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
                        if self.pressed_keys.insert(note) {
                            buf.push(MidiMessage::NoteOn {
                                key: note.to_midi_index().into(),
                                vel: u7::max_value(),
                            });
                        }
                    }
                }
                Event::KeyUp {
                    scancode: Some(scancode),
                    ..
                } => {
                    if let Some(note) =
                        self.key_mappings.note_from_scancode(scancode)
                    {
                        self.pressed_keys.remove(note);
                        buf.push(MidiMessage::NoteOff {
                            key: note.to_midi_index().into(),
                            vel: 0.into(),
                        });
                    }
                }
                _ => (),
            }
        }
    }

    pub fn render_note(&mut self) -> anyhow::Result<()> {
        let text = if let Some(last) = self.pressed_keys.keys.last() {
            last.to_string()
        } else {
            return Ok(());
        };
        let text_surface = self
            .font
            .render(text.as_str())
            .blended(Color::WHITE)
            .map_err(|e| anyhow!("{e}"))?;
        let text_texture =
            text_surface.as_texture(&self.window.texture_creator)?;
        let (canvas_width, _canvas_height) = self
            .window
            .canvas
            .output_size()
            .map_err(|e| anyhow!("{e}"))?;
        let text_texture_query = text_texture.query();
        let value_space_px = 20;
        // Render the title centred at the bottom of the window.
        let text_rect = Rect::new(
            (canvas_width as i32 - text_texture_query.width as i32) / 2,
            value_space_px,
            text_texture_query.width,
            text_texture_query.height,
        );
        self.window
            .canvas
            .copy(&text_texture, None, Some(text_rect))
            .map_err(|e| anyhow!("{e}"))?;
        Ok(())
    }

    fn render(&mut self) -> anyhow::Result<()> {
        self.window.canvas.set_draw_color(Color::BLACK);
        self.window.canvas.clear();
        self.window.render_title()?;
        self.render_note()?;
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
