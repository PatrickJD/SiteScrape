pub enum AppState {
    Idle,
    Crawling,
    Paused,
    Done,
    #[allow(dead_code)]
    Error(String),
}

pub enum LogStatus {
    Ok,
    Error,
    AuthFail(u16),
    Visiting,
    Info(String),
    Banner(String),
}

pub struct LogEntry {
    pub status: LogStatus,
    pub url: String,
    pub filepath: Option<String>,
    pub message: Option<String>,
}

pub struct App {
    pub state: AppState,
    pub url: String,
    pub browser_name: String,
    pub cookie_count: usize,
    pub log: Vec<LogEntry>,
    pub progress: (usize, usize),
    pub scroll_offset: usize,
    pub output_dir: String,
    pub delay_ms: u64,
    pub max_pages: usize,
    pub saved: usize,
    pub failed: usize,
    pub auth_failed: usize,
    pub input: String,
    pub cursor_pos: usize,
    pub interactive: bool,
    pub selector: Option<String>,
    pub spinner_frame: usize,
}

impl App {
    pub fn new(
        url: String,
        browser_name: String,
        cookie_count: usize,
        output_dir: String,
        delay_ms: u64,
        max_pages: usize,
    ) -> Self {
        Self {
            state: AppState::Crawling,
            url,
            browser_name,
            cookie_count,
            log: Vec::new(),
            progress: (0, 0),
            scroll_offset: 0,
            output_dir,
            delay_ms,
            max_pages,
            saved: 0,
            failed: 0,
            auth_failed: 0,
            input: String::new(),
            cursor_pos: 0,
            interactive: false,
            selector: None,
            spinner_frame: 0,
        }
    }

    pub fn new_interactive(
        browser_name: String,
        cookie_count: usize,
        output_dir: String,
        delay_ms: u64,
        max_pages: usize,
    ) -> Self {
        Self {
            state: AppState::Idle,
            url: String::new(),
            browser_name,
            cookie_count,
            log: Vec::new(),
            progress: (0, 0),
            scroll_offset: 0,
            output_dir,
            delay_ms,
            max_pages,
            saved: 0,
            failed: 0,
            auth_failed: 0,
            input: String::new(),
            cursor_pos: 0,
            interactive: true,
            selector: None,
            spinner_frame: 0,
        }
    }

    pub fn push_log(&mut self, entry: LogEntry) {
        let at_bottom =
            self.log.is_empty() || self.scroll_offset >= self.log.len().saturating_sub(1);
        self.log.push(entry);
        if self.log.len() > 10_000 {
            let excess = self.log.len() - 10_000;
            self.log.drain(0..excess);
            self.scroll_offset = self.scroll_offset.saturating_sub(excess);
        }
        if at_bottom {
            // Keep showing the bottom: offset such that the last entry is visible.
            // The actual visible window is [scroll_offset .. scroll_offset + height],
            // but we don't know height here. Setting to len() means visible_log()
            // will clamp it via: start = min(scroll_offset, len-1), showing the tail.
            self.scroll_offset = self.log.len();
        }
    }

    pub fn push_info(&mut self, msg: String) {
        self.push_log(LogEntry {
            status: LogStatus::Info(msg.clone()),
            url: String::new(),
            filepath: None,
            message: Some(msg),
        });
    }

    pub fn push_banner(&mut self, msg: String) {
        self.push_log(LogEntry {
            status: LogStatus::Banner(msg.clone()),
            url: String::new(),
            filepath: None,
            message: Some(msg),
        });
    }

    pub fn set_progress(&mut self, current: usize, total: usize) {
        self.progress = (current, total);
    }

    pub fn toggle_pause(&mut self) {
        self.state = match self.state {
            AppState::Crawling => AppState::Paused,
            AppState::Paused => AppState::Crawling,
            _ => return,
        };
    }

    pub fn set_done(&mut self, saved: usize, failed: usize, auth_failed: usize) {
        self.saved = saved;
        self.failed = failed;
        self.auth_failed = auth_failed;
        self.state = AppState::Done;
    }

    #[allow(dead_code)]
    pub fn set_error(&mut self, msg: String) {
        self.state = AppState::Error(msg);
    }

    pub fn reset_for_scrape(&mut self, url: String) {
        self.url = url;
        self.state = AppState::Crawling;
        self.progress = (0, 0);
        self.saved = 0;
        self.failed = 0;
        self.auth_failed = 0;
    }

    pub fn back_to_idle(&mut self) {
        self.state = AppState::Idle;
    }

    pub fn input_char(&mut self, c: char) {
        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    pub fn input_backspace(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        // Find the char boundary before cursor_pos
        let mut pos = self.cursor_pos - 1;
        while !self.input.is_char_boundary(pos) {
            pos -= 1;
        }
        self.input.remove(pos);
        self.cursor_pos = pos;
    }

    pub fn input_delete(&mut self) {
        if self.cursor_pos >= self.input.len() {
            return;
        }
        self.input.remove(self.cursor_pos);
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        let mut pos = self.cursor_pos - 1;
        while !self.input.is_char_boundary(pos) {
            pos -= 1;
        }
        self.cursor_pos = pos;
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_pos >= self.input.len() {
            return;
        }
        let mut pos = self.cursor_pos + 1;
        while !self.input.is_char_boundary(pos) {
            pos += 1;
        }
        self.cursor_pos = pos;
    }

    pub fn submit_input(&mut self) -> String {
        let val = self.input.clone();
        self.input.clear();
        self.cursor_pos = 0;
        val
    }

    pub fn clear_input(&mut self) {
        self.input.clear();
        self.cursor_pos = 0;
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        if self.scroll_offset < self.log.len() {
            self.scroll_offset += 1;
        }
    }

    pub fn visible_log(&self, height: usize) -> &[LogEntry] {
        if self.log.is_empty() || height == 0 {
            return &[];
        }
        // scroll_offset >= log.len() means "show the tail"
        let start = if self.scroll_offset >= self.log.len() {
            self.log.len().saturating_sub(height)
        } else {
            self.scroll_offset
        };
        let end = (start + height).min(self.log.len());
        &self.log[start..end]
    }
}
