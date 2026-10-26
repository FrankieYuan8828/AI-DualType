import io
keys = {}
with io.open(r"C:\Program Files\MC-TypeEnglish\data\en2zh.tsv", encoding="utf-8") as fh:
    for line in fh:
        if not line.strip():
            continue
        k, _, v = line.rstrip("\n").partition("\t")
        keys[k] = v
print("已安装包英文词头数:", len(keys))
probe = ["hello","anything","what","world","computer","beautiful","understand","question","answer",
         "water","book","love","run","time","people","work","money","friend","study","beijing",
         "thank","sorry","morning","night","happy","food","school","teacher","student","phone",
         "books","running","worked","cars","teach","learn","write","read","sleep","travel"]
hit = 0
for w in probe:
    v = keys.get(w)
    if v:
        hit += 1
        print("  OK   %-12s %s" % (w, v[:60]))
    else:
        print("  MISS %-12s" % w)
print("\n命中 %d/%d = %d%%" % (hit, len(probe), hit * 100 // len(probe)))
