use crossterm::event::{self, KeyEvent, KeyEventKind};
use std::time::Duration;

pub enum Event {
    Key(KeyEvent),
    Tick,
}

pub fn poll() -> color_eyre::Result<Event> {
    if event::poll(Duration::from_millis(50))? {
        if let event::Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                return Ok(Event::Key(key));
            }
        }
    }
    Ok(Event::Tick)
}
