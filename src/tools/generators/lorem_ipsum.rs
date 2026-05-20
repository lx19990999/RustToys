use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use rand::Rng;

// Classic Lorem Ipsum word set
const LOREM_WORDS: &[&str] = &[
    "lorem", "ipsum", "dolor", "sit", "amet", "consectetur", "adipiscing", "elit",
    "sed", "do", "eiusmod", "tempor", "incididunt", "ut", "labore", "et", "dolore",
    "magna", "aliqua", "enim", "ad", "minim", "veniam", "quis", "nostrud",
    "exercitation", "ullamco", "laboris", "nisi", "aliquip", "ex", "ea", "commodo",
    "consequat", "duis", "aute", "irure", "in", "reprehenderit", "voluptate",
    "velit", "esse", "cillum", "fugiat", "nulla", "pariatur", "excepteur", "sint",
    "occaecat", "cupidatat", "non", "proident", "sunt", "culpa", "qui", "officia",
    "deserunt", "mollit", "anim", "id", "est", "laborum", "perspiciatis", "unde",
    "omnis", "iste", "natus", "error", "voluptatem", "accusantium", "doloremque",
    "laudantium", "totam", "rem", "aperiam", "eaque", "ipsa", "quae", "ab", "illo",
    "inventore", "veritatis", "quasi", "architecto", "beatae", "vitae", "dicta",
    "explicabo", "nemo", "ipsam", "quia", "voluptas", "aspernatur", "aut", "odit",
    "fugit", "consequuntur", "magni", "dolores", "eos", "ratione", "sequi", "nesciunt",
];

// Extended Latin word set (more varied)
const EXTENDED_WORDS: &[&str] = &[
    "lorem", "ipsum", "dolor", "sit", "amet", "consectetur", "adipiscing", "elit",
    "sed", "do", "eiusmod", "tempor", "incididunt", "ut", "labore", "et", "dolore",
    "magna", "aliqua", "enim", "ad", "minim", "veniam", "quis", "nostrud",
    "exercitation", "ullamco", "laboris", "nisi", "aliquip", "ex", "ea", "commodo",
    "consequat", "duis", "aute", "irure", "in", "reprehenderit", "voluptate",
    "velit", "esse", "cillum", "fugiat", "nulla", "pariatur", "excepteur", "sint",
    "occaecat", "cupidatat", "non", "proident", "sunt", "culpa", "qui", "officia",
    "deserunt", "mollit", "anim", "id", "est", "laborum", "perspiciatis", "unde",
    "omnis", "iste", "natus", "error", "voluptatem", "accusantium", "doloremque",
    "laudantium", "totam", "rem", "aperiam", "eaque", "ipsa", "quae", "ab", "illo",
    "inventore", "veritatis", "quasi", "architecto", "beatae", "vitae", "dicta",
    "explicabo", "nemo", "ipsam", "quia", "voluptas", "aspernatur", "aut", "odit",
    "fugit", "consequuntur", "magni", "dolores", "eos", "ratione", "sequi", "nesciunt",
    "commodi", "consequatur", "porro", "possimus", "repellendus", "suscipit", "saepe",
    "repudiandae", "atque", "eveniet", "optio", "dignissimos", "ducimus", "placeat",
    "recusandae", "tempora", "praesentium", "corrupti", "assumenda", "nihil",
    "temporibus", "architecto", "impedit", "facere", "debitis", "maxime", "minima",
    "nostrum", "necessitatibus", "eligendi", "harum", "reiciendis", "incidunt",
    "dolorem", "iusto", "consequuntur", "numquam", "neque", "aliquid", "similique",
    "accusamus", "quaerat", "deserunt", "repellat", "voluptatum", "sapiente",
    "laboriosam", "blanditiis", "distinctio", "molestias", "illum", "perferendis",
    "obcaecati", "molestiae", "quibusdam", "itaque", "deleniti", "fuga", "earum",
];

// Simple English word set
const ENGLISH_WORDS: &[&str] = &[
    "the", "quick", "brown", "fox", "jumps", "over", "lazy", "dog", "and",
    "runs", "through", "green", "fields", "beneath", "blue", "sky", "with",
    "birds", "singing", "softly", "trees", "sway", "gentle", "breeze",
    "morning", "sun", "casts", "golden", "light", "across", "meadow",
    "flowers", "bloom", "vibrant", "colors", "bees", "buzz", "around",
    "stream", "flows", "quietly", "between", "mossy", "rocks", "fish",
    "swim", "deep", "clear", "water", "deer", "graze", "near", "edge",
    "forest", "rabbits", "hop", "playfully", "among", "wildflowers",
    "squirrels", "chase", "each", "other", "pine", "branches", "owl",
    "watches", "from", "high", "above", "moon", "rises", "stars",
    "appear", "one", "by", "night", "falls", "peaceful", "world",
    "dreams", "drift", "sleeping", "creatures", "fireflies", "glow",
    "softly", "warm", "summer", "evening", "crickets", "chirp",
    "rhythm", "nature", "rests", "awaits", "dawn", "promise",
    "another", "beautiful", "day", "filled", "wonder", "adventure",
    "discovery", "hope", "joy", "laughter", "friends", "family",
    "together", "memories", "lasting", "forever", "timeless",
];

// Chinese Lorem-like text set
const CHINESE_WORDS: &[&str] = &[
    "春", "夏", "秋", "冬", "风", "花", "雪", "月", "山", "水",
    "天", "地", "日", "月", "星", "云", "雨", "雷", "电", "虹",
    "江", "河", "湖", "海", "溪", "泉", "潭", "池", "瀑", "涛",
    "松", "竹", "梅", "兰", "菊", "荷", "桂", "柳", "桃", "杏",
    "鸟", "鱼", "虫", "兽", "龙", "凤", "鹤", "鹿", "蝶", "蝉",
    "诗", "词", "歌", "赋", "琴", "棋", "书", "画", "笔", "墨",
    "红", "黄", "蓝", "绿", "紫", "橙", "白", "黑", "青", "翠",
    "东", "南", "西", "北", "上", "下", "左", "右", "前", "后",
    "云", "霞", "雾", "露", "霜", "烟", "尘", "沙", "石", "玉",
    "琴", "瑟", "笙", "箫", "鼓", "钟", "磬", "笛", "筝", "弦",
];

const TEXT_LIBRARIES: &[(&str, &[&str])] = &[
    ("Classic Lorem", LOREM_WORDS),
    ("Extended Latin", EXTENDED_WORDS),
    ("Simple English", ENGLISH_WORDS),
    ("Chinese Poetic", CHINESE_WORDS),
];

pub struct LoremIpsum {
    output: String,
    count: usize,
    mode: usize, // 0=paragraphs, 1=sentences, 2=words
    library: usize,
    start_with_classic: bool,
    initialized: bool,
}

impl Default for LoremIpsum {
    fn default() -> Self {
        Self {
            output: String::new(),
            count: 3,
            mode: 0,
            library: 0,
            start_with_classic: true,
            initialized: false,
        }
    }
}

impl LoremIpsum {
    fn generate(&mut self) {
        let (_, words) = TEXT_LIBRARIES[self.library];
        let mut rng = rand::thread_rng();

        self.output = match self.mode {
            0 => {
                // Paragraphs
                let mut paragraphs = Vec::new();
                if self.start_with_classic && self.library == 0 {
                    paragraphs.push(
                        "Lorem ipsum dolor sit amet, consectetur adipiscing elit, \
                         sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. \
                         Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris \
                         nisi ut aliquip ex ea commodo consequat."
                            .to_string(),
                    );
                    for _ in 1..self.count {
                        let sentence_count = rng.gen_range(3..=8);
                        paragraphs.push(generate_paragraph(&mut rng, words, sentence_count));
                    }
                } else {
                    for _ in 0..self.count {
                        let sentence_count = rng.gen_range(3..=8);
                        paragraphs.push(generate_paragraph(&mut rng, words, sentence_count));
                    }
                }
                paragraphs.join("\n\n")
            }
            1 => {
                // Sentences
                let mut sentences = Vec::new();
                if self.start_with_classic && self.library == 0 {
                    sentences.push(
                        "Lorem ipsum dolor sit amet, consectetur adipiscing elit."
                            .to_string(),
                    );
                    for _ in 1..self.count {
                        let word_count = rng.gen_range(8..=20);
                        sentences.push(generate_sentence(&mut rng, words, word_count));
                    }
                } else {
                    for _ in 0..self.count {
                        let word_count = rng.gen_range(8..=20);
                        sentences.push(generate_sentence(&mut rng, words, word_count));
                    }
                }
                sentences.join(" ")
            }
            2 => {
                // Words
                let mut word_list = Vec::new();
                if self.start_with_classic && self.library == 0 {
                    let classic = ["Lorem", "ipsum", "dolor", "sit", "amet"];
                    for w in classic {
                        word_list.push(w.to_string());
                    }
                    for _ in classic.len()..self.count {
                        word_list.push(words[rng.gen_range(0..words.len())].to_string());
                    }
                } else {
                    for _ in 0..self.count {
                        word_list.push(words[rng.gen_range(0..words.len())].to_string());
                    }
                }
                if let Some(first) = word_list.first_mut() {
                    if let Some(c) = first.chars().next() {
                        first.replace_range(0..c.len_utf8(), &c.to_uppercase().to_string());
                    }
                }
                word_list.join(" ") + "."
            }
            _ => String::new(),
        };
    }
}

impl Tool for LoremIpsum {
    fn name(&self) -> &str { "Lorem Ipsum Generator" }
    fn description(&self) -> &str { "Generate words, sentences or paragraphs of Lorem Ipsum" }
    fn category(&self) -> ToolCategory { ToolCategory::Generators }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if !self.initialized {
            self.generate();
            self.initialized = true;
        }

        // Generation type
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.mode, 0, "Paragraphs");
            ui.radio_value(&mut self.mode, 1, "Sentences");
            ui.radio_value(&mut self.mode, 2, "Words");
        });
        ui.add_space(4.0);

        // Text library selection
        ui.horizontal(|ui| {
            ui.label("Text Library:");
            egui::ComboBox::from_id_salt("lorem_library")
                .selected_text(TEXT_LIBRARIES[self.library].0)
                .show_ui(ui, |ui| {
                    for (i, (name, _)) in TEXT_LIBRARIES.iter().enumerate() {
                        ui.selectable_value(&mut self.library, i, *name);
                    }
                });
        });
        ui.add_space(4.0);

        // Count
        ui.horizontal(|ui| {
            let label = match self.mode {
                0 => "Paragraphs:",
                1 => "Sentences:",
                2 => "Words:",
                _ => "Count:",
            };
            ui.label(label);
            ui.add(egui::DragValue::new(&mut self.count).range(1..=100).speed(1));
        });
        ui.add_space(4.0);

        // Start with classic option (only for Classic Lorem library)
        if self.library == 0 {
            ui.checkbox(&mut self.start_with_classic, "Start with \"Lorem ipsum dolor sit amet...\"");
            ui.add_space(4.0);
        } else {
            self.start_with_classic = false;
        }

        // Action buttons
        ui.horizontal(|ui| {
            if ui.button("Generate").clicked() {
                self.generate();
            }
            if ui.button("Refresh").clicked() {
                self.generate();
            }
        });
        ui.add_space(4.0);

        // Output
        ui.label("Output:");
        ui.add(
            egui::TextEdit::multiline(&mut self.output)
                .desired_width(f32::INFINITY)
                .desired_rows(12),
        );

        // Output action buttons
        if !self.output.is_empty() {
            ui.horizontal(|ui| {
                if ui.button("Copy").clicked() {
                    ui.ctx().copy_text(self.output.clone());
                }
                if ui.button("Save As...").clicked() {
                    if let Some(path) = crate::tools::async_utils::save_file_dialog("Save as", "Text", &["txt"], "lorem_ipsum.txt") {
                        let _ = std::fs::write(path, &self.output);
                    }
                }
            });
        }
    }
}

fn generate_sentence(rng: &mut impl Rng, words: &[&str], word_count: usize) -> String {
    let mut ws: Vec<&str> = Vec::new();
    for _ in 0..word_count {
        ws.push(words[rng.gen_range(0..words.len())]);
    }
    let mut sent = ws.join(" ");
    if let Some(c) = sent.chars().next() {
        sent.replace_range(0..c.len_utf8(), &c.to_uppercase().to_string());
    }
    sent.push('.');
    sent
}

fn generate_paragraph(rng: &mut impl Rng, words: &[&str], sentence_count: usize) -> String {
    let mut sentences = Vec::new();
    for _ in 0..sentence_count {
        let word_count = rng.gen_range(8..=20);
        sentences.push(generate_sentence(rng, words, word_count));
    }
    sentences.join(" ")
}
