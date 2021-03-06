#[allow(dead_code)]
mod user_interface;
#[allow(dead_code)]
mod util;

use crate::memo;
use crate::tui::user_interface::{ui, App};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    error::Error,
    io::stdout,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use tui::{backend::CrosstermBackend, Terminal};

enum Event<I> {
    Input(I),
    Tick,
}

/// Crossterm demo
#[derive(Debug)]
struct Cli {
    /// time in ms between two ticks.
    tick_rate: u64,
    /// whether unicode symbols are used to improve the overall look of the app
    enhanced_graphics: bool,
}

// TODO:日本語対応
pub fn read_line() -> Result<String, Box<dyn Error>> {
    let mut line = String::new();
    while let CEvent::Key(KeyEvent { code, .. }) = event::read()? {
        match code {
            KeyCode::Enter => {
                break;
            }
            KeyCode::Char(c) => {
                line.push(c);
            }
            _ => {}
        }
    }

    Ok(line)
}

pub fn launch_tui(lst_memo: &Vec<memo::Memo>) -> Result<(), Box<dyn Error>> {

    let cli: Cli = Cli{tick_rate:250, enhanced_graphics:true};

    enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(backend)?;

    // Setup input handling
    let (tx, rx) = mpsc::channel();

    let tick_rate = Duration::from_millis(cli.tick_rate);
    thread::spawn(move || {
            let mut last_tick = Instant::now();
            loop {
                // poll for tick rate duration, if no events, sent tick event.
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or_else(|| Duration::from_secs(0));
                if event::poll(timeout).unwrap() {
                    if let CEvent::Key(key) = event::read().unwrap() {
                        tx.send(Event::Input(key)).unwrap();
                    }
                }
                if last_tick.elapsed() >= tick_rate {
                    match tx.send(Event::Tick) {
                        Err(e) => {
                            panic!("send error:{}", e);
                        },
                        _ => {}
                    }
                    last_tick = Instant::now();
                }
            }
        });

    let mut app = App::new("Crossterm Demo", lst_memo, cli.enhanced_graphics);

    terminal.clear()?;

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;
        match rx.recv()? {
            Event::Input(event) => match event.modifiers {
                KeyModifiers::NONE => {
                    match event.code {
                        // TODO:本当は、on_keyの実装はmain内でやるべき？'/'の実装はappでやりたいが。。
                        KeyCode::Char(c) => {
                            if c != '/' {
                                app.on_key(c, terminal.get_cursor().unwrap());
                            } else {
                                let search = read_line().unwrap();
                                app.search_string_in_this_path(&search);
                            }
                        }
                        KeyCode::Left => app.on_left(),
                        KeyCode::Up => app.on_up(),
                        KeyCode::Right => app.on_right(),
                        KeyCode::Down => app.on_down(),
                        KeyCode::Enter => app.on_enter_dir(),
                        KeyCode::Esc => {
                            disable_raw_mode()?;
                            execute!(
                                terminal.backend_mut(),
                                LeaveAlternateScreen,
                                DisableMouseCapture
                            )?;
                            terminal.show_cursor()?;
                            return Ok(());
                        }
                        _ => {},
                    }
                },
                _ => {
                    match event {
                        KeyEvent {
                            code: KeyCode::Char('d'),
                            modifiers: KeyModifiers::CONTROL,
                        } => { (0..4).for_each(|_| app.on_down()) },
                        KeyEvent {
                            code: KeyCode::Char('u'),
                            modifiers: KeyModifiers::CONTROL,
                        } => { (0..4).for_each(|_| app.on_up()) },
                        _ => {},
                    }
                }
            },
            Event::Tick => {
                app.on_tick();
            }
        }
        if app.should_quit {
            disable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )?;
            terminal.show_cursor()?;
            break;
        }
    }

    Ok(())
}
