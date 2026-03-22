use pulldown_cmark::{html, CowStr, Event, LinkType, Parser, Tag, TagEnd};
use regex::Regex;
use std::sync::OnceLock;

static MENTION_REGEX: OnceLock<Regex> = OnceLock::new();
pub fn render_markdown_with_mentions(markdown_input: &str) -> (String, Vec<String>) {
    let mut users: Vec<String> = Vec::new();
    let mut in_link = false;
    let transformer = Parser::new(markdown_input).flat_map(|event| -> Vec<Event> {
        if let Event::Start(Tag::Link { .. }) = event {
            in_link = true;
            return vec![event];
        }
        if let Event::End(TagEnd::Link { .. }) = event {
            in_link = false;
            return vec![event];
        }
        if in_link {
            return vec![event];
        }
        if let Event::Text(text) = event {
            return at_user(&mut users, text);
        }
        return vec![event];
    });
    let mut html_output = String::new();
    html::push_html(&mut html_output, transformer);
    (html_output, users)
}

fn at_user<'a>(users: &mut Vec<String>, text: CowStr<'a>) -> Vec<Event<'a>> {
    let r = MENTION_REGEX.get_or_init(|| Regex::new(r"\B@(\w+)").unwrap());
    let text_str = text.as_ref();
    if !r.is_match(text_str) {
        return vec![Event::Text(text)];
    }

    // 如果没有匹配到 @，快速返回，避免不必要的 Allocation
    if !r.is_match(text_str) {
        return vec![Event::Text(text)];
    }

    // 开始分割文本并插入链接事件
    let mut new_events = Vec::new();
    let mut last_match_end = 0;

    for cap in r.captures_iter(text_str) {
        let Some(match_whole) = cap.get(0) else { continue; }; // @User
        let Some(username) = cap.get(1).map(|m| m.as_str()) else { continue; }; // User
        let start = match_whole.start();
        let end = match_whole.end();
        users.push(username.to_string());

        // 3.1 插入 @ 之前的普通文本
        if start > last_match_end {
            let prev_text = &text_str[last_match_end..start];
            new_events.push(Event::Text(CowStr::from(prev_text.to_string())));
        }

        // 3.2 插入链接开始
        let link_url = format!("/u/{}", username);
        new_events.push(Event::Start(Tag::Link {
            link_type: LinkType::Inline,
            dest_url: CowStr::from(link_url),
            title: CowStr::from(""),
            id: CowStr::from(""),
        }));

        // 3.3 插入链接显示的文本
        new_events.push(Event::Text(CowStr::from(match_whole.as_str().to_string())));

        // 3.4 插入链接结束
        new_events.push(Event::End(TagEnd::Link));

        last_match_end = end;
    }

    // 3.5 插入剩余的文本
    if last_match_end < text_str.len() {
        let remaining_text = &text_str[last_match_end..];
        new_events.push(Event::Text(CowStr::from(remaining_text.to_string())));
    }

    new_events
}
