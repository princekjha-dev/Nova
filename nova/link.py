import http.client,ipaddress,socket,ssl,tempfile,time
from pathlib import Path
from urllib.parse import urlsplit,urljoin,unquote
from .media import ingest,ROOT

def target(url):
    u=urlsplit(url)
    if u.scheme!='https' or not u.hostname or u.username or u.password or u.port not in (None,443):raise ValueError('Use a public direct HTTPS link on port 443')
    addrs=socket.getaddrinfo(u.hostname,443,type=socket.SOCK_STREAM)
    ips=[ipaddress.ip_address(row[4][0]) for row in addrs]
    if not ips or any(not ip.is_global or ip.is_multicast or ip.is_reserved or ip.sixtofour or ip.teredo for ip in ips if ip.version==6) or any(not ip.is_global or ip.is_multicast or ip.is_reserved for ip in ips):raise ValueError('Destination is not public unicast')
    return u,str(ips[0])
def download(url):
    deadline=time.monotonic()+60
    for _ in range(5):
        u,ip=target(url);conn=http.client.HTTPSConnection(u.hostname,timeout=8)
        try:
            with socket.create_connection((ip,443),timeout=8) as raw:
                conn.sock=ssl.create_default_context().wrap_socket(raw,server_hostname=u.hostname)
                conn.request('GET',(u.path or '/')+('?'+u.query if u.query else ''),headers={'Host':u.hostname,'Accept-Encoding':'identity'})
                res=conn.getresponse()
                if res.status in (301,302,303,307,308):
                    url=urljoin(url,res.getheader('Location',''));continue
                if res.status!=200:raise ValueError(f'Source returned HTTP {res.status}')
                with tempfile.TemporaryFile(dir=ROOT/'jobs') as tmp:
                    while chunk:=res.read1(262144):
                        if tmp.tell()+len(chunk)>256*1024**2 or time.monotonic()>deadline:raise ValueError('Link exceeded 256 MiB or 60 seconds')
                        tmp.write(chunk)
                    tmp.seek(0);return ingest(tmp,Path(unquote(u.path)).name)
        finally:conn.close()
    raise ValueError('Too many redirects')
