import io,json,os,secrets,tempfile,threading,time,uuid,shutil
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from typing import Literal
from fastapi import FastAPI,HTTPException,Request,Response
from fastapi.responses import FileResponse,JSONResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel
from .media import ROOT,atomic,init,ingest,linked,frame,LIMIT
from .model import Project
from .export import export,Canceled
from .vision import detect,track
from .render import ENGINE,cv2
from .link import download

BASE=Path(__file__).resolve().parents[1];init()
app=FastAPI(title='NOVA local media API',docs_url=None,redoc_url=None)
port=int(os.environ.get('NOVA_PORT',8765));host=f'127.0.0.1:{port}'
bootstrap=secrets.token_urlsafe(32);session=secrets.token_urlsafe(32);auth_lock=threading.Lock()
worker=ThreadPoolExecutor(1);events={};records={};preview_lock=threading.Lock();job_lock=threading.RLock()
db=ROOT/'recovery/jobs.json'
if db.exists():
    records=json.loads(db.read_text())
    for v in records.values():
        if v['state'] not in ('Ready','Needs Review','No overlay found','Failed','Canceled'):v.update(state='Failed',error='Interrupted by restart. Retry from the export in-point.')
def persist():
    with job_lock:atomic(db,records)
@app.middleware('http')
async def auth(req,call_next):
    if req.headers.get('host')!=host or (req.headers.get('origin') and req.headers['origin']!='http://'+host):return Response(status_code=403)
    if (req.url.path.startswith('/api/') or req.url.path=='/openapi.json') and req.url.path!='/api/session' and not secrets.compare_digest(req.cookies.get('nova',''),session):return JSONResponse({'detail':'Open the launch URL printed by NOVA to start this session.'},401)
    if req.method=='POST' and req.url.path!='/api/import' and (not req.headers.get('content-length','').isdigit() or int(req.headers['content-length'])>16*1024**2):return Response(status_code=413)
    response=await call_next(req);response.headers['X-Content-Type-Options']='nosniff';response.headers['Referrer-Policy']='no-referrer';response.headers['Content-Security-Policy']="default-src 'self'; img-src 'self' blob: data:; media-src 'self' blob:; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'; object-src 'none'"
    return response
@app.exception_handler(ValueError)
@app.exception_handler(OSError)
def bad(req,exc):return JSONResponse({'detail':str(exc)},400)
class Token(BaseModel):token:str
@app.post('/api/session')
def login(req:Token,response:Response):
    global bootstrap
    with auth_lock:
        if not bootstrap or not secrets.compare_digest(req.token,bootstrap):raise HTTPException(403,'Expired launch credential')
        bootstrap=''
    response.set_cookie('nova',session,httponly=True,samesite='strict');return {'ok':True}
@app.get('/api/status')
def status():return dict(engine=ENGINE,opencv=cv2 is not None)
def item(pair):return dict(project=pair[0],meta=pair[1])
@app.post('/api/import')
async def upload(req:Request,name:str):
    from starlette.concurrency import run_in_threadpool
    with tempfile.TemporaryFile(dir=ROOT/'jobs') as tmp:
        async for chunk in req.stream():
            if tmp.tell()+len(chunk)>LIMIT or shutil.disk_usage(ROOT).free<len(chunk)+64*1024**2:raise HTTPException(413,'Insufficient space or file exceeds 4 GiB')
            tmp.write(chunk)
        tmp.seek(0);return item(await run_in_threadpool(ingest,tmp,name))
class Link(BaseModel):url:str
@app.post('/api/link')
def url(req:Link):return item(download(req.url))
@app.post('/api/project')
def load(p:Project):return item((p,linked(p)[1]))
@app.post('/api/frame')
def render(p:Project,t:int=0,draft:bool=False,before:bool=False):
    with preview_lock:
        out=io.BytesIO();frame(p,t,draft,before).save(out,'PNG',compress_level=1);return Response(out.getvalue(),media_type='image/png')
@app.post('/api/recovery')
def save(p:Project):
    linked(p);atomic(ROOT/'recovery'/f'{p.source}.json',p.model_dump());return {'ok':True}
@app.get('/api/recovery')
def recover():return [json.loads(f.read_text()) for f in sorted((ROOT/'recovery').glob('*.json'),key=lambda f:f.stat().st_mtime,reverse=True) if len(f.stem)==64]
class Task(BaseModel):
    project:Project
    action:Literal['auto','track','preview','export']
    mask:str=''
    anchor:int=0
@app.post('/api/jobs')
def submit(task:Task):
    linked(task.project);ident=uuid.uuid4().hex;event=threading.Event();events[ident]=event
    row=dict(id=ident,state='Queued',progress=0,action=task.action,project=task.project.model_dump(),error='')
    with job_lock:records[ident]=row;persist()
    def work():
        last=0
        def progress(stage,f):
            nonlocal last
            if time.monotonic()-last>.15 or stage!=row.get('stage'):
                row.update(stage=stage,progress=round(f,4));last=time.monotonic()
        try:
            if event.is_set():raise Canceled('Canceled')
            p=task.project
            if task.action=='auto':
                row.update(state='Analyzing');masks=detect(p,event,progress);p=p.model_copy(update={'masks':[*p.masks,*masks]});row['project']=p.model_dump()
                if not masks:row.update(state='No overlay found');return
            if task.action=='track':
                row.update(state='Tracking');row['suggestion']=track(p,task.mask,task.anchor,event,progress)
            else:
                preview=task.action!='export';row.update(state='Rendering Preview' if preview else 'Exporting')
                path=export(p,ROOT/'outputs',preview,event,progress);row['output']=path.name
            row.update(state='Needs Review' if task.action=='auto' else 'Ready',progress=1)
        except Exception as exc:row.update(state='Canceled' if event.is_set() else 'Failed',error=str(exc))
        finally:persist();events.pop(ident,None)
    worker.submit(work);return {'id':ident}
@app.get('/api/jobs')
def jobs():
    with job_lock:return list(records.values())
@app.post('/api/jobs/{ident}/cancel')
def cancel(ident:str):
    if ident in events:events[ident].set()
    return {'ok':True}
@app.get('/api/output/{ident}')
def output(ident:str,inline:bool=False):
    name=records.get(ident,{}).get('output')
    if not name:raise HTTPException(404,'Output unavailable')
    return FileResponse(ROOT/'outputs'/Path(name).name,filename=name,content_disposition_type='inline' if inline else 'attachment')
@app.get('/api/docs')
def docs():return Response((BASE/'README.md').read_text(),media_type='text/plain')
app.mount('/',StaticFiles(directory=BASE/'dist',html=True,check_dir=False))
