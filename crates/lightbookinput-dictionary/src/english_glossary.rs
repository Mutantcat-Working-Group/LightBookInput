use std::path::Path;

use crate::error::DictionaryError;

/// 程序员模式的中→英释义表：中文候选词 → 英文释义。
///
/// 文件格式 TSV：`词\t[词性. ]释义[\t释义…]`，`#` 开头是注释，空行忽略。
/// 词性形如 `v.` / `adj.` / `n.`，跟着一个空格，取释义时剥掉。
/// 一行缺词或缺释义都算坏行：这张表是人工校过的，缺字段说明生成时出了问题。
#[derive(Debug, Default)]
pub struct EnglishGlossary {
    /// (中文词, 释义)，按中文词排好，二分定位。释义最多留三条，用 `; ` 连起来。
    entries: Vec<(String, String)>,
}

/// 一条最多留几个释义，再多对候选窗里那一小行没意义。
const MAX_SENSES: usize = 3;

impl EnglishGlossary {
    /// 读 TSV 并排序；路径不存在返回 [`DictionaryError::Io`]。
    pub fn from_path(path: &Path) -> Result<Self, DictionaryError> {
        let source = std::fs::read_to_string(path)?;
        Self::parse(&source)
    }

    /// 解析 TSV 文本。
    pub fn parse(source: &str) -> Result<Self, DictionaryError> {
        let mut entries: Vec<(String, String)> = Vec::new();
        for (index, raw) in source.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            let word = fields
                .next()
                .filter(|s| !s.is_empty())
                .ok_or(DictionaryError::Line {
                    line: index + 1,
                    reason: "missing word",
                })?;
            let gloss = fields
                .next()
                .map(|sense| strip_part_of_speech(sense).trim().to_owned())
                .filter(|s| !s.is_empty())
                .ok_or(DictionaryError::Line {
                    line: index + 1,
                    reason: "missing gloss",
                })?;
            let mut kept = vec![gloss];
            for sense in fields.take(MAX_SENSES - 1) {
                let sense = strip_part_of_speech(sense).trim();
                if !sense.is_empty() && !kept.iter().any(|have| have == sense) {
                    kept.push(sense.to_owned());
                }
            }
            entries.push((word.to_owned(), kept.join("; ")));
        }
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        entries.dedup_by(|a, b| a.0 == b.0);
        tracing::debug!(words = entries.len(), "中英释义表加载完成");
        Ok(Self { entries })
    }

    /// 查一个中文词的英文释义；没有这个词或没装表返回 `None`。
    pub fn gloss_of(&self, word: &str) -> Option<&str> {
        self.entries
            .binary_search_by(|entry| entry.0.as_str().cmp(word))
            .ok()
            .map(|index| self.entries[index].1.as_str())
    }

    /// 表里有多少条。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 表是不是空的（没装或文件里没有条目）。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// 剥掉释义前面的词性（`v. ` / `adj. ` / `n. `）：点前是字母，点后是空格或有结尾。
fn strip_part_of_speech(sense: &str) -> &str {
    let trimmed = sense.trim();
    if let Some(dot) = trimmed.find('.') {
        let head = &trimmed[..dot];
        if !head.is_empty()
            && head.chars().all(|c| c.is_ascii_alphabetic() || c == '-')
            && trimmed[dot..].starts_with(". ")
        {
            return trimmed[dot + 2..].trim_start();
        }
    }
    trimmed
}

#[cfg(test)]
mod tests {
    use super::EnglishGlossary;

    #[test]
    fn parses_words_and_strips_part_of_speech() {
        let table = EnglishGlossary::parse("# 注释\n\n开发\tv. develop\tv. exploit\n").unwrap();
        assert_eq!(table.len(), 1);
        assert_eq!(table.gloss_of("开发"), Some("develop; exploit"));
    }

    #[test]
    fn reports_missing_gloss() {
        assert!(EnglishGlossary::parse("开发\n").is_err());
    }

    #[test]
    fn unknown_word_has_no_gloss() {
        let table = EnglishGlossary::parse("开发\tv. develop\n").unwrap();
        assert_eq!(table.gloss_of("不存在"), None);
    }
}
