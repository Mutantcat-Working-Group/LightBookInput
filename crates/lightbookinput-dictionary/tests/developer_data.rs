//! 开发者词库与中英释义表的数据契约测试。
//!
//! 两份人工覆盖文件是源数据；运行时文件由它们和生成数据合并。测试保证覆盖词没有在
//! 后续重跑数据管道时悄悄丢失，也保证随包词库的可见署名统一为「轻书」。

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use lightbookinput_dictionary::EnglishGlossary;

const MIN_DEVELOPER_TERMS: usize = 120;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn rows(path: &Path) -> Vec<Vec<String>> {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("读不了 {}：{error}", path.display()))
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.split('\t').map(str::to_owned).collect())
        .collect()
}

fn runtime_dictionary_words() -> HashSet<String> {
    let mut found = HashSet::new();
    let dicts = workspace_root().join("assets/lexicon/dicts");
    let mut files = vec![workspace_root().join("assets/lexicon/dict.tsv")];
    files.extend(
        fs::read_dir(&dicts)
            .unwrap_or_else(|error| panic!("读不了 {}：{error}", dicts.display()))
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "tsv")),
    );
    files.sort();
    for path in files {
        for row in rows(&path) {
            let [word, pinyin, frequency, ..] = row.as_slice() else {
                panic!(
                    "{} 的运行时词库行需要 词、拼音、词频：{row:?}",
                    path.display()
                );
            };
            assert_eq!(
                pinyin.split_whitespace().count(),
                word.chars().count(),
                "{} 的 {word} 拼音音节数与字数不一致：{pinyin}",
                path.display()
            );
            assert!(
                frequency.parse::<u64>().is_ok(),
                "{} 的 {word} 词频不是非负整数：{frequency}",
                path.display()
            );
            found.insert(word.clone());
        }
    }
    found
}

#[test]
fn developer_dictionary_overlay_is_merged_into_runtime_data() {
    let overlay_path = workspace_root().join("assets/lexicon/developer_words.tsv");
    let overlay = rows(&overlay_path);
    assert!(
        overlay.len() >= MIN_DEVELOPER_TERMS,
        "开发者词库只有 {} 条，目标至少 {MIN_DEVELOPER_TERMS} 条",
        overlay.len()
    );

    let mut seen = HashSet::new();
    let runtime = runtime_dictionary_words();
    let mut missing = Vec::new();
    for row in &overlay {
        let [word, _, pinyin, ..] = row.as_slice() else {
            panic!("developer_words.tsv 每行需要 词、次数、拼音：{row:?}");
        };
        assert!(
            seen.insert(word.clone()),
            "developer_words.tsv 重复词：{word}"
        );
        assert_eq!(
            pinyin.split_whitespace().count(),
            word.chars().count(),
            "{word} 的拼音音节数与字数不一致：{pinyin}"
        );
        if !runtime.contains(word) {
            missing.push(word.clone());
        }
    }
    missing.sort();
    assert!(
        missing.is_empty(),
        "开发者词没有并入运行时词库：{}",
        missing.join("、")
    );
}

#[test]
fn developer_glossary_overlay_is_merged_into_runtime_glossary() {
    let overlay_path = workspace_root().join("assets/glossary/developer-terms.tsv");
    let overlay = EnglishGlossary::from_path(&overlay_path).unwrap();
    assert!(
        overlay.len() >= MIN_DEVELOPER_TERMS,
        "开发者释义只有 {} 条，目标至少 {MIN_DEVELOPER_TERMS} 条",
        overlay.len()
    );

    let runtime =
        EnglishGlossary::from_path(&workspace_root().join("assets/glossary/glossary-en.tsv"))
            .unwrap();
    let overlay_rows = rows(&overlay_path);
    let mut missing = Vec::new();
    for row in overlay_rows {
        let [word, ..] = row.as_slice() else {
            continue;
        };
        if runtime.gloss_of(word) != overlay.gloss_of(word) {
            missing.push(format!("{word}: {:?}", overlay.gloss_of(word)));
        }
    }
    missing.sort();
    assert!(
        missing.is_empty(),
        "开发者释义没有按覆盖顺序并入运行时释义表：{}",
        missing.join("、")
    );
}

#[test]
fn bundled_dictionary_sources_use_lightbook_name() {
    let mut sources = vec![
        workspace_root().join("assets/lexicon/dict.tsv"),
        workspace_root().join("assets/lexicon/developer_words.tsv"),
        workspace_root().join("assets/wubi/wubi86.tsv"),
    ];
    let dicts = workspace_root().join("assets/lexicon/dicts");
    let mut domain_sources: Vec<PathBuf> = fs::read_dir(&dicts)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "tsv"))
        .collect();
    domain_sources.sort();
    sources.extend(domain_sources);

    for path in sources {
        let source = fs::read_to_string(&path).unwrap();
        let first_line = source.lines().next().unwrap_or_default();
        assert!(
            first_line.contains("轻书"),
            "{} 第一行没有轻书署名：{first_line}",
            path.display()
        );
        assert!(
            !source.contains("青简"),
            "{} 仍含旧词库名称",
            path.display()
        );
    }
}
