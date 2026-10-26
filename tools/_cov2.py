import io
want = {"hello","anything","what","china","world","understand","thank","teach","learn","write","read","phone","beijing","worked","cars","running","books","beautiful","question"}
found = {}
with io.open("data/en2zh.tsv", encoding="utf-8") as fh:
    for line in fh:
        k, _, v = line.rstrip("\n").partition("\t")
        if k in want:
            found[k] = v
for w in sorted(want):
    print(("  OK   " if w in found else "  MISS ") + "%-12s %s" % (w, found.get(w, "")[:70]))
print("\n命中 %d/%d" % (len(found), len(want)))
