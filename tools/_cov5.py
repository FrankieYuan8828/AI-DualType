import io
overrides = set()
with io.open("data/en_overrides.tsv", encoding="utf-8") as fh:
    for line in fh:
        if line.strip() and not line.startswith("#"):
            overrides.add(line.split("\t")[0].strip())
out = []
with io.open("tools/eval_cases.tsv", encoding="utf-8") as fh:
    for line in fh:
        if line.startswith("en2zh\t"):
            word = line.split("\t")[1].strip()
            if word in overrides:
                continue
        out.append(line)
io.open("tools/_eval_no_override.tsv", "w", encoding="utf-8", newline="\n").write("".join(out))
print("独立评测集（去掉人工校正词）用例数:", sum(1 for l in out if l.strip() and not l.startswith("#")))
