use crate::paths::extension;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    pub path: String,
    pub line: usize,
    pub inline: bool,
    pub src: Option<String>,
    pub source: String,
}

pub fn scripts_in(path: &str, text: &str) -> Vec<Script> {
    match extension(path) {
        Some("js") => vec![Script {
            path: path.to_string(),
            line: 1,
            inline: false,
            src: None,
            source: text.to_string(),
        }],
        Some("html") | Some("svg") => script_elements(path, text, 1),
        Some("md") => html_blocks(text)
            .into_iter()
            .flat_map(|block| script_elements(path, &block.body, block.line))
            .collect(),
        _ => Vec::new(),
    }
}

pub(crate) struct HtmlBlock {
    pub line: usize,
    pub body: String,
}

pub(crate) fn html_blocks(markdown: &str) -> Vec<HtmlBlock> {
    let mut blocks = Vec::new();
    let mut open: Option<(Fence, HtmlBlock)> = None;
    for (index, line) in markdown.split('\n').enumerate() {
        let fence = fence_of(line);
        match (&mut open, fence) {
            (Some((opening, _)), Some(closing)) if closing.closes(opening) => {
                if let Some((opening, block)) = open.take() {
                    if opening.is_html {
                        blocks.push(block);
                    }
                }
            }
            (Some((_, block)), _) => {
                block.body.push_str(line);
                block.body.push('\n');
            }
            (None, Some(opening)) => {
                open = Some((
                    opening,
                    HtmlBlock {
                        line: index + 2,
                        body: String::new(),
                    },
                ));
            }
            (None, None) => {}
        }
    }
    if let Some((opening, block)) = open {
        if opening.is_html {
            blocks.push(block);
        }
    }
    blocks
}

struct Fence {
    marker: char,
    length: usize,
    is_html: bool,
    has_info: bool,
}

impl Fence {
    fn closes(&self, opening: &Fence) -> bool {
        self.marker == opening.marker && self.length >= opening.length && !self.has_info
    }
}

fn fence_of(line: &str) -> Option<Fence> {
    let content = line.trim_start_matches(|c: char| c.is_whitespace() || c == '>');
    let marker = content.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let length = content.chars().take_while(|c| *c == marker).count();
    if length < 3 {
        return None;
    }
    let info = content[length..].trim();
    if marker == '`' && info.contains('`') {
        return None;
    }
    let language = info
        .split(|c: char| c.is_whitespace() || c == ',' || c == '{' || c == '}')
        .find(|word| !word.is_empty())
        .unwrap_or("")
        .trim_start_matches('.');
    Some(Fence {
        marker,
        length,
        is_html: language.eq_ignore_ascii_case("html"),
        has_info: !info.is_empty(),
    })
}

fn script_elements(path: &str, html: &str, first_line: usize) -> Vec<Script> {
    let lowered = html.to_ascii_lowercase();
    let mut scripts = Vec::new();
    let mut cursor = 0;
    while let Some(found) = lowered[cursor..].find("<script") {
        let tag_start = cursor + found;
        let after_name = tag_start + "<script".len();
        let is_tag = lowered[after_name..]
            .chars()
            .next()
            .is_some_and(|c| c.is_whitespace() || c == '>' || c == '/');
        if !is_tag {
            cursor = after_name;
            continue;
        }
        let Some(tag_end) = end_of_tag(&lowered, after_name) else {
            break;
        };
        let body_start = tag_end + 1;
        let body_end = lowered[body_start..]
            .find("</script")
            .map_or(html.len(), |offset| body_start + offset);
        scripts.push(Script {
            path: path.to_string(),
            line: first_line + html[..body_start].matches('\n').count(),
            inline: true,
            src: attribute_value(&html[after_name..tag_end], "src"),
            source: html[body_start..body_end].to_string(),
        });
        cursor = body_end;
    }
    scripts
}

fn end_of_tag(text: &str, from: usize) -> Option<usize> {
    let mut quote: Option<char> = None;
    for (offset, c) in text[from..].char_indices() {
        match quote {
            Some(open) if c == open => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == '>' => return Some(from + offset),
            None => {}
        }
    }
    None
}

pub(crate) fn attribute_values(text: &str, name: &str) -> Vec<String> {
    let lowered = text.to_ascii_lowercase();
    let mut values = Vec::new();
    let mut cursor = 0;
    while let Some(found) = lowered[cursor..].find(name) {
        let start = cursor + found;
        let after = start + name.len();
        cursor = after;
        let preceded_by_boundary = text[..start]
            .chars()
            .next_back()
            .is_none_or(|c| c.is_whitespace() || c == '"' || c == '\'' || c == '<' || c == '/');
        if !preceded_by_boundary {
            continue;
        }
        let rest = text[after..].trim_start();
        let Some(rest) = rest.strip_prefix('=') else {
            continue;
        };
        let rest = rest.trim_start();
        let value = match rest.chars().next() {
            Some(quote @ ('"' | '\'')) => rest[1..].split(quote).next().unwrap_or(""),
            Some(_) => rest
                .split(|c: char| c.is_whitespace() || c == '>')
                .next()
                .unwrap_or(""),
            None => "",
        };
        values.push(decode_entities(value));
    }
    values
}

fn attribute_value(tag: &str, name: &str) -> Option<String> {
    attribute_values(tag, name).into_iter().next()
}

fn decode_entities(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&#34;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

pub(crate) fn long_lines(script: &Script, limit: usize) -> Vec<(usize, usize)> {
    script
        .source
        .split('\n')
        .enumerate()
        .flat_map(|(index, line)| {
            line.split('\r')
                .map(move |piece| (script.line + index, piece.chars().count()))
        })
        .filter(|(_, length)| *length > limit)
        .collect()
}
