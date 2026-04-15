use crate::tui::app::{App, AppState, LogStatus};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Position},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
};

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn draw(frame: &mut Frame, app: &App) {
    if app.interactive {
        draw_interactive(frame, app);
    } else {
        draw_oneshot(frame, app);
    }
}

fn separator_line(width: u16) -> Line<'static> {
    Line::from(Span::styled(
        "─".repeat(width as usize),
        Style::default().fg(Color::DarkGray),
    ))
}

fn header_line(app: &App) -> Line<'static> {
    let muted = Style::default().fg(Color::DarkGray);
    let white = Style::default().fg(Color::White);
    let sep = Span::styled(" │ ", muted);
    Line::from(vec![
        Span::styled(app.browser_name.clone(), white),
        sep.clone(),
        Span::styled(format!("{} cookies", app.cookie_count), white),
        sep.clone(),
        Span::styled(app.output_dir.clone(), white),
        sep.clone(),
        Span::styled(format!("{}ms delay", app.delay_ms), white),
        sep.clone(),
        Span::styled(format!("{} max", app.max_pages), white),
    ])
}

fn status_line_widget(app: &App) -> Line<'static> {
    let spinner = SPINNER[app.spinner_frame % SPINNER.len()];
    match &app.state {
        AppState::Idle => Line::from(vec![
            Span::styled("○ ", Style::default().fg(Color::Cyan)),
            Span::styled("Ready. Type /help for commands.", Style::default().fg(Color::Cyan)),
        ]),
        AppState::Crawling => Line::from(vec![
            Span::styled(format!("{} ● ", spinner), Style::default().fg(Color::Magenta)),
            Span::styled(format!("Scraping: {}", app.url), Style::default().fg(Color::Magenta)),
        ]),
        AppState::Paused => Line::from(vec![
            Span::styled("❚❚ ", Style::default().fg(Color::Yellow)),
            Span::styled("Paused", Style::default().fg(Color::Yellow)),
        ]),
        AppState::Done => Line::from(vec![
            Span::styled("✓ ", Style::default().fg(Color::Green)),
            Span::styled(
                format!("Done — {} saved, {} failed, {} auth errors", app.saved, app.failed, app.auth_failed),
                Style::default().fg(Color::Green),
            ),
        ]),
        AppState::Error(msg) => Line::from(vec![
            Span::styled("✗ ", Style::default().fg(Color::Red)),
            Span::styled(format!("Error — {}", msg), Style::default().fg(Color::Red)),
        ]),
    }
}

fn progress_bar(app: &App, width: u16) -> Option<Line<'static>> {
    if matches!(app.state, AppState::Idle) {
        return None;
    }
    let (cur, total) = app.progress;
    let label = if total > 0 {
        format!(" {}/{} pages", cur, total)
    } else {
        format!(" {} pages", cur)
    };
    let label_len = label.len() as u16;
    let bar_width = width.saturating_sub(label_len).saturating_sub(1) as usize;
    let filled = if total > 0 {
        (cur * bar_width / total).min(bar_width)
    } else {
        0
    };
    let unfilled = bar_width.saturating_sub(filled);
    let bar_fill = "━".repeat(filled);
    let bar_empty = "─".repeat(unfilled);
    Some(Line::from(vec![
        Span::styled(bar_fill, Style::default().fg(Color::Magenta)),
        Span::styled(bar_empty, Style::default().fg(Color::DarkGray)),
        Span::styled(label, Style::default().fg(Color::DarkGray)),
    ]))
}

fn draw_oneshot(frame: &mut Frame, app: &App) {
    let area = frame.area().inner(Margin { horizontal: 1, vertical: 0 });
    let width = area.width;

    let chunks = Layout::vertical([
        Constraint::Length(1), // title
        Constraint::Length(1), // config
        Constraint::Length(1), // separator
        Constraint::Length(1), // status
        Constraint::Length(1), // progress
        Constraint::Length(1), // spacer
        Constraint::Min(0),    // log
        Constraint::Length(1), // spacer
        Constraint::Length(1), // separator
        Constraint::Length(1), // keybindings
    ])
    .split(area);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "🕷 sitescrape",
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        ))),
        chunks[0],
    );
    frame.render_widget(Paragraph::new(header_line(app)), chunks[1]);
    frame.render_widget(Paragraph::new(separator_line(width)), chunks[2]);
    frame.render_widget(Paragraph::new(status_line_widget(app)), chunks[3]);

    if let Some(bar) = progress_bar(app, width) {
        frame.render_widget(Paragraph::new(bar), chunks[4]);
    }

    // chunks[5] is blank spacer
    render_log(frame, app, chunks[6]);
    // chunks[7] is blank spacer

    frame.render_widget(Paragraph::new(separator_line(width)), chunks[8]);

    let footer = Line::from(Span::styled(
        "ctrl+c quit · ↑↓ scroll",
        Style::default().fg(Color::DarkGray),
    ));
    frame.render_widget(Paragraph::new(footer), chunks[9]);
}

fn draw_interactive(frame: &mut Frame, app: &App) {
    let area = frame.area().inner(Margin { horizontal: 1, vertical: 0 });
    let width = area.width;

    let chunks = Layout::vertical([
        Constraint::Length(1), // title
        Constraint::Length(1), // config
        Constraint::Length(1), // separator
        Constraint::Length(1), // status
        Constraint::Length(1), // progress
        Constraint::Length(1), // spacer
        Constraint::Min(0),    // log
        Constraint::Length(1), // spacer
        Constraint::Length(1), // separator
        Constraint::Length(1), // input
        Constraint::Length(1), // keybindings
    ])
    .split(area);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "🕷 sitescrape",
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        ))),
        chunks[0],
    );
    frame.render_widget(Paragraph::new(header_line(app)), chunks[1]);
    frame.render_widget(Paragraph::new(separator_line(width)), chunks[2]);
    frame.render_widget(Paragraph::new(status_line_widget(app)), chunks[3]);

    if let Some(bar) = progress_bar(app, width) {
        frame.render_widget(Paragraph::new(bar), chunks[4]);
    }

    // chunks[5] is blank spacer
    render_log(frame, app, chunks[6]);
    // chunks[7] is blank spacer

    frame.render_widget(Paragraph::new(separator_line(width)), chunks[8]);

    // Input line
    let input_area = chunks[9];
    let input_line = if app.input.is_empty() {
        Line::from(vec![
            Span::styled("❯ ", Style::default().fg(Color::Magenta)),
            Span::styled("Type /help for commands", Style::default().fg(Color::DarkGray)),
        ])
    } else {
        Line::from(vec![
            Span::styled("❯ ", Style::default().fg(Color::Magenta)),
            Span::styled(app.input.clone(), Style::default().fg(Color::White)),
        ])
    };
    frame.render_widget(Paragraph::new(input_line), input_area);

    // Cursor: chevron is 2 chars wide ("❯ "), no border offset needed
    let chevron_width = 2u16;
    frame.set_cursor_position(Position::new(
        input_area.x + chevron_width + app.cursor_pos as u16,
        input_area.y,
    ));

    let keys = match &app.state {
        AppState::Crawling => "esc stop · ↑↓ scroll · ctrl+c quit",
        AppState::Paused => "p resume · esc stop · ↑↓ scroll · ctrl+c quit",
        _ => "enter run · ↑↓ scroll · ctrl+c quit",
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(keys, Style::default().fg(Color::DarkGray)))),
        chunks[10],
    );
}

fn render_log(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let entries = app.visible_log(area.height as usize);
    let crawling = matches!(app.state, AppState::Crawling);
    let spinner = SPINNER[app.spinner_frame % SPINNER.len()];

    let items: Vec<ListItem> = entries
        .iter()
        .map(|e| {
            let line = match &e.status {
                LogStatus::Ok => {
                    let mut spans = vec![
                        Span::styled("✓ ", Style::default().fg(Color::Green)),
                        Span::styled(e.url.clone(), Style::default().fg(Color::White)),
                    ];
                    if let Some(fp) = &e.filepath {
                        spans.push(Span::styled(" → ", Style::default().fg(Color::DarkGray)));
                        spans.push(Span::styled(fp.clone(), Style::default().fg(Color::Blue)));
                    }
                    Line::from(spans)
                }
                LogStatus::Error => {
                    let mut spans = vec![
                        Span::styled("✗ ", Style::default().fg(Color::Red)),
                        Span::styled(e.url.clone(), Style::default().fg(Color::White)),
                    ];
                    if let Some(msg) = &e.message {
                        spans.push(Span::styled(format!(" ({})", msg), Style::default().fg(Color::DarkGray)));
                    }
                    Line::from(spans)
                }
                LogStatus::AuthFail(code) => {
                    let mut spans = vec![
                        Span::styled("⚠ ", Style::default().fg(Color::Yellow)),
                        Span::styled(e.url.clone(), Style::default().fg(Color::White)),
                        Span::styled(format!(" ({} auth failed)", code), Style::default().fg(Color::DarkGray)),
                    ];
                    if let Some(msg) = &e.message {
                        spans.push(Span::styled(format!(" {}", msg), Style::default().fg(Color::DarkGray)));
                    }
                    Line::from(spans)
                }
                LogStatus::Visiting => {
                    let icon = if crawling { format!("{} ", spinner) } else { "◌ ".to_string() };
                    Line::from(vec![
                        Span::styled(icon, Style::default().fg(Color::Blue)),
                        Span::styled(e.url.clone(), Style::default().fg(Color::White)),
                    ])
                }
                LogStatus::Info(msg) => Line::from(vec![
                    Span::styled("ℹ ", Style::default().fg(Color::Cyan)),
                    Span::styled(msg.clone(), Style::default().fg(Color::Cyan)),
                ]),
                LogStatus::Banner(msg) => Line::from(vec![
                    Span::styled(msg.clone(), Style::default().fg(Color::Magenta)),
                ]),
            };
            ListItem::new(line)
        })
        .collect();

    frame.render_widget(List::new(items), area);
}
