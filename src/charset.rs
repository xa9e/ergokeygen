use crate::{Key, Layout};

pub(crate) fn sanitize_charset_as_keys(layout: &Layout, raw: &[char]) -> Vec<Key> {
    let mut out = Vec::new();
    for &ch in raw {
        if let Some(key) = layout.key(ch) {
            if !out.iter().any(|existing: &Key| existing.typed == key.typed) {
                out.push(key);
            }
        }
    }
    out
}

pub fn charset_by_name(name_or_chars: &str) -> Result<Vec<char>, String> {
    let raw = match name_or_chars {
        "lower" | "alpha" => "abcdefghijklmnopqrstuvwxyz".to_string(),
        "upper" => "ABCDEFGHIJKLMNOPQRSTUVWXYZ".to_string(),
        "letters" => "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ".to_string(),
        "digits" | "num" => "0123456789".to_string(),
        "lowerdigits" | "alnum" => "abcdefghijklmnopqrstuvwxyz0123456789".to_string(),
        "symbols" => "`-=[]\\;',./~!@#$%^&*()_+{}|:\"<>?".to_string(),
        "full" => "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789`-=[]\\;',./~!@#$%^&*()_+{}|:\"<>?".to_string(),
        custom if !custom.is_empty() => custom.to_string(),
        _ => return Err("charset cannot be empty".to_string()),
    };

    let layout = Layout::ansi_qwerty();
    let mut out = Vec::new();
    for ch in raw.chars() {
        if layout.contains(ch) && !out.contains(&ch) {
            out.push(ch);
        }
    }
    if out.is_empty() {
        return Err("charset has no supported chars".to_string());
    }
    Ok(out)
}
