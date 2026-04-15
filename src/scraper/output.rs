use color_eyre::Result;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

fn hash8(s: &str) -> String {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    format!("{:08x}", h.finish() & 0xFFFFFFFF)
}

fn segment_to_name(s: &str) -> String {
    if s.is_empty() { return "Index".into(); }
    let s = s.trim_end_matches(".html").trim_end_matches(".htm")
        .trim_end_matches(".php").trim_end_matches(".aspx").trim_end_matches(".jsp");
    if s.is_empty() { return "Index".into(); }
    if !s.contains('-') && s.chars().any(|c| c.is_uppercase()) { return s.into(); }
    s.split('-').map(|w| {
        let mut c = w.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    }).collect::<Vec<_>>().join("-")
}

pub fn url_to_filepath(url: &str, base_url: &str) -> String {
    let (u, b) = match (url::Url::parse(url), url::Url::parse(base_url)) {
        (Ok(u), Ok(b)) => (u, b),
        _ => return "Index.md".into(),
    };
    let mut path = u.path().to_string();
    let base_path = b.path().trim_end_matches('/');
    if !base_path.is_empty() && path.starts_with(base_path) {
        path = path[base_path.len()..].to_string();
    }
    let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty() && *s != "." && *s != "..").collect();
    let filename = if segs.is_empty() { "Index".into() } else { segment_to_name(segs.last().unwrap()) };
    let dirs: Vec<String> = if segs.len() > 1 { segs[..segs.len()-1].iter().map(|s| segment_to_name(s)).collect() } else { vec![] };
    let suffix = u.query().map(|q| format!("_{}", hash8(q))).unwrap_or_default();
    let name = format!("{}{}.md", filename, suffix);
    if dirs.is_empty() { name } else { format!("{}/{}", dirs.join("/"), name) }
}

pub fn write_page(output_dir: &str, filepath: &str, title: &str, url: &str, markdown: &str) -> Result<PathBuf> {
    let path = PathBuf::from(output_dir).join(filepath);
    // Reject path traversal
    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err(color_eyre::eyre::eyre!("Path traversal detected: {}", filepath));
        }
    }
    if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
    let content = format!("---\ntitle: \"{}\"\nurl: \"{}\"\n---\n\n{}", title.replace('"', "\\\""), url, markdown);
    std::fs::write(&path, content)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(path)
}
