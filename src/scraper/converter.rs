use color_eyre::Result;

pub struct ConvertedPage {
    pub markdown: String,
    pub title: String,
}

fn extract_title(html: &str) -> String {
    for (open, close) in [("<title", "</title>"), ("<h1", "</h1>")] {
        if let Some(start) = html.find(open) {
            if let Some(gt) = html[start..].find('>') {
                let cs = start + gt + 1;
                if let Some(end) = html[cs..].find(close) {
                    let t = html[cs..cs + end].trim();
                    if !t.is_empty() {
                        return t.to_string();
                    }
                }
            }
        }
    }
    String::new()
}

/// Extract innerHTML of the first element matching a simple tag/attribute pattern.
fn extract_content<'a>(html: &'a str, selector: &str) -> Option<&'a str> {
    // Map selector to a search string we can find in raw HTML
    let search = if selector.starts_with('<') {
        selector.to_string()
    } else if selector.starts_with('[') {
        // e.g. [role="main"] -> role="main"
        selector.trim_matches(|c| c == '[' || c == ']').to_string()
    } else if let Some(stripped) = selector.strip_prefix('.') {
        // e.g. .content -> class=" followed by word boundary check
        format!("class=\"{}", stripped)
    } else {
        format!("<{}", selector)
    };

    let lower = html.to_lowercase();
    let search_lower = search.to_lowercase();
    let match_pos = lower.find(&search_lower)?;

    // Walk backwards from match_pos to find the opening '<' of this tag
    let tag_open = lower[..match_pos].rfind('<')?;

    // Find the closing > of the opening tag
    let gt = html[tag_open..].find('>')?;
    let content_start = tag_open + gt + 1;

    // Extract the tag name (between '<' and first whitespace or '>')
    let tag_name: &str = html[tag_open + 1..]
        .split(|c: char| c.is_whitespace() || c == '>')
        .next()?;
    let close_tag = format!("</{}", tag_name.to_lowercase());

    // Find matching close tag (simple: first occurrence after content_start)
    let content_lower = &lower[content_start..];
    let end = content_lower.find(&close_tag)?;

    Some(&html[content_start..content_start + end])
}

pub fn html_to_markdown(html: &str, selector: Option<&str>) -> Result<ConvertedPage> {
    let title = extract_title(html);

    // Priority: custom selector > <main> > <article> > [role="main"] > .content > <body>
    let content = if let Some(sel) = selector {
        extract_content(html, sel).unwrap_or(html)
    } else {
        extract_content(html, "<main")
            .or_else(|| extract_content(html, "<article"))
            .or_else(|| extract_content(html, "[role=\"main\"]"))
            .or_else(|| extract_content(html, ".content"))
            .or_else(|| extract_content(html, "<body"))
            .unwrap_or(html)
    };

    let converter = htmd::HtmlToMarkdown::builder()
        .skip_tags(vec!["script", "style", "noscript", "svg"])
        .build();

    let markdown = converter
        .convert(content)
        .map_err(|e| color_eyre::eyre::eyre!(e))?;

    Ok(ConvertedPage { markdown, title })
}
