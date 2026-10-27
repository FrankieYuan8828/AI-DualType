#!/usr/bin/env python3
"""从 CC-CEDICT 生成两个运行时数据包。

输入：
  tools/data/cedict.ts.u8   CC-CEDICT（`繁 简 [pinyin] /gloss1/gloss2/`）
  data/dict.txt             我们的拼音词库（用来限定规模 + 给反查排序）
  data/glossary.tsv         手写多语词表（en/de/ja/fr/es；优先级最高）

输出：
  data/dict_en.tsv   `简体词<TAB>gloss1;gloss2`        中 → 英（候选子项用）
  data/en2zh.tsv     `英文词<TAB>中文1;中文2;中文3`     英 → 中（输入英文反查用）

两个文件都按 key 排序、每行 `key<TAB>value`，Rust 侧用二分查找直接读，
不建 HashMap —— 每个宿主进程只多 ~5 MB 常驻，而不是几十 MB。
"""

import io
import os
import re
import sys
import unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "data")
CEDICT_GZ = os.path.join(DATA, "cedict.txt.gz")
CEDICT = os.path.join(DATA, "cedict.u8")
OUT_EN = os.path.abspath(os.path.join(HERE, "..", "data", "dict_en.tsv"))
OUT_REV = os.path.abspath(os.path.join(HERE, "..", "data", "en2zh.tsv"))
DICT = os.path.abspath(os.path.join(HERE, "..", "data", "dict.txt"))
GLOSSARY = os.path.abspath(os.path.join(HERE, "..", "data", "glossary.tsv"))
OVERRIDES = os.path.abspath(os.path.join(HERE, "..", "data", "en_overrides.tsv"))

# 这些释义是词典元信息，不是翻译
NOISE = re.compile(
    r"^(variant of|old variant of|archaic variant of|used in|see |surname |abbr\.|also pr\.|"
    r"Taiwan pr\.|Japanese variant|erhua variant)",
    re.IGNORECASE,
)
CJK = re.compile(r"^[\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaffA-Za-z0-9 ]+$")
EN_WORD = re.compile(r"^[a-z][a-z'\- ]{0,24}$")


def read_cedict():
    if not os.path.exists(CEDICT):
        import gzip
        if not os.path.exists(CEDICT_GZ):
            sys.exit("缺少 %s" % CEDICT_GZ)
        raw = gzip.open(CEDICT_GZ, "rt", encoding="utf-8").read()
        io.open(CEDICT, "w", encoding="utf-8", newline="\n").write(raw)
    with io.open(CEDICT, encoding="utf-8") as fh:
        return fh.read().splitlines()


def norm_en(text):
    """把词典释义归一化成「用户会打的英文词」。

    CEDICT 里大量释义是 `to understand` / `Beijing, capital of China` /
    `(bound form) book`，直接当 key 用户永远打不到。这里做四件事：
    去括号、只取第一个义项短语、剥掉 `to/a/an/the` 前缀、去掉尾部标点。
    """
    text = text.lower().strip()
    text = re.sub(r"\([^)]*\)", "", text).strip()
    text = re.split(r"[,;:]", text)[0].strip()
    text = re.sub(r"^(to|a|an|the|be|being)\s+", "", text).strip()
    return text.strip(" .'")


STOPWORDS = {
    "to", "a", "an", "the", "of", "in", "on", "at", "for", "with", "and", "or",
    "be", "is", "are", "was", "were", "as", "by", "from", "that", "this", "it",
    "one", "some", "not", "used", "etc", "sb", "sth", "cl",
}


def head_token(text):
    """英文名词短语是「中心词在前」：phone call -> phone，Beijing municipality -> beijing。"""
    first = text.split(" ")[0].strip(" .'")
    if len(first) >= 3 and first not in STOPWORDS:
        return first
    return None


NON_INFLECTED = {
    "because", "both", "but", "and", "or", "for", "with", "from", "that", "this",
    "they", "them", "their", "when", "where", "why", "how", "what", "who", "which",
    "always", "never", "maybe", "however", "please", "sorry", "thanks", "hello",
    "yes", "no", "ok", "into", "onto", "upon", "about", "above", "after", "before",
    "under", "over", "between", "among", "through", "during", "without", "again",
}

def inflections(base):
    """给常用词补词形，让 worked/working/books/cars 也能查到。"""
    out = []
    if len(base) < 3 or " " in base or base in NON_INFLECTED:
        return out
    if base.endswith("e"):
        out += [base + "d", base + "s", base[:-1] + "ing"]
    elif base.endswith("y") and len(base) > 3:
        out += [base[:-1] + "ies", base[:-1] + "ied", base + "ing"]
    else:
        out += [base + "ed", base + "s", base + "ing", base + "es"]
    return [w for w in out if w != base]


def clean_gloss(text):
    text = text.strip()
    text = re.sub(r"\([^)]*\)", "", text)          # 去掉括号补充说明
    text = unicodedata.normalize("NFC", text)
    text = text.replace("CL:", "").strip()
    return text.strip(" .,;:")


def load_dict_words():
    """词库里的词 -> 最小频率序（rank 列，越小越常用）。

    注意 dict.txt 是按拼音 key 排序的，行号**不是**频率；必须读第三列 rank，
    否则反查会按拼音字母序出词（`china` 会给出「东大/东土」而不是「中国」）。
    """
    order = {}
    with io.open(DICT, encoding="utf-8") as fh:
        for line in fh:
            parts = line.rstrip("\n").split("\t")
            if len(parts) < 3:
                continue
            word, rank = parts[1], parts[2]
            try:
                rank = int(rank)
            except ValueError:
                continue
            if word not in order or rank < order[word]:
                order[word] = rank
    return order


def load_handwritten():
    """手写词表：word -> {lang: text}，同时作为反查的补充来源。"""
    table = {}
    if not os.path.exists(GLOSSARY):
        return table
    with io.open(GLOSSARY, encoding="utf-8") as fh:
        for line in fh:
            line = line.rstrip("\n")
            if not line or line.startswith("#"):
                continue
            parts = line.split("\t")
            if len(parts) < 3:
                continue
            word, lang, text = parts[0].strip(), parts[1].strip(), parts[2].strip()
            if not word or not lang or not text:
                continue
            table.setdefault(word, {})[lang] = text
    return table


def build():
    lines = read_cedict()
    dict_words = load_dict_words()
    handwritten = load_handwritten()
    print("CEDICT 行数        : %d" % len(lines))
    print("词库词条           : %d" % len(dict_words))
    print("手写词条           : %d" % len(handwritten))

    # ---------- 中 → 英 ----------
    en_by_word = {}
    for word, langs in handwritten.items():
        if "en" in langs:
            en_by_word[word] = [langs["en"]]

    for line in lines:
        if line.startswith("#") or "[" not in line:
            continue
        try:
            head, _, rest = line.partition("[")
            _, _, gloss_part = rest.partition("]")
            trad, simp = head.split(" ")[0], head.split(" ")[1]
        except (ValueError, IndexError):
            continue
        word = simp.strip()
        if not CJK.match(word):
            continue
        # CEDICT 里同一条目内部还会用 `;` 分隔多个义项（如 `/hello; hi/`），要再拆一次
        raw_glosses = []
        for chunk in gloss_part.strip().split("/"):
            raw_glosses.extend(chunk.split(";"))
        glosses = [clean_gloss(g) for g in raw_glosses]
        glosses = [g for g in glosses if g and not NOISE.match(g) and len(g) <= 40]
        if not glosses:
            continue
        # 只保留词库里真的会出现的词（否则候选里永远用不上，白占体积）
        if word not in dict_words and word not in handwritten:
            continue
        bucket = en_by_word.setdefault(word, [])
        seen = {g.lower() for g in bucket}
        for g in glosses:
            if g.lower() not in seen and len(bucket) < 3:
                bucket.append(g)
                seen.add(g.lower())

    rows = []
    for word in sorted(en_by_word):
        # 逐条归一化（去掉 `to ` 之类），去重后最多保留 2 条，
        # 这样「回答」显示的是 `reply / answer` 而不是只有 `to reply`。
        cleaned = []
        for gloss in en_by_word[word]:
            text = norm_en(gloss)
            if not text:
                continue
            if any(text.lower() == existing.lower() for existing in cleaned):
                continue
            cleaned.append(text)
            if len(cleaned) >= 2:
                break
        if cleaned:
            rows.append("%s\t%s" % (word, ";".join(cleaned)))
    io.open(OUT_EN, "w", encoding="utf-8", newline="\n").write("\n".join(rows) + "\n")
    print("dict_en.tsv        : %d 词, %.1f MB" % (len(rows), os.path.getsize(OUT_EN) / 1048576.0))

    # ---------- 英 → 中 ----------
    # 归一化释义 + 位置权重（越靠前的释义越能代表这个词）+ 全量中文覆盖。
    # 注释里保留踩坑记录：不归一化时 `understand`/`thank` 都查不到，
    # 不加位置权重时 `anything` 会先给「什么」而不是「任何东西」。
    zh_by_en = {}
    for line in lines:
        if line.startswith("#") or "[" not in line:
            continue
        try:
            head, _, rest = line.partition("[")
            _, _, gloss_part = rest.partition("]")
            trad, simp = head.split(" ")[0], head.split(" ")[1]
        except (ValueError, IndexError):
            continue
        word = simp.strip()
        if not CJK.match(word):
            continue
        raw = []
        for chunk in gloss_part.strip().split("/"):
            raw.extend(chunk.split(";"))
        for pos, gloss in enumerate(raw):
            en = norm_en(gloss)
            if not EN_WORD.match(en) or NOISE.match(en):
                continue
            weight = 3 if pos == 0 else (2 if pos == 1 else 1)
            if len(word) >= 2:
                weight += 1

            keys = [(en, weight)]
            if " " in en:
                head = head_token(en)
                if head:
                    # 中心词单独成键，权重略低（Beijing municipality -> beijing）
                    keys.append((head, max(1, weight - 1)))
            for key, w in keys:
                bucket = zh_by_en.setdefault(key, {})
                if len(bucket) < 60 and w > bucket.get(word, 0):
                    bucket[word] = w

    # 人工校正表：最高权重，覆盖 CEDICT 对超高频词的偏颇排序
    if os.path.exists(OVERRIDES):
        with io.open(OVERRIDES, encoding="utf-8") as fh:
            for line in fh:
                line = line.rstrip("\n")
                if not line or line.startswith("#"):
                    continue
                parts = line.split("\t")
                if len(parts) < 2:
                    continue
                en = norm_en(parts[0])
                # 行内顺序即优先级：第一条 99，第二条 98……保证「是的」压过「对」
                for rank_in_line, zh in enumerate(parts[1].split(";")):
                    zh = zh.strip()
                    if not en or not zh or not CJK.match(zh):
                        continue
                    bucket = zh_by_en.setdefault(en, {})
                    weight = 99 - rank_in_line
                    bucket[zh] = max(bucket.get(zh, 0), weight)

    # 手写词表补进去：hallo -> 你好（德语也能反查）；手写优先，权重给最高
    for word, langs in handwritten.items():
        for lang, text in langs.items():
            key = norm_en(text)
            if EN_WORD.match(key) and len(key) >= 3:
                bucket = zh_by_en.setdefault(key, {})
                bucket[word] = max(bucket.get(word, 0), 9)

    # 词形变体：只给常用基础词补（避免文件膨胀），如 work -> worked/working/works
    extra = {}
    for en, targets in zh_by_en.items():
        best = max(targets.values())
        if best < 3:
            continue
        if min(dict_words.get(w, 10**9) for w in targets) > 30000:
            continue
        for variant in inflections(en):
            if variant not in zh_by_en and variant not in extra:
                extra[variant] = dict(targets)
    for en, targets in extra.items():
        zh_by_en.setdefault(en, targets)

    # 已经存在的词形键（如 worked 被别的释义占用）也要并入基础词的释义，
    # 否则会出现「worked -> 熟料/激昂」这种垃圾结果。
    merge = {}
    for base, targets in zh_by_en.items():
        for variant in inflections(base):
            if variant in zh_by_en:
                bucket = merge.setdefault(variant, {})
                for w, weight in targets.items():
                    if weight > bucket.get(w, 0):
                        bucket[w] = weight
    for key, add in merge.items():
        bucket = zh_by_en[key]
        for w, weight in add.items():
            if weight > bucket.get(w, 0):
                bucket[w] = weight
    print("词形变体补充      : %d (+%d 合并)" % (len(extra), len(merge)))

    def order_key(item):
        word, weight = item
        # 综合分：释义位置权重为主，词频把「生僻词」压下去。
        # 这一条修的是实测问题：anything 曾经给到生僻的「无论何事」而不是常用词。
        rank = min(dict_words.get(word, 10**9), 20000)
        return -(weight * 10000 - rank)

    rev_rows = []
    for en in sorted(zh_by_en):
        ranked = sorted(zh_by_en[en].items(), key=order_key)[:6]
        if ranked:
            # 带上真实词频 rank：候选排序时反查词和拼音词才能公平比较
            # （否则 `china` 会被拼音「chi na」的「吃奶」抢走，`中国` 排不上）
            # 词库外的词给 30000 而不是 1e9：人工校正的答案是「我们确定它正确」，
            # 用 1e9 会让它在引擎打分里永远垫底（yes 曾因此出「对」而不是「是的」）。
            payload = ";".join(
                "%s:%d:%d" % (w, min(dict_words.get(w, 30000), 30000), weight)
                for w, weight in ranked
            )
            rev_rows.append("%s\t%s" % (en, payload))
    io.open(OUT_REV, "w", encoding="utf-8", newline="\n").write("\n".join(rev_rows) + "\n")
    print("en2zh.tsv          : %d 词, %.1f MB" % (len(rev_rows), os.path.getsize(OUT_REV) / 1048576.0))

    for probe in ("hello", "anything", "china", "phone", "beijing", "worked", "cars", "understand"):
        hits = zh_by_en.get(probe)
        if hits:
            best = [w for w, _ in sorted(hits.items(), key=order_key)[:4]]
            print("反查 %-10s -> %s" % (probe, ", ".join(best)))
        else:
            print("反查 %-10s -> (miss)" % probe)
    for probe in ("你好", "中国", "随便", "谢谢"):
        print("正向 %-6s -> %s" % (probe, "; ".join(en_by_word.get(probe, [])[:2])))


if __name__ == "__main__":
    build()
