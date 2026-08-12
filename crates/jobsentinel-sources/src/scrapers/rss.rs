//! RSS item parsing helpers for source adapters.

use quick_xml::{escape::unescape, events::Event, Reader};
use std::{collections::BTreeMap, error::Error};

pub(super) type RssParseError = Box<dyn Error + Send + Sync>;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct RssItem {
    fields: BTreeMap<String, String>,
}

impl RssItem {
    pub(super) fn get(&self, tag: &str) -> Option<&str> {
        let key = normalize_name(tag);
        self.fields
            .get(&key)
            .or_else(|| self.fields.get(local_name(&key)))
            .map(String::as_str)
            .filter(|value| !value.trim().is_empty())
    }

    fn insert_once(&mut self, tag: &str, value: &str) {
        let value = value.trim();
        if value.is_empty() {
            return;
        }

        let key = normalize_name(tag);
        self.fields
            .entry(key.clone())
            .or_insert_with(|| value.to_string());

        let local = local_name(&key);
        if local != key {
            self.fields
                .entry(local.to_string())
                .or_insert_with(|| value.to_string());
        }
    }

    fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}

pub(super) fn parse_rss_items(xml: &str, limit: usize) -> Result<Vec<RssItem>, RssParseError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut buf = Vec::new();
    let mut items = Vec::with_capacity(limit.min(32));
    let mut current_item = RssItem::default();
    let mut in_item = false;
    let mut active_field: Option<String> = None;
    let mut active_text = String::new();
    let mut element_depth = 0_usize;

    loop {
        let event = reader.read_event_into(&mut buf)?;
        match &event {
            Event::Start(_) => {
                element_depth = element_depth.checked_add(1).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "RSS element depth overflowed",
                    )
                })?;
            }
            Event::End(_) => {
                element_depth = element_depth.checked_sub(1).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "RSS closed an unopened element",
                    )
                })?;
            }
            Event::Eof if element_depth != 0 => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "RSS ended before all elements closed",
                )
                .into());
            }
            _ => {}
        }

        match event {
            Event::Start(element) if is_local_name(element.name().as_ref(), b"item") => {
                in_item = true;
                current_item = RssItem::default();
                active_field = None;
                active_text.clear();
            }
            Event::Start(element) if in_item && active_field.is_none() => {
                active_field = Some(name_from_bytes(element.name().as_ref()));
                active_text.clear();
            }
            Event::End(element) if in_item && is_local_name(element.name().as_ref(), b"item") => {
                if let Some(field) = active_field.take() {
                    current_item.insert_once(&field, &active_text);
                    active_text.clear();
                }
                if !current_item.is_empty() && items.len() < limit {
                    items.push(current_item.clone());
                }
                in_item = false;
            }
            Event::End(element) if in_item => {
                if let Some(field) = active_field.as_deref() {
                    if name_matches(element.name().as_ref(), field) {
                        current_item.insert_once(field, &active_text);
                        active_field = None;
                        active_text.clear();
                    }
                }
            }
            Event::Text(text) if in_item && active_field.is_some() => {
                let decoded = text.decode()?;
                let unescaped = unescape(decoded.as_ref())?;
                active_text.push_str(&unescaped);
            }
            Event::CData(text) if in_item && active_field.is_some() => {
                let decoded = text.decode()?;
                active_text.push_str(decoded.as_ref());
            }
            Event::GeneralRef(reference) if in_item && active_field.is_some() => {
                let decoded = reference.decode()?;
                let entity = format!("&{};", decoded);
                let unescaped = unescape(&entity)?;
                active_text.push_str(&unescaped);
            }
            Event::Eof => break,
            _ => {}
        }

        buf.clear();
    }

    Ok(items)
}

#[cfg(test)]
pub(super) fn extract_xml_tag(fragment: &str, tag: &str) -> Option<String> {
    parse_first_tag(fragment, tag).ok().flatten()
}

#[cfg(test)]
fn parse_first_tag(fragment: &str, tag: &str) -> Result<Option<String>, RssParseError> {
    let wrapped = format!("<root>{fragment}</root>");
    let mut reader = Reader::from_str(&wrapped);
    reader.config_mut().trim_text(false);

    let mut buf = Vec::new();
    let mut in_tag = false;
    let target = normalize_name(tag);
    let mut text_content = String::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(element) if !in_tag && name_matches(element.name().as_ref(), &target) => {
                in_tag = true;
                text_content.clear();
            }
            Event::End(element) if in_tag && name_matches(element.name().as_ref(), &target) => {
                let value = text_content.trim();
                return Ok((!value.is_empty()).then(|| value.to_string()));
            }
            Event::Text(text) if in_tag => {
                let decoded = text.decode()?;
                let unescaped = unescape(decoded.as_ref())?;
                text_content.push_str(&unescaped);
            }
            Event::CData(text) if in_tag => {
                let decoded = text.decode()?;
                text_content.push_str(decoded.as_ref());
            }
            Event::GeneralRef(reference) if in_tag => {
                let decoded = reference.decode()?;
                let entity = format!("&{};", decoded);
                let unescaped = unescape(&entity)?;
                text_content.push_str(&unescaped);
            }
            Event::Eof => break,
            _ => {}
        }

        buf.clear();
    }

    Ok(None)
}

fn name_matches(actual: &[u8], expected: &str) -> bool {
    normalize_name(&name_from_bytes(actual)) == expected
        || local_name(&name_from_bytes(actual)) == local_name(expected)
}

fn is_local_name(actual: &[u8], expected: &[u8]) -> bool {
    actual.rsplit(|byte| *byte == b':').next().unwrap_or(actual) == expected
}

fn name_from_bytes(name: &[u8]) -> String {
    String::from_utf8_lossy(name).into_owned()
}

fn normalize_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

fn local_name(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rss_items_accepts_item_attributes_and_cdata() {
        let xml = r#"
            <rss>
                <channel>
                    <item rdf:about="https://example.com/job/1">
                        <title><![CDATA[Company: Role]]></title>
                        <link>https://example.com/job/1</link>
                    </item>
                </channel>
            </rss>
        "#;

        let items = parse_rss_items(xml, 10).expect("rss items");

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].get("title"), Some("Company: Role"));
        assert_eq!(items[0].get("link"), Some("https://example.com/job/1"));
    }

    #[test]
    fn parse_rss_items_keeps_namespaced_tags_available_by_full_or_local_name() {
        let xml = r#"
            <rss>
                <channel>
                    <item>
                        <georss:point>39.7392 -104.9903</georss:point>
                    </item>
                </channel>
            </rss>
        "#;

        let items = parse_rss_items(xml, 10).expect("rss items");

        assert_eq!(items[0].get("georss:point"), Some("39.7392 -104.9903"));
        assert_eq!(items[0].get("point"), Some("39.7392 -104.9903"));
    }

    #[test]
    fn parse_rss_items_rejects_a_truncated_tail_after_the_result_limit() {
        let xml = r#"
            <rss>
                <channel>
                    <item>
                        <title>Company: Complete Role</title>
                        <link>https://example.com/job/1</link>
                    </item>
                    <item><title>Truncated Role
        "#;

        assert!(parse_rss_items(xml, 1).is_err());
    }
}
