pub enum Command {
    Scrape {
        url: String,
        max_pages: Option<usize>,
        selector: Option<String>,
        prefix: Option<String>,
        delay: Option<u64>,
        no_browser: bool,
    },
    Stop,
    Status,
    Config,
    Set { key: String, value: String },
    Cookies,
    Clear,
    Help,
    Quit,
    Unknown(String),
}

pub fn parse_command(input: &str) -> Command {
    let input = input.trim();
    if input.is_empty() {
        return Command::Unknown(String::new());
    }
    if !input.starts_with('/') {
        return Command::Unknown("Unknown command. Type /help for commands.".into());
    }

    let tokens: Vec<&str> = input.split_whitespace().collect();
    let cmd = tokens[0];
    let args = &tokens[1..];

    match cmd {
        "/stop" => Command::Stop,
        "/status" => Command::Status,
        "/config" => Command::Config,
        "/cookies" => Command::Cookies,
        "/clear" => Command::Clear,
        "/help" => Command::Help,
        "/quit" | "/q" => Command::Quit,
        "/set" => {
            if args.len() < 2 {
                Command::Unknown("/set requires <key> <value>".into())
            } else {
                Command::Set {
                    key: args[0].to_string(),
                    value: args[1..].join(" "),
                }
            }
        }
        "/scrape" => parse_scrape(args),
        _ => Command::Unknown(format!("Unknown command: {}", cmd)),
    }
}

fn parse_scrape(args: &[&str]) -> Command {
    let url = match args.iter().find(|a| !a.starts_with('-')) {
        Some(u) => u.to_string(),
        None => return Command::Unknown("/scrape requires a URL".into()),
    };

    let mut max_pages: Option<usize> = None;
    let mut selector: Option<String> = None;
    let mut prefix: Option<String> = None;
    let mut delay: Option<u64> = None;
    let mut no_browser = false;

    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "--no-browser" => no_browser = true,
            "-m" | "--max-pages" => {
                if i + 1 < args.len() {
                    max_pages = args[i + 1].parse().ok();
                    i += 1;
                }
            }
            "-s" | "--selector" => {
                if i + 1 < args.len() {
                    selector = Some(args[i + 1].to_string());
                    i += 1;
                }
            }
            "-p" | "--prefix" => {
                if i + 1 < args.len() {
                    prefix = Some(args[i + 1].to_string());
                    i += 1;
                }
            }
            "-d" | "--delay" => {
                if i + 1 < args.len() {
                    delay = args[i + 1].parse().ok();
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    Command::Scrape { url, max_pages, selector, prefix, delay, no_browser }
}
