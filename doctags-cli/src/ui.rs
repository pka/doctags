use ansi_term::{ANSIString, ANSIStrings, Colour, Style};
use anyhow::Result;
use crossterm::{cursor, terminal, ClearType, InputEvent, KeyEvent, RawScreen};
use doctags::{search, Index};
use std::io::Write;

pub fn ui(index: &Index) -> Result<()> {
    #[derive(PartialEq)]
    enum State {
        Selecting,
        Quit,
        Selected(String),
    }
    let mut state = State::Selecting;
    let mut lines = Vec::new();
    if let Ok(_raw) = RawScreen::into_raw_mode() {
        // User input for search
        let mut searchinput = String::new();
        let mut selected = 0;

        let mut cursor = cursor();
        let _ = cursor.hide();
        let input = crossterm::input();
        let mut sync_stdin = input.read_sync();
        let (_cols, rows) = terminal().terminal_size();

        while state == State::Selecting {
            if let Ok(results) = search::search_matches(index, &searchinput, (rows - 1) as usize) {
                // Ignore empty results or search errors (e.g. incomplete ':' expression)
                if results.len() > 0 {
                    lines = results;
                }
            }
            paint_selection_list(&lines, selected);
            if let Some(ev) = sync_stdin.next() {
                match ev {
                    InputEvent::Keyboard(k) => match k {
                        KeyEvent::Esc | KeyEvent::Ctrl('c') => {
                            state = State::Quit;
                        }
                        KeyEvent::Up => {
                            if selected > 0 {
                                selected -= 1;
                            }
                        }
                        KeyEvent::Down => {
                            if selected + 1 < lines.len() {
                                selected += 1;
                            }
                        }
                        KeyEvent::Char('\n') => {
                            state = State::Selected(lines[selected].text.clone());
                        }
                        KeyEvent::Char(ch) => {
                            searchinput.push(ch);
                            selected = 0;
                        }
                        KeyEvent::Backspace => {
                            searchinput.pop();
                            selected = 0;
                        }
                        _ => {
                            // println!("{}", format!("OTHER InputEvent: {:?}\n\n", k));
                        }
                    },
                    _ => {}
                }
            }
            cursor.move_up(lines.len() as u16);
        }
        let (_x, y) = cursor.pos();
        cursor.goto(0, y)?;
        let _ = cursor.show();

        RawScreen::disable_raw_mode()?;
    }
    let terminal = terminal();
    let _ = terminal.clear(ClearType::FromCursorDown);

    match state {
        State::Selected(line) => {
            print!("{}", line);
        }
        _ => {}
    }
    Ok(())
}

fn paint_selection_list(lines: &Vec<search::Match>, selected: usize) {
    let terminal = terminal();
    let size = terminal.terminal_size();
    let width = size.0 as usize;
    let cursor = cursor();
    let (_x, y) = cursor.pos();
    for (i, line) in lines.iter().enumerate() {
        let _ = cursor.goto(0, y + (i as u16));
        let (style, highlighted) = if selected == i {
            (Colour::White.normal(), Colour::Cyan.normal())
        } else {
            (Colour::White.dimmed(), Colour::Cyan.normal())
        };
        let mut ansi_strings = highlight(&line, style, highlighted);
        for _ in line.text.len()..width {
            ansi_strings.push(style.paint(' '.to_string()));
        }
        println!("{}", ANSIStrings(&ansi_strings));
    }
    let _ = cursor.goto(0, y + (lines.len() as u16));
    print!("{}", Colour::Blue.paint("[ESC to quit, Enter to select]"));

    let _ = std::io::stdout().flush();
    // Clear additional lines from previous selection
    let _ = terminal.clear(ClearType::FromCursorDown);
}

fn highlight(line: &search::Match, normal: Style, highlighted: Style) -> Vec<ANSIString> {
    let mut ansi_strings = vec![];
    let snippet = &line.snippet;
    let parts = snippet.highlighted();
    if parts.len() == 0 {
        ansi_strings.push(normal.paint(&line.text));
    } else {
        let mut start_from = 0;
        for (start, end) in parts.iter().map(|h| h.bounds()) {
            ansi_strings.push(normal.paint(&snippet.fragments()[start_from..start]));
            ansi_strings.push(highlighted.paint(&snippet.fragments()[start..end]));
            start_from = end;
        }
        ansi_strings.push(normal.paint(&snippet.fragments()[start_from..]));
    }
    ansi_strings
}
