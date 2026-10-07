<div align=center>
<img src="icon.png" style="width:100px;" width="100" alt="LightBookInput icon"/>
<h2>LightBookInput (轻书)</h2>
</div>

[简体中文](README.md) | English

### 1. Overview

- Cross-platform input method for macOS, Windows, and Linux (Fcitx5). The engine is platform-independent; each platform is only a shell around it.
- Programmer mode: hold `~` (·) and the English for the highlighted candidate appears beside the candidates. Press a number or `Space` to commit it directly, so writing code or looking up wording never needs a second input method.
- Sentence input: bigram language model with Viterbi plus beam search, whole-sentence abbreviations, personal n-gram learning online, and a small local Transformer that rescoring the top paths after a pause.
- Self-built dictionary: 205,000 base entries plus 11 domain dictionaries, mmap'd zero-copy from the `.qj` binary container with roughly 50 ms startup.
- Privacy first: pinyin conversion, dictionary lookup, local models, and input-habit learning all run on your device. No account, no upload.
- Cloud prediction (optional, off by default): when enabled, the current input and nearby text go to an AI provider you configure yourself, which returns candidates or a sentence completion.

Core value:

- English within reach: programmer mode is a local table lookup only, so the keypress commits the English without breaking your Chinese input or sending anything anywhere.
- Feels more like you over time: personal bigram and trigram counts are learned online, and whole-sentence conversion keeps getting closer to your phrasing.
- Reliable engineering: learning data is written atomically, tolerates corrupt files, and survives crashes, flushing at least once a minute while an input session is active.
- Consistent across platforms: the core and the platform layer are strictly separated, so macOS, Windows, and Linux share one input engine.

### 2. Features

#### Typing experience

- Full pinyin, abbreviations, double pinyin (Xiaohe, Ziranma, Microsoft, Sogou), zhuyin, and wubi (in progress).
- Whole-sentence conversion: word graph, Viterbi, and beam search, with the full sentence ranked first.
- Spelling correction: one edit (transposition, substitution, insertion, or deletion) still produces a complete syllable.
- Fuzzy pinyin: z/zh, c/ch, s/sh, n/l, f/h, l/r, an/ang, en/eng, in/ing.
- Mixed Chinese and English typing: English candidates appear as soon as the string matches the English word list.
- English mode: exact matches, prefix completion, and single-edit correction.
- Emoji candidates follow the matching word and label the word itself.
- Quick candidates: `rq`, `sj`, and `xq` produce date, time, and weekday; `v` starts expression mode for arithmetic.

#### Programmer mode

- While `~` (·) is held, the English for each candidate appears beside it and the status row hints what to do.
- Press `1` to `9` to commit the English of that candidate; press `Space` to commit the first one.
- Release the key, press `~` again, press `Esc`, or type anything else to leave the mode and return to normal Chinese input.
- 239,000 Chinese-to-English entries ship with the product, resolved entirely on this device.

#### Input statistics

- The statistics page shows today, the last 7 days, and lifetime input volume, plus the equivalent number of books.

#### Personalization

- Personal n-gram: bigram and trigram counts interpolated with the static model by evidence, updated online as you commit.
- Personal word frequency, user words, a personal English word list, and a personal typo table.
- Automatic word creation: two words chosen in a row that are not in the dictionary become a user word once they pass the counting threshold.
- Local sentence model: a small Transformer (candle) rescoring the top whole-sentence paths.

#### Cloud prediction (optional)

- Local candidates and the local sentence are marked in the prompt as possibly-wrong local guesses, so the model judges on its own and only returns what local lookup cannot give.
- Question mode: `?` plus a pinyin question, over the same predictor channel.
- Sentence completion: one request after a pause returns both cloud words and a sentence completion shown to the right of the pinyin, accepted with `Tab`.

### 3. Installation & Downloads

1. **macOS / Windows**: download the latest installer from the [website](https://lightbookinput.app/download).
    - macOS: the pkg installs into `/Library/Input Methods/`, registering and enabling the input source automatically.
    - Windows: the NSIS installer includes the server process, the TSF DLL, and the WinUI 3 settings app.
2. **Linux (Fcitx5)**: install from source and start the background server manually. See the [Linux guide](https://lightbookinput.app/docs/getting-started/linux).
3. **Build from source**: Rust >= 1.88, then `cargo build --workspace`.

### 4. Quick Start

1. Type as usual: enter pinyin, choose candidates, write whole sentences.
2. When you need English, hold `~` (·) and the candidates flip to their English spellings; press a number or `Space` to commit.
3. Release the key and you are back in Chinese input, with no input-method switching and no settings to change.

Full usage documentation is at [the docs site](https://lightbookinput.app/docs).

### 5. Roadmap

- [x] Core input engine (pinyin parsing, candidate generation, ranking, sentences, double pinyin, zhuyin, English mode)
- [x] Self-built dictionary (205,000 base entries plus 11 domain dictionaries)
- [x] Programmer mode (hold `~` to reveal candidate English, commit with number or space)
- [x] Personal n-gram (bigram and trigram)
- [x] Local sentence model (small Transformer rescoring)
- [x] Cloud prediction (OpenAI-compatible API)
- [x] macOS shell (IMK, self-drawn candidate window, preferences)
- [x] Windows shell (TSF, server, WinUI 3 settings, NSIS installer)
- [x] Linux shell (Fcitx5 and server)
- [ ] Wubi (version 86, first phase covers core, CLI, and Windows)
- [ ] Personal fine-tuning (LoRA on a frozen base model)
- [ ] Idle-time training (gated on power, temperature, and idle duration)

[GPL-3.0-or-later](LICENSE)

---

## Acknowledgments

This project is a fork of [qingjian-team/qingjian](https://github.com/qingjian-team/qingjian). Thanks to the original repository and its authors for the excellent open-source work; this repository continues to maintain and improve on top of it.
