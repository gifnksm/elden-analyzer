use std::{borrow::Cow, collections::HashSet, sync::LazyLock};

use regex::{Captures, Regex};

use super::{Confidence, Recognition};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostProcess {
    None,
    ItemText,
    ItemCount,
    Digits,
}

impl PostProcess {
    pub fn run(&self, text: &str, conf: Confidence) -> Recognition {
        static REPLACE_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new("[①②③④⑤⑥⑦⑧⑨⑩⑪⑫⑬⑭⑮⑯⑰⑱⑲⑳]").unwrap());
        // WORKAROUND: Tesseract sometimes recognize "1" as "①" etc.
        let text = REPLACE_RE
            .replace_all(text, |cap: &Captures| match cap.get(0).unwrap().as_str() {
                "①" => "1",
                "②" => "2",
                "③" => "3",
                "④" => "4",
                "⑤" => "5",
                "⑥" => "6",
                "⑦" => "7",
                "⑧" => "8",
                "⑨" => "9",
                "⑩" => "10",
                "⑪" => "11",
                "⑫" => "12",
                "⑬" => "13",
                "⑭" => "14",
                "⑮" => "15",
                "⑯" => "16",
                "⑰" => "17",
                "⑱" => "18",
                "⑲" => "19",
                "⑳" => "20",
                _ => unreachable!(),
            })
            .into_owned();

        match self {
            PostProcess::None => Recognition::Possible(text, conf),
            PostProcess::ItemText => item_text(&text, conf),
            PostProcess::ItemCount => item_count(&text, conf),
            PostProcess::Digits => digits(&text, conf),
        }
    }
}

fn is_valid_item_name(name: &str) -> bool {
    static ITEM_NAMES: LazyLock<HashSet<String>> = LazyLock::new(|| {
        let text = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/item.txt"));
        text.lines()
            .filter(|x| !x.is_empty() && !x.starts_with("#"))
            .map(String::from)
            .collect()
    });

    static IGNORE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
        r"^(?:重厚な|鋭利な|上質な|魔力の|炎の|炎術の|雷の|神聖な|毒の|血の|冷たい|神秘の|)|\+\d+$"
    ).unwrap()
    });

    if ITEM_NAMES.contains(name) {
        return true;
    }
    if let Cow::Owned(replaced) = IGNORE_RE.replace(name, "") {
        if ITEM_NAMES.contains(&replaced) {
            return true;
        }
    }
    false
}

fn item_text(text: &str, conf: Confidence) -> Recognition {
    static REPLACE_RE: LazyLock<Vec<(Regex, &str)>> = LazyLock::new(|| {
        vec![
            (Regex::new(r#"[~\-_/*,\."'′\$#\^。]"#).unwrap(), ""),
            (Regex::new(r"^[\d、ー]+").unwrap(), ""),
            (Regex::new(r"[・]+$").unwrap(), ""),
            (Regex::new(r"\(").unwrap(), "（"),
            (Regex::new(r"\)").unwrap(), "）"),
            (Regex::new(r":").unwrap(), "："),
            (Regex::new(r"!").unwrap(), "！"),
            (Regex::new(r"＋").unwrap(), "+"),
            (Regex::new(r"壷").unwrap(), "壺"),
            (Regex::new(r"蝿").unwrap(), "蠅"),
        ]
    });

    static TRY_REPLACE_RE: LazyLock<Vec<(Regex, &str)>> = LazyLock::new(|| {
        let mut re = vec![
            (Regex::new(r"1").unwrap(), "！"),
            (Regex::new(r"`(.*)」").unwrap(), "「$1」"),
            (
                Regex::new(r"(戦灰|呼び声頭|情報|絵画)`?(.*)」").unwrap(),
                "$1「$2」",
            ),
            (
                Regex::new(r"^[ァィゥェォヵヶャュョぁぃぅぇぉゃゅょa-zA-Z]+").unwrap(),
                "",
            ),
            (Regex::new(r"丿レ").unwrap(), "ル"),
            (Regex::new(r"丿（").unwrap(), "ハ"),
            (Regex::new(r"丿（").unwrap(), "バ"),
            (Regex::new(r"丿（").unwrap(), "パ"),
            (Regex::new(r"[十†一]([1-3])$").unwrap(), "+$1"),
            (Regex::new(r"[結総][晶暁賞]").unwrap(), "結晶"),
            (Regex::new(r"[琥琲][珀珈]").unwrap(), "琥珀"),
            (Regex::new(r"[蟷蝉螺鏡][螂螺融]").unwrap(), "蟷螂"),
            (Regex::new(r"蕪葉").unwrap(), "落葉"),
            (Regex::new(r"膨敗").unwrap(), "腐敗"),
            (Regex::new(r"王[筑箇箕符笥]").unwrap(), "王笏"),
            (Regex::new(r"泥[澳澪澤]").unwrap(), "泥濘"),
            (Regex::new(r"蝉血").unwrap(), "蜜血"),
            (Regex::new(r"[息恐]寵").unwrap(), "恩寵"),
            (Regex::new(r"[播撫]き").unwrap(), "擬き"),
            (Regex::new(r"[番男]者").unwrap(), "勇者"),
            (Regex::new(r"冒[淳浄浣]").unwrap(), "冒涜"),
            (Regex::new(r"地場|坤場|堂場|在場|垂場").unwrap(), "坩堝"),
            (Regex::new(r"諸柑|諸杷").unwrap(), "諸相"),
            (Regex::new(r"[逃透送]る").unwrap(), "迸る"),
            (Regex::new(r"[縺縣]り").unwrap(), "縋り"),
            (Regex::new(r"牡").unwrap(), "牢"),
            (Regex::new(r"獲").unwrap(), "獄"),
            (Regex::new(r"[啓]").unwrap(), "壺"),
            (Regex::new(r"[檜横橿楕権様椿楠]").unwrap(), "槍"),
            (Regex::new(r"[報塁]").unwrap(), "睡"),
            (Regex::new(r"[蛹蛸頌蛉蛇]").unwrap(), "蛆"),
            (
                Regex::new(r"[贄贅暫註昭説晴諏曾替暴暮智賜賀]").unwrap(),
                "誓",
            ),
            (Regex::new(r"[錠銃錯錨]").unwrap(), "銛"),
            (Regex::new(r"潰").unwrap(), "漬"),
            (Regex::new(r"[鐙鐘]").unwrap(), "鎧"),
            (Regex::new(r"[縁繰緻]").unwrap(), "緑"),
            (Regex::new(r"[蝦智蜜暖唱]").unwrap(), "帽"),
            (Regex::new(r"[嚢嘆]").unwrap(), "喪"),
            (Regex::new(r"鍬").unwrap(), "鍛"),
            (Regex::new(r"渦").unwrap(), "湧"),
            (Regex::new(r"桔").unwrap(), "枯"),
            (Regex::new(r"[吉号史]").unwrap(), "古"),
            (Regex::new(r"皿").unwrap(), "血"),
            (Regex::new(r"絨").unwrap(), "紐"),
            (Regex::new(r"探").unwrap(), "投"),
            (Regex::new(r"大").unwrap(), "矢"),
            (Regex::new(r"螺").unwrap(), "蟲"),
            (Regex::new(r"[逢道]").unwrap(), "遺"),
            (Regex::new(r"[曇噌噴嘗]").unwrap(), "瞳"),
            (Regex::new(r"惠").unwrap(), "恵"),
            (Regex::new(r"[雷管]").unwrap(), "雫"),
            (Regex::new(r"喋り").unwrap(), "啜り"),
            (Regex::new(r"電").unwrap(), "雷"),
            (Regex::new(r"[縄繩楽編]").unwrap(), "紫"),
            (Regex::new(r"[湛]").unwrap(), "泄"),
            (Regex::new(r"[弟墟]").unwrap(), "兜"),
            (Regex::new(r"[思忠]").unwrap(), "ぬ"),
            (Regex::new(r"[檀操]").unwrap(), "露"),
            (Regex::new(r"[館幽森]").unwrap(), "髪"),
            (Regex::new(r"[瘡瘍]").unwrap(), "瘤"),
            (Regex::new(r"[苛菰芹菩昔苫昼]").unwrap(), "苔"),
            (Regex::new(r"薙").unwrap(), "薬"),
            (Regex::new(r"相").unwrap(), "首"),
            (Regex::new(r"[饂乙人]").unwrap(), "亀"),
            (Regex::new(r"[要芋婆裳菱華製]").unwrap(), "装"),
            (Regex::new(r"[嵌]").unwrap(), "嵐"),
            (Regex::new(r"[翔]").unwrap(), "羽"),
            (Regex::new(r"[蛸蠅]").unwrap(), "蛹"),
            (Regex::new(r"[寿赦]").unwrap(), "毒"),
            (Regex::new(r"[件]").unwrap(), "仗"),
            (Regex::new(r"[福]").unwrap(), "禍"),
            (Regex::new(r"[窒]").unwrap(), "空"),
            (Regex::new(r"[棚柱]").unwrap(), "枷"),
            (Regex::new(r"[斬]").unwrap(), "断"),
            (Regex::new(r"[書脆脊]").unwrap(), "眷"),
            (Regex::new(r"[身]").unwrap(), "鳥"),
            (Regex::new(r"[霧露]").unwrap(), "覆"),
            (Regex::new(r"[田]").unwrap(), "山"),
            (Regex::new(r"[衰]").unwrap(), "衾"),
            (Regex::new(r"[鋼]").unwrap(), "銅"),
            (Regex::new(r"[箭]").unwrap(), "笛"),
            (Regex::new(r"[站峰岳]").unwrap(), "冷"),
            (Regex::new(r"[命]").unwrap(), "岩"),
            (Regex::new(r"[箔]").unwrap(), "笠"),
            (Regex::new(r"[播撚]").unwrap(), "揺"),
        ];
        let ambicious_pairs = &[
            ("木", "本"),
            ("土", "士"),
            ("日", "目"),
            ("白", "自"),
            ("ェ", "ュ"),
            ("へ", "ヘ"),
            ("ハ", "バ"),
            ("ヒ", "ビ"),
            ("フ", "ブ"),
            ("ヘ", "ベ"),
            ("ホ", "ボ"),
            ("ハ", "パ"),
            ("ヒ", "ピ"),
            ("フ", "プ"),
            ("ヘ", "ペ"),
            ("ホ", "ポ"),
            ("パ", "バ"),
            ("ピ", "ビ"),
            ("プ", "ブ"),
            ("ペ", "ベ"),
            ("ポ", "ボ"),
            ("グ", "ダ"),
            ("レ", "ル"),
            ("ーン", "シ"),
            ("き", "ミ"),
            ("き", "さ"),
            ("ぎ", "ざ"),
            ("ぎ", "さ"),
            ("そ", "え"),
            ("る", "さ"),
            ("る", "ざ"),
            ("る", "え"),
            ("る", "を"),
            ("ま", "よ"),
            ("ぬ", "ゐ"),
            ("イ", "ィ"),
            ("力", "カ"),
            ("カ", "ヵ"),
            ("ケ", "ヶ"),
            ("タ", "ツ"),
        ];
        for (a, b) in ambicious_pairs {
            assert_ne!(a, b);
            re.extend([
                (Regex::new(&format!("{b}{a}|{a}{b}?")).unwrap(), *b),
                (Regex::new(&format!("{a}{b}|{b}{a}?")).unwrap(), *a),
            ]);
        }
        re
    });

    let decayed_conf = conf * 8 / 10;
    let mut conf = conf;

    let mut text = Cow::Borrowed(text);
    for (reg, repl) in &*REPLACE_RE {
        if let Cow::Owned(owned) = reg.replace_all(text.as_ref(), *repl) {
            text = Cow::Owned(owned);
            conf = decayed_conf;
        }
    }

    if is_valid_item_name(text.as_ref()) {
        return Recognition::Found(text.into_owned(), conf);
    }

    let mut candidates = HashSet::new();
    candidates.insert(text.clone().into_owned());
    for (reg, repl) in &*TRY_REPLACE_RE {
        let mut new_candidates = HashSet::with_capacity(candidates.capacity());
        for cand in candidates {
            for caps in reg.captures_iter(&cand) {
                let m = caps.get(0).unwrap();
                let mut replaced = cand[..m.start()].to_owned();
                caps.expand(repl, &mut replaced);
                replaced += &cand[m.end()..];
                if is_valid_item_name(&replaced) {
                    return Recognition::Found(replaced, decayed_conf);
                }
                new_candidates.insert(replaced);
            }
            new_candidates.insert(cand);
        }
        candidates = new_candidates;
    }
    tracing::trace!(?candidates);

    Recognition::Possible(text.into_owned(), conf)
}

fn item_count(text: &str, conf: Confidence) -> Recognition {
    static PREFIX_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^[×xX〆くへヘべベメ＜＞※]+").unwrap());
    static TEXT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^×\d+$").unwrap());

    let text = PREFIX_RE.replace_all(text, "×").into_owned();
    if TEXT_RE.is_match(&text) {
        Recognition::Found(text, conf)
    } else {
        Recognition::Possible(text, conf)
    }
}

fn digits(text: &str, conf: Confidence) -> Recognition {
    if text.chars().all(char::is_numeric) {
        Recognition::Found(text.to_owned(), conf)
    } else {
        Recognition::Possible(text.to_owned(), conf)
    }
}
