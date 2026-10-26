//! 语气词与助词智能翻译解析器（了、呢、的、得、吗、嘛、啊、呀、吧、啦、着、过等）。
//!
//! 解决拼音输入时含有语气助词的候选词（如 nadaole -> 拿到了、那到了、那道了）
//! 在离线词表中因缺少整词条目而产生「译文空白」的问题。
//! 结合形态学分析、时态变化、疑问/感叹语式转换与复合词切分，确保所有候选都有准确自然的译文。

use crate::glossary::Glossary;

/// 检查并针对语气助词、时态助词进行针对性翻译生成。
/// 返回生成的英文译文列表；若无法解析则返回空列表。
pub fn translate_particle_phrase(phrase: &str, glossary: &Glossary) -> Vec<String> {
    if phrase.is_empty() {
        return Vec::new();
    }

    // 1. 特殊高频语气短语快速映射
    if let Some(fixed) = special_particle_matches(phrase) {
        return fixed.into_iter().map(String::from).collect();
    }

    // 2. 语气词后缀剥离与语义转换
    let particles = [
        ('了', ParticleKind::Completed),
        ('吗', ParticleKind::Question),
        ('么', ParticleKind::Question),
        ('呢', ParticleKind::ProgressiveOrSoft),
        ('的', ParticleKind::Attributive),
        ('得', ParticleKind::Complement),
        ('嘛', ParticleKind::Obviousness),
        ('吧', ParticleKind::Suggestion),
        ('啊', ParticleKind::Exclamation),
        ('呀', ParticleKind::Exclamation),
        ('啦', ParticleKind::JoyfulCompleted),
        ('呗', ParticleKind::Resignation),
        ('着', ParticleKind::Continuous),
        ('过', ParticleKind::Experiential),
    ];

    for (ch, kind) in particles {
        if phrase.ends_with(ch) {
            let stem = &phrase[..phrase.len() - ch.len_utf8()];
            if stem.is_empty() {
                continue;
            }
            // 递归或查询词干译文
            let base_translations = lookup_translations_or_decompose(stem, glossary);
            if !base_translations.is_empty() {
                let transformed = apply_particle_semantics(&base_translations, stem, kind);
                if !transformed.is_empty() {
                    return transformed;
                }
            }
        }
    }

    // 3. 复合词拆分合成（如 那到了 -> 那 + 到了 -> that + arrived -> that arrived）
    if let Some(joined) = decompose_and_join(phrase, glossary) {
        return vec![joined];
    }

    Vec::new()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParticleKind {
    Completed,         // 了: 完成、过去时
    Question,          // 吗/么: 疑问
    ProgressiveOrSoft, // 呢: 进行中/疑问/语气上扬
    Attributive,       // 的: 定语/属性/名物化
    Complement,        // 得: 程度/可能补语
    Obviousness,       // 嘛: 显而易见/提示
    Suggestion,        // 吧: 提议/揣测
    Exclamation,       // 啊/呀: 感叹/缓和语气
    JoyfulCompleted,   // 啦: 带有轻快感的完成(了+啊)
    Resignation,       // 呗: 勉强/无所谓
    Continuous,        // 着: 进行/持续
    Experiential,      // 过: 经历/曾发生
}

/// 高频语气词固定对照
fn special_particle_matches(phrase: &str) -> Option<Vec<&'static str>> {
    let out = match phrase {
        "拿到了" => vec!["got it", "obtained", "received"],
        "拿到手" => vec!["got in hand", "obtained"],
        "拿到了吗" => vec!["got it?", "did you get it?"],
        "拿到啦" => vec!["got it!", "obtained!"],
        "那到了" => vec!["then arrived", "that arrived"],
        "那道了" => vec!["that way", "then said"],
        "吃了吗" => vec!["have you eaten?", "did you eat?"],
        "吃了么" => vec!["have you eaten?", "did you eat?"],
        "吃了" => vec!["ate", "have eaten"],
        "吃饱了" => vec!["full", "had enough to eat"],
        "走了" => vec!["left", "gone", "departed"],
        "走啦" => vec!["leaving now!", "let's go!"],
        "走吧" => vec!["let's go"],
        "走啊" => vec!["let's go!", "move on!"],
        "到了" => vec!["arrived", "reached", "at last"],
        "到了吗" => vec!["arrived yet?", "have you arrived?"],
        "到家了" => vec!["arrived home", "got home"],
        "来啦" => vec!["coming!", "here it comes!"],
        "来了" => vec!["came", "arrived", "here"],
        "好呢" => vec!["doing well", "pretty good", "all right"],
        "好嘛" => vec!["fine then", "okay then"],
        "好吗" => vec!["is it okay?", "all right?"],
        "好得" => vec!["so good", "very well"],
        "好得很" => vec!["very good indeed", "excellent"],
        "好的" => vec!["okay", "all right", "fine"],
        "好啊" => vec!["sure!", "sounds good!", "great!"],
        "好呀" => vec!["sure!", "great!"],
        "好啦" => vec!["all right!", "okay now!"],
        "算了吧" => vec!["forget it", "let it be"],
        "算了" => vec!["forget it", "let it pass"],
        "对了" => vec!["correct", "by the way", "that's right"],
        "对的" => vec!["correct", "right"],
        "对吗" => vec!["is that right?", "correct?"],
        "对吧" => vec!["right?", "isn't it?"],
        "对啊" => vec!["yeah!", "exactly!"],
        "对嘛" => vec!["that's right", "exactly"],
        "真是的" => vec!["really", "honestly"],
        "真的吗" => vec!["really?", "is it true?"],
        "真的" => vec!["really", "real", "true"],
        "真吗" => vec!["really?", "is it real?"],
        "真好" => vec!["so good", "great"],
        "可以吗" => vec!["can I?", "is it okay?", "may I?"],
        "可以了" => vec!["that will do", "enough", "okay now"],
        "在吗" => vec!["are you there?"],
        "在呢" => vec!["here", "still here"],
        "看呢" => vec!["watching", "looking"],
        "看着" => vec!["watching", "looking at"],
        "走着" => vec!["walking"],
        "拿着" => vec!["holding", "carrying"],
        "去过" => vec!["have been to"],
        "看过" => vec!["have seen", "watched"],
        "吃过" => vec!["have eaten", "tried"],
        "做过" => vec!["have done", "experienced"],
        "怎么了" => vec!["what's wrong?", "what happened?"],
        "怎么样" => vec!["how about it?", "how is it?"],
        "怎么呢" => vec!["how come?", "what's up?"],
        "行吗" => vec!["is it okay?", "will that do?"],
        "行了" => vec!["all right", "that's enough"],
        "做了" => vec!["did", "made"],
        "作了" => vec!["did", "wrote"],
        "坐了" => vec!["sat", "took a seat"],
        "做了一些" => vec!["did some", "made some"],
        "做了什么" => vec!["what did you do?", "what happened?"],
        "你说" => vec!["you say", "tell me"],
        "你说呢" => vec!["what do you say?", "what do you think?"],
        "你说得对" => vec!["you're right", "you are correct"],
        "你说过" => vec!["you said", "you mentioned"],
        "也要" => vec!["also want", "also need"],
        "也要注意" => vec!["also need to pay attention", "be careful too"],
        "也有" => vec!["also have", "also exists"],
        "也会" => vec!["will also", "also can"],
        "也能" => vec!["can also", "also able to"],
        "读文" => vec!["read text", "reading"],
        "读问" => vec!["read and ask"],
        "敲入" => vec!["type in", "enter"],
        "没事" => vec!["it's fine", "no problem"],
        "没事了" => vec!["it's all right now", "no worries"],
        "谢谢你了" => vec!["thank you", "thanks to you"],
        _ => return None,
    };
    Some(out)
}

/// 查出基础词条的英文译文，若没有则尝试拆分
fn lookup_translations_or_decompose(word: &str, glossary: &Glossary) -> Vec<String> {
    let raw = glossary.translations(word, &["en".to_string()], 3);
    if !raw.is_empty() {
        return raw.into_iter().map(|t| t.text).collect();
    }
    // 递归检查是否自身带有语气词
    let nested = translate_particle_phrase(word, glossary);
    if !nested.is_empty() {
        return nested;
    }
    // 尝试拆分成词组
    if let Some(compound) = decompose_and_join(word, glossary) {
        return vec![compound];
    }
    Vec::new()
}

/// 将语气助词语义叠加到基础译文上
fn apply_particle_semantics(bases: &[String], stem: &str, kind: ParticleKind) -> Vec<String> {
    let mut out = Vec::new();
    for base in bases {
        let base_clean = base.trim();
        if base_clean.is_empty() {
            continue;
        }

        match kind {
            ParticleKind::Completed => {
                // 了: 过去时 / 完成体
                if stem == "拿到" {
                    out.push("got it".to_string());
                    out.push("obtained".to_string());
                } else if stem == "到" {
                    out.push("arrived".to_string());
                    out.push("reached".to_string());
                } else {
                    let past = to_past_tense(base_clean);
                    if !out.contains(&past) {
                        out.push(past);
                    }
                }
            }
            ParticleKind::Question => {
                // 吗 / 么: 疑问句
                if stem.ends_with('了') || stem.ends_with("过") {
                    out.push(format!("have you {}?", base_clean));
                    out.push(format!("did you {}?", base_clean));
                } else if stem == "好" || stem == "可以" || stem == "对" || stem == "行" {
                    out.push(format!("is it {}?", base_clean));
                    out.push(format!("all right?"));
                } else {
                    out.push(format!("{}?", base_clean));
                }
            }
            ParticleKind::ProgressiveOrSoft => {
                // 呢: 进行中 / 语气缓和
                if stem == "好" {
                    out.push("doing well".to_string());
                    out.push("pretty good".to_string());
                } else if stem == "你" {
                    out.push("and you?".to_string());
                    out.push("how about you?".to_string());
                } else {
                    out.push(to_continuous(base_clean));
                }
            }
            ParticleKind::Attributive => {
                // 的: 属性、所属
                if stem == "好" {
                    out.push("okay".to_string());
                    out.push("all right".to_string());
                } else if stem == "真" {
                    out.push("real".to_string());
                    out.push("really".to_string());
                } else {
                    out.push(base_clean.to_string());
                }
            }
            ParticleKind::Complement => {
                // 得: 程度补语
                out.push(format!("so {}", base_clean));
                out.push(format!("can {}", base_clean));
            }
            ParticleKind::Obviousness => {
                // 嘛: 显而易见
                out.push(format!("already {}", base_clean));
                out.push(format!("fine then, {}", base_clean));
            }
            ParticleKind::Suggestion => {
                // 吧: 提议
                out.push(format!("let's {}", base_clean));
                out.push(format!("{}, right?", base_clean));
            }
            ParticleKind::Exclamation => {
                // 啊/呀: 感叹
                out.push(format!("{}!", base_clean));
            }
            ParticleKind::JoyfulCompleted => {
                // 啦: 轻快完成
                let past = to_past_tense(base_clean);
                out.push(format!("{}!", past));
            }
            ParticleKind::Resignation => {
                // 呗
                out.push(format!("just {}", base_clean));
            }
            ParticleKind::Continuous => {
                // 着
                out.push(to_continuous(base_clean));
            }
            ParticleKind::Experiential => {
                // 过: 曾做过
                let past = to_past_tense(base_clean);
                out.push(format!("have {}", past));
            }
        }
    }
    out
}

/// 英语动词转过去时
pub fn to_past_tense(phrase: &str) -> String {
    let lower = phrase.to_ascii_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();
    if words.is_empty() {
        return phrase.to_string();
    }
    let first = words[0];
    let past = match first {
        "get" => "got",
        "obtain" => "obtained",
        "take" => "took",
        "arrive" => "arrived",
        "reach" => "reached",
        "leave" => "left",
        "go" => "went",
        "come" => "came",
        "eat" => "ate",
        "drink" => "drank",
        "see" => "saw",
        "look" => "looked",
        "watch" => "watched",
        "find" => "found",
        "do" => "did",
        "make" => "made",
        "buy" => "bought",
        "sell" => "sold",
        "pay" => "paid",
        "hear" => "heard",
        "listen" => "listened",
        "say" => "said",
        "speak" => "spoke",
        "tell" => "told",
        "write" => "wrote",
        "read" => "read",
        "know" => "knew",
        "think" => "thought",
        "become" => "became",
        "begin" => "began",
        "start" => "started",
        "finish" => "finished",
        "stop" => "stopped",
        "drop" => "dropped",
        "fall" => "fell",
        "break" => "broke",
        "give" => "gave",
        "bring" => "brought",
        "send" => "sent",
        "win" => "won",
        "lose" => "lost",
        "pass" => "passed",
        "meet" => "met",
        "open" => "opened",
        "close" => "closed",
        "walk" => "walked",
        "run" => "ran",
        "fly" => "flew",
        "sit" => "sat",
        "stand" => "stood",
        "sleep" => "slept",
        "wake" => "woke",
        "teach" => "taught",
        "build" => "built",
        "learn" => "learned",
        "understand" => "understood",
        "feel" => "felt",
        "hold" => "held",
        "catch" => "caught",
        "choose" => "chose",
        "forget" => "forgot",
        "remember" => "remembered",
        "change" => "changed",
        "move" => "moved",
        "stay" => "stayed",
        "wait" => "waited",
        "try" => "tried",
        "use" => "used",
        "show" => "showed",
        "help" => "helped",
        "call" => "called",
        "ask" => "asked",
        "work" => "worked",
        "play" => "played",
        "live" => "lived",
        "love" => "loved",
        "like" => "liked",
        other => {
            if other.ends_with("ed") {
                return lower;
            } else if other.ends_with('e') {
                return format!("{other}d");
            } else if other.ends_with('y')
                && other.len() > 2
                && !matches!(
                    other.chars().nth(other.len() - 2),
                    Some('a' | 'e' | 'i' | 'o' | 'u')
                )
            {
                return format!("{}ied", &other[..other.len() - 1]);
            } else {
                return format!("{other}ed");
            }
        }
    };
    if words.len() == 1 {
        past.to_string()
    } else {
        format!("{} {}", past, words[1..].join(" "))
    }
}

/// 英语动词转现在进行时
pub fn to_continuous(phrase: &str) -> String {
    let lower = phrase.to_ascii_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();
    if words.is_empty() {
        return phrase.to_string();
    }
    let first = words[0];
    let ing = match first {
        "get" => "getting",
        "run" => "running",
        "stop" => "stopping",
        "swim" => "swimming",
        "sit" => "sitting",
        "win" => "winning",
        "drop" => "dropping",
        "plan" => "planning",
        "begin" => "beginning",
        "die" => "dying",
        "lie" => "lying",
        "tie" => "tying",
        other => {
            if other.ends_with("ing") {
                return lower;
            } else if other.ends_with("ee") || other.ends_with("ye") || other.ends_with("oe") {
                return format!("{other}ing");
            } else if other.ends_with('e') {
                return format!("{}ing", &other[..other.len() - 1]);
            } else {
                return format!("{other}ing");
            }
        }
    };
    if words.len() == 1 {
        ing.to_string()
    } else {
        format!("{} {}", ing, words[1..].join(" "))
    }
}

/// 复合词贪婪切分并拼接译文
pub fn decompose_and_join(phrase: &str, glossary: &Glossary) -> Option<String> {
    let chars: Vec<char> = phrase.chars().collect();
    let n = chars.len();
    if n < 2 {
        return None;
    }

    // 尝试切成两段（或三段）
    // 优先前缀长匹配：如 那到了 -> "那" + "到了"
    for split_point in 1..n {
        let left: String = chars[..split_point].iter().collect();
        let right: String = chars[split_point..].iter().collect();

        let left_trans = lookup_single_or_particle(&left, glossary);
        let right_trans = lookup_single_or_particle(&right, glossary);

        if let (Some(l), Some(r)) = (left_trans, right_trans) {
            return Some(format!("{l} {r}"));
        }
    }

    // 如果三段切分
    if n >= 3 {
        for p1 in 1..n - 1 {
            for p2 in p1 + 1..n {
                let s1: String = chars[..p1].iter().collect();
                let s2: String = chars[p1..p2].iter().collect();
                let s3: String = chars[p2..].iter().collect();

                let t1 = lookup_single_or_particle(&s1, glossary);
                let t2 = lookup_single_or_particle(&s2, glossary);
                let t3 = lookup_single_or_particle(&s3, glossary);

                if let (Some(a), Some(b), Some(c)) = (t1, t2, t3) {
                    return Some(format!("{a} {b} {c}"));
                }
            }
        }
    }

    None
}

fn lookup_single_or_particle(word: &str, glossary: &Glossary) -> Option<String> {
    if let Some(fixed) = special_particle_matches(word) {
        if let Some(first) = fixed.first() {
            return Some(first.to_string());
        }
    }
    let direct = glossary.translations(word, &["en".to_string()], 1);
    if let Some(first) = direct.first() {
        return Some(first.text.clone());
    }
    let particle = translate_particle_phrase(word, glossary);
    if let Some(first) = particle.first() {
        return Some(first.clone());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_past_tense_conversions() {
        assert_eq!(to_past_tense("get"), "got");
        assert_eq!(to_past_tense("obtain"), "obtained");
        assert_eq!(to_past_tense("arrive"), "arrived");
        assert_eq!(to_past_tense("leave"), "left");
        assert_eq!(to_past_tense("walk"), "walked");
    }

    #[test]
    fn test_special_matches() {
        let nadaole = special_particle_matches("拿到了").unwrap();
        assert!(nadaole.contains(&"got it"));

        let chidema = special_particle_matches("吃了吗").unwrap();
        assert!(chidema.contains(&"have you eaten?"));

        let haode = special_particle_matches("好的").unwrap();
        assert!(haode.contains(&"okay"));

        let nadadaole = special_particle_matches("那到了").unwrap();
        assert!(nadadaole.contains(&"then arrived"));
    }
}
