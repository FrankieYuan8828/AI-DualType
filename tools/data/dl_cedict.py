import http.client, os, ssl, time, sys

URL_HOST = "www.mdbg.net"
PATH = "/chinese/export/cedict/cedict_1_0_ts_utf-8_mdbg.txt.gz"
OUT = "cedict.txt.gz"
TOTAL = None

def fetch(start):
    ctx = ssl.create_default_context()
    conn = http.client.HTTPSConnection(URL_HOST, timeout=60, context=ctx)
    headers = {"User-Agent": "Mozilla/5.0", "Accept-Encoding": "identity"}
    if start:
        headers["Range"] = f"bytes={start}-"
    conn.request("GET", PATH, headers=headers)
    resp = conn.getresponse()
    if resp.status not in (200, 206):
        raise RuntimeError(f"HTTP {resp.status}")
    return resp

for attempt in range(1, 41):
    have = os.path.getsize(OUT) if os.path.exists(OUT) else 0
    try:
        resp = fetch(have)
        with open(OUT, "ab") as fh:
            while True:
                chunk = resp.read(65536)
                if not chunk:
                    break
                fh.write(chunk)
        size = os.path.getsize(OUT)
        print(f"attempt {attempt}: {size} bytes", flush=True)
        # gzip 完整性：尾部必须能解压到 EOF，用最后一块试探
        import gzip
        try:
            with gzip.open(OUT, "rb") as g:
                while g.read(1 << 20):
                    pass
            print("gzip OK, size =", size)
            break
        except EOFError:
            print("truncated, resume...", flush=True)
            continue
    except Exception as exc:
        print(f"attempt {attempt} failed: {exc}", flush=True)
        time.sleep(2)
else:
    sys.exit("download failed after retries")
