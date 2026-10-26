import io
overrides = set()
with io.open("data/en_overrides.tsv", encoding="utf-8") as fh:
    for line in fh:
        if line.strip() and not line.startswith("#"):
            overrides.add(line.split("\t")[0].strip())
en_cases = []
with io.open("tools/eval_cases.tsv", encoding="utf-8") as fh:
    for line in fh:
        if line.startswith("en2zh\t"):
            en_cases.append(line.split("\t")[1].strip())
covered = [w for w in en_cases if w in overrides]
print("en2zh 用例 %d 条，其中 %d 条由人工校正表兜底（其余由 CC-CEDICT 提供）" % (len(en_cases), len(covered)))
