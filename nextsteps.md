# Next Steps — SiteScrape

## Current State (2026-04-13)

### What's done
- **chromiumoxide migration complete** — `headless_chrome` replaced, Chrome 146 launches and executes JS successfully
- **TUI works** — `/help`, `/scrape`, scroll, stop all functional. Auto-scroll bug fixed.
- **Browser detection fixed** — plist parser handles nested dicts correctly
- **HTTP mode works** — `--no-browser` flag for static sites
- **Browser mode is the default** — chromiumoxide launches Chrome, JS executes (verified: `document.title` is set by the SPA)
- **Spec complete** — `.kiro/specs/2026-04-13-chromiumoxide-migration/` — all tasks [x], review PASS, security review PASS

### What's broken
The target site (`ep.federate.a2z.com/help`) returns an empty body after markdown conversion, even though Chrome IS launching and JS IS executing (the page title is correctly set to "Amazon Federate Enrollment Portal").

## Debugging needed

### Root cause candidates (in order of likelihood)

1. **Auth cookies not taking effect** — Cookies are loaded from Firefox and set via `browser.set_cookies()` before navigation. But the SPA may check auth on an API call (XHR/fetch) that happens after page load. If the auth cookie isn't being sent with those API requests, the app renders an empty/logged-out state silently. 
   - **Debug step**: Dump the raw HTML from `page.content()` to a file before markdown conversion. Check if the HTML has actual content or just an empty `<div id="root"></div>`.
   - **Debug step**: Check if the page redirects to a login URL. Log `page.url()` after the wait loop.

2. **`html_to_markdown` stripping all content** — The converter's `extract_content()` function looks for `<main>`, `<article>`, `[role="main"]`, `.content`, `<body>` in that order. If the SPA renders into a `<div id="root">` that doesn't match any of these selectors, the content extraction returns the full HTML, but the markdown converter's `skip_tags` list (`nav`, `header`, `footer`, `aside`, `script`, `style`) might be stripping everything.
   - **Debug step**: Save the raw HTML to `/tmp/debug.html` and inspect it manually.

3. **Wait loop exiting too early** — The condition checks `innerText.trim().length > 50`. If the SPA renders but the visible text is short (e.g., just a nav bar or error message), the loop times out after 15s and proceeds with whatever partial HTML is there.
   - **Debug step**: Log the `innerText.length` value at each poll iteration.

### Recommended fix approach

Add a `--debug` flag that dumps raw HTML to `<output_dir>/_debug/<filename>.html` alongside the markdown files. This makes it easy to see exactly what Chrome is rendering without modifying the core code each time.

Then inspect the HTML to determine which of the 3 candidates is the actual cause.

## Pending specs

- **TUI redesign** — `.kiro/specs/2026-04-13-tui-redesign/` — spec and tasks written but not started. Makes the TUI colorful with Unicode icons, braille spinner, magenta accent colors (Claude Code / Kiro CLI style).

## Uncommitted changes

Everything is uncommitted (21 untracked files, 1 modified). Should commit the working state before continuing:

```bash
git add -A
git commit -m "feat: replace headless_chrome with chromiumoxide, fix TUI bugs

- Migrate from headless_chrome (broken with Chrome 146+) to chromiumoxide v0.8.0
- Make Navigate trait, crawl(), and TUI async (tokio)
- Browser mode is default, --no-browser for HTTP-only
- Fix auto-scroll bug (only last log line visible)
- Fix browser detection (nested plist dict parsing)
- Fix extract_content selector matching (walk back to '<' for tag name)
- Add Engine Drop impl (abort handler task)
- Wrap crossterm poll in spawn_blocking for async safety
"
```
