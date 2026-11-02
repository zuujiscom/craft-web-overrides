# Compares two CRAFTS_ROOT trees (bash crafts-sync vs the Rust one): same names and links, same content (.gz/.br compared
# decompressed), reports mode differences. Run: python3 -I compare-roots.py rootA rootB. index.html is compared separately
# (the icon ?v= mtimes differ): diff <(sed -E "s/\?v=[0-9]+//g" A/www/index.html) <(sed -E "s/\?v=[0-9]+//g" B/www/index.html)
import os,sys,hashlib,gzip,subprocess
A,B=sys.argv[1:3]
def walk(r):
    out={}
    for d,ds,fs in os.walk(r):
        for n in ds+fs:
            p=os.path.join(d,n); out[os.path.relpath(p,r)]=p
    return out
a,b=walk(A),walk(B)
print("only A:",sorted(set(a)-set(b))[:10]); print("only B:",sorted(set(b)-set(a))[:10]); print("files",len(a),len(b))
def content(p):
    if os.path.islink(p): return ("link",os.readlink(p))
    if os.path.isdir(p): return ("dir",)
    data=open(p,"rb").read()
    if p.endswith(".gz"): data=gzip.decompress(data)
    elif p.endswith(".br"): data=subprocess.run(["brotli","-dc",p],capture_output=True).stdout
    return ("f",hashlib.sha256(data).hexdigest())
bad=0
for k in sorted(set(a)&set(b)):
    if k in ("tmp",".lock") or k.endswith("index.html") and k=="www/index.html": continue
    if content(a[k])!=content(b[k]): print("DIFF",k); bad+=1
print("content diffs:",bad)
modes={}
for k in set(a)&set(b):
    if os.path.islink(a[k]): continue
    ma,mb=os.stat(a[k]).st_mode&0o7777,os.stat(b[k]).st_mode&0o7777
    if ma!=mb: modes[(oct(ma),oct(mb))]=modes.get((oct(ma),oct(mb)),0)+1
print("mode differences (bash,rust):",modes)
