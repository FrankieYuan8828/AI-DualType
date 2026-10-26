import io
# 用用户报的那批英文词统计命中和首位命中
keys = {}
with io.open(r"C:\Program Files\MC-TypeEnglish\data\en2zh.tsv", encoding="utf-8") as fh:
    for line in fh:
        if not line.strip(): continue
        k, _, v = line.rstrip("\n").partition("\t")
        keys[k] = v
words = ["anything","something","everything","nothing","someone","everyone","always","never","maybe",
         "because","however","please","sorry","thanks","hello","bye","yes","no","ok","nice","good","bad",
         "big","small","fast","slow","happy","sad","today","tomorrow","yesterday","morning","night","water",
         "fire","money","food","book","phone","computer","friend","family","work","study","run","eat","drink",
         "sleep","read","write","speak","help","buy","sell","open","close","start","stop","beijing","china",
         "world","understand","question","answer","beautiful","teacher","student","school"]
hit = sum(1 for w in words if w in keys)
print("总词数 %d，命中 %d（%d%%）" % (len(words), hit, hit*100//len(words)))
miss = [w for w in words if w not in keys]
print("未命中:", miss)
print("\n首位示例：")
for w in ["anything","thanks","love","beijing","understand","phone","world","china"]:
    v = keys.get(w,"")
    print("  %-11s -> %s" % (w, v.split(';')[0]))
