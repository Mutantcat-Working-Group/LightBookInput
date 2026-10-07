//! 呈现：删候选、按应用关英文候选、会话重置与候选窗口绘制。

use super::*;

impl Host {
    /// 程序员模式下左右键切换英文释义。返回是否有变化。
    pub fn move_gloss(&mut self, delta: isize) -> bool {
        let Some(candidate) = self.session.candidate(self.session.highlighted) else {
            return false;
        };
        let Some(gloss) = self.engine.english_gloss(&candidate.text) else {
            return false;
        };
        let count = gloss.split("; ").count();
        if count <= 1 {
            return false;
        }
        let current = self.gloss_index as isize;
        let next = (current + delta).rem_euclid(count as isize) as usize;
        if next == self.gloss_index {
            return false;
        }
        self.gloss_index = next;
        true
    }

    /// 删掉当前页第 `offset` 格的候选：用户词整个删、词库词清学习。返回给用户看的一句话；那格没有候选返回 `None`。
    pub fn forget_candidate(&mut self, offset: usize) -> Option<String> {
        let index = self.session.index_on_page(offset)?;
        let candidate = self.session.candidate(index)?;
        let forgotten = self.engine.forget(&candidate);
        let text = &candidate.text;
        Some(if forgotten.user_word {
            format!("已删除用户词「{text}」")
        } else if forgotten.learning {
            format!("已忘掉对「{text}」的学习记录")
        } else {
            format!("「{text}」是词库里的词，也没有学习记录，没什么可删")
        })
    }

    /// 这个应用里英文模式给不给候选：全局开关开着，且应用不在 `[apps] english_candidates_off` 里。
    pub fn english_candidates_in(&self, bundle: Option<&str>) -> bool {
        self.english_candidates && !bundle.is_some_and(|b| self.apps.english_candidates_off(b))
    }

    /// 横排矩阵这套按键是否生效：开关开着（`[general] horizontal_grid`）而且排布是横排。
    pub fn grid_keys(&self) -> bool {
        self.horizontal_grid && self.layout == LayoutMode::Horizontal
    }

    /// 新一轮候选：每页格数取配置与窗口能画的行数中较小者，云端槽位数取配置。
    pub fn reset_session(&mut self, preedit: Option<Preedit>, candidates: Vec<Candidate>) {
        self.status = None;
        let page_size = self.page_size.min(self.window.max_rows()).max(1);
        self.session
            .reset(preedit, candidates, page_size, self.cloud_slots);
        self.gloss_index = 0;
    }

    /// 按会话状态画候选窗口。候选为空且没有 preedit 时收窗。
    pub fn render(&mut self) {
        let size = self.session.layout.page_size();
        let page = self.session.page;
        // 横排展开成矩阵时画视口里的几行，序号只标在高亮所在那一行（数字键选的就是它）；单行时只画当前页
        let grid = self.session.grid_cells();
        let first = self.session.grid.map_or(page, |grid| grid.top()) * size;
        let (cells, columns) = match &grid {
            Some((cells, columns)) => (cells.clone(), *columns),
            None => (self.session.page_cells(), 0),
        };
        let rows: Vec<Row> = cells
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                let offset = i % size;
                let labelled = columns == 0 || (first + i) / size == page;
                let index = if labelled {
                    (offset + 1).to_string()
                } else {
                    String::new()
                };
                let Some(candidate) = cell.candidate() else {
                    return Row {
                        // 矩阵里的空位什么都不画；单行里的空位留着序号
                        index: if columns == 0 { index } else { String::new() },
                        text: String::new(),
                        annotation: Vec::new(),
                        cloud: false,
                    };
                };
                let mut row = Row::from_candidate(offset, candidate);
                row.index = index;
                row.cloud = candidate.kind == CandidateKind::Cloud;
                // 程序员模式（按住 ~）：右侧补一行英文释义，方向键选、数字键 / 空格上屏的就是它
                let gloss = self
                    .programmer_mode
                    .then(|| {
                        self.engine
                            .english_gloss(&candidate.text)
                            .map(|gloss| gloss.split("; ").map(str::to_owned).collect::<Vec<_>>())
                    })
                    .flatten();
                row.with_gloss(gloss, self.gloss_index)
            })
            .collect();
        // 配置成只在行内显示时，窗口顶部不画拼音行
        let preedit = self
            .preedit_mode
            .in_window()
            .then(|| self.session.preedit.clone())
            .flatten();
        if rows.is_empty() && self.session.preedit.is_none() {
            self.window.hide();
            return;
        }
        let pages = self.session.pages();
        let footer = (pages > 1).then(|| format!("{}/{pages}", page + 1));
        // 程序员模式开着时常驻一行状态：用户得知道现在数字键选的是英文
        let status = if self.programmer_mode {
            Some(PROGRAMMER_STATUS.to_owned())
        } else {
            self.status.clone()
        };
        let frame = Frame {
            preedit,
            rows,
            highlighted: self.session.highlighted.saturating_sub(first),
            columns,
            column_ems: if columns > 0 {
                lightbookinput_core::Grid::column_ems(&self.session.layout)
            } else {
                Vec::new()
            },
            footer,
            sentence: self.sentence.clone(),
            status,
        };
        self.window.show(frame, self.anchor);
    }
}

/// 程序员模式开着时候选窗口里常驻的一行字。
const PROGRAMMER_STATUS: &str = "程序员模式：↑↓ 换词 ←→ 换释义，数字 / 空格上屏，Esc 退出";
