#!/usr/bin/env python3
"""Build the runtime pinyin dictionary for MC-TypeEnglish.

Inputs (downloaded into tools/data/):

  essay.txt           rime/rime-essay word frequency list   -> `词<TAB>权重`  (trad.)
  phrase_pinyin.txt   mozillazg/phrase-pinyin-data          -> `词: pīn yīn`  (simp.)
  char_pinyin.txt     mozillazg/pinyin-data                 -> `U+4F60: nǐ  # 你`
  TSCharacters.txt    OpenCC traditional -> simplified       -> `國<TAB>国`

Output: data/dict.txt — UTF-8, tab separated, one entry per line:

    <toneless-pinyin-key>\t<word>\t<rank>

`rank` is the position when all entries are sorted by descending frequency
weight, so lower is better. Entries are emitted in rank order, grouped by key,
letting the Rust side binary-search a key prefix and read candidates in order.

Words missing from the curated phrase list get their pinyin *composed* from the
per-character table and carry a large rank penalty, so an ambiguous polyphone
reading can never outrank a real curated entry.
"""

import io
import os
import re
import sys
import unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "data")
OUT = os.path.abspath(os.path.join(HERE, "..", "data", "dict.txt"))

ESSAY = os.path.join(DATA, "essay.txt")
PHRASE = os.path.join(DATA, "phrase_pinyin.txt")
CHAR = os.path.join(DATA, "char_pinyin.txt")
T2S = os.path.join(DATA, "TSCharacters.txt")
LUNA = os.path.join(DATA, "luna_pinyin.dict.yaml")

MAX_PER_KEY = 10
MAX_WORD_LEN = 6   # 允许更长的整词（如「中华人民共和国」），长串优先整词匹配
# Frequency floors keep the bundle small: the essay list has ~440k words but
# half of them are rare proper nouns nobody types. Single characters stay in at
# a much lower floor because character-by-character typing needs them.
MIN_WEIGHT_PHRASE = 500
MIN_WEIGHT_CHAR = 20

WORD_RE = re.compile(r"^[\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff]{1,%d}$" % MAX_WORD_LEN)
PINYIN_RE = re.compile(r"^[a-z]+(?: [a-z]+)*$")

UMLAUT = {"ü": "v", "ǖ": "v", "ǘ": "v", "ǚ": "v", "ǜ": "v"}


def toneless(pinyin):
    """`nǐ hǎo` / `lü4 se4` -> `ni hao` / `lv se`."""
    text = pinyin.strip().lower().replace("u:", "v")
    for src, dst in UMLAUT.items():
        text = text.replace(src, dst)
    text = re.sub(r"[0-9]", "", text)
    text = "".join(c for c in unicodedata.normalize("NFD", text)
                   if unicodedata.category(c) != "Mn")
    text = text.replace("\u00fc", "v")
    return re.sub(r"\s+", " ", text).strip()


def read_lines(path):
    with io.open(path, "r", encoding="utf-8") as fh:
        return fh.read().splitlines()


def load_t2s():
    table = {}
    for line in read_lines(T2S):
        if not line or line.startswith("#"):
            continue
        parts = line.split("\t")
        if len(parts) < 2:
            continue
        trad, simp = parts[0].strip(), parts[1].split()[0].strip()
        if trad and simp:
            table[trad] = simp
    return table


def to_simplified(word, t2s):
    return "".join(t2s.get(ch, ch) for ch in word)


def load_phrases(t2s):
    """word -> list of toneless keys (curated, high confidence)"""
    table = {}
    for line in read_lines(PHRASE):
        if not line or line.startswith("#") or ":" not in line:
            continue
        word, _, raw = line.partition(":")
        word = to_simplified(word.strip(), t2s)
        if not WORD_RE.match(word):
            continue
        plain = toneless(raw)
        if not PINYIN_RE.match(plain):
            continue
        table.setdefault(word, []).append(plain.replace(" ", ""))
    return table


def load_chars(t2s):
    """char -> (primary syllable, extra syllables)"""
    primary, extra = {}, {}
    for line in read_lines(CHAR):
        if not line or line.startswith("#") or ":" not in line:
            continue
        code, _, rest = line.partition(":")
        rest = rest.split("#", 1)[0].strip()
        if not rest:
            continue
        try:
            ch = chr(int(code.strip().replace("U+", ""), 16))
        except ValueError:
            continue
        ch = t2s.get(ch, ch)
        readings = [toneless(p) for p in rest.split(",") if p.strip()]
        readings = [r for r in readings if r]
        if not readings:
            continue
        primary.setdefault(ch, readings[0])
        for r in readings[1:]:
            bucket = extra.setdefault(ch, [])
            if r not in bucket:
                bucket.append(r)
    return primary, extra


def load_essay(t2s):
    """word -> best (highest) frequency weight, simplified"""
    weights = {}
    for line in read_lines(ESSAY):
        if not line or line.startswith("#"):
            continue
        parts = line.split("\t")
        if len(parts) < 2:
            continue
        word = to_simplified(parts[0].strip(), t2s)
        if not WORD_RE.match(word):
            continue
        try:
            weight = int(parts[1])
        except ValueError:
            continue
        if weight > weights.get(word, -1):
            weights[word] = weight
    return weights


def load_luna_singles(t2s):
    """char -> set of toneless syllables from luna_pinyin (high quality standard)"""
    table = {}
    if not os.path.exists(LUNA):
        return table
    for line in read_lines(LUNA):
        if not line or line.startswith("#") or "\t" not in line:
            continue
        parts = line.split("\t")
        char = to_simplified(parts[0].strip(), t2s)
        if len(char) == 1 and WORD_RE.match(char):
            pinyin = toneless(parts[1].split()[0])
            if pinyin:
                table.setdefault(char, set()).add(pinyin)
    return table


def main():
    for path in (ESSAY, PHRASE, CHAR, T2S):
        if not os.path.exists(path):
            sys.exit("missing input: %s" % path)

    t2s = load_t2s()
    phrases = load_phrases(t2s)
    primary, extra = load_chars(t2s)
    weights = load_essay(t2s)
    luna_singles = load_luna_singles(t2s)

    # 预设高频关联词汇注入，确保连贯输入与组词精准
    curated_overrides = {
        "读文": ("duwen", 2000),
        "敲入": ("qiaoru", 1800),
        "拿到了": ("nadaole", 4122),
        "那到了": ("nadaole", 4123),
        "那道了": ("nadaole", 4124),
        "你说": ("nishuo", 1630),
        "也要": ("yeyao", 832),
        "吃了么": ("chilema", 3500),
        "浮": ("fu", 5600),
        "好呢": ("haone", 2200),
        "好嘛": ("haoma", 2300),
        "好得": ("haode", 2100),
        "走了": ("zoule", 1900),
    }
    for word, (py, wt) in curated_overrides.items():
        phrases.setdefault(word, []).append(py)
        if word not in weights or weights[word] < wt:
            weights[word] = wt

    print("t2s pairs            : %d" % len(t2s))
    print("essay words (simp)   : %d" % len(weights))
    print("curated phrase words : %d" % len(phrases))
    print("chars with pinyin    : %d" % len(primary))
    print("luna polyphone chars : %d" % len(luna_singles))

    candidates = {}
    for word, weight in weights.items():
        floor = MIN_WEIGHT_CHAR if len(word) == 1 else MIN_WEIGHT_PHRASE
        if weight < floor and word not in phrases:
            continue
        if word in phrases:
            candidates[word] = weight
        elif all(c in primary for c in word):
            candidates[word] = weight
    # Curated phrases absent from the frequency list still deserve a slot, but
    # below everything the frequency list knows about.
    for word in phrases:
        candidates.setdefault(word, 0)

    # Pure frequency order. Curated pinyin wins over composed pinyin for the
    # same word, but the ranking never depends on which source produced it, so
    # 谢谢 does not sit below 泄泻 just because it was composed.
    ordered = sorted(candidates.items(),
                     key=lambda kv: (-kv[1], len(kv[0]), kv[0]))
    print("candidate words      : %d" % len(ordered))

    buckets = {}
    for rank, (word, _weight) in enumerate(ordered):
        if word in phrases:
            keys_with_ranks = [(k, rank) for k in phrases[word]]
        elif len(word) == 1:
            # 严格依据 luna_pinyin 标准库校准多音字，滤除古音古韵与伪多音字（如搞->qiao、月->ru、招->qiao）
            luna_set = luna_singles.get(word, set())
            p = primary[word]
            keys_with_ranks = [(p, rank)]
            for alt in extra.get(word, ()):
                if not luna_set or alt in luna_set:
                    # 次要多音字读音给予加权惩罚（+15000），避免喧宾夺主挤掉该音节的主选字
                    keys_with_ranks.append((alt, rank + 15000))
        else:
            keys_with_ranks = [("".join(primary[c] for c in word), rank)]

        for key, entry_rank in keys_with_ranks:
            if not key:
                continue
            bucket = buckets.setdefault(key, [])
            if any(w == word for w, _ in bucket):
                continue
            bucket.append((word, entry_rank))

    rows = []
    for key in sorted(buckets):
        # 单字音节（如 fu, shi, yi, li 等）容纳最多 100 个单字，翻页可翻出所有常见单字；词汇短语保留前 30 个
        limit = 100 if (" " not in key and len(key) <= 6) else 30
        sorted_bucket = sorted(buckets[key], key=lambda t: t[1])[:limit]
        for word, rank in sorted_bucket:
            rows.append("%s\t%s\t%d" % (key, word, rank))

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with io.open(OUT, "w", encoding="utf-8", newline="\n") as fh:
        fh.write("\n".join(rows))
        fh.write("\n")

    print("keys emitted         : %d" % len(buckets))
    print("rows emitted         : %d" % len(rows))
    print("output               : %s (%.1f MB)" % (OUT, os.path.getsize(OUT) / 1048576.0))
    for probe in ("nihao", "xiexie", "wo", "zhongguo", "wanshang"):
        got = sorted(buckets.get(probe, []), key=lambda t: t[1])[:5]
        print("probe %-9s -> %s" % (probe, ", ".join("%s#%d" % t for t in got)))


if __name__ == "__main__":
    main()
