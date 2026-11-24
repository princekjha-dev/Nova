"""Local candidate ranking, sparse feature tracking and temporal restoration."""
import math, statistics, uuid
from PIL import Image,ImageFilter,ImageChops,ImageOps
from .media import linked,sourceframe,timestamp
from .render import cv2,coverage,rect
from .model import Mask,Box,geometry
if cv2 is not None:import numpy as np

def candidates(im):
    gray=im.convert('L')
    if cv2 is not None:
        a=np.asarray(gray)
        detail=cv2.morphologyEx(a,cv2.MORPH_TOPHAT,np.ones((7,7),np.uint8))
        binary=(detail>28).astype('uint8')*255
        joined=cv2.morphologyEx(binary,cv2.MORPH_CLOSE,np.ones((3,15),np.uint8))
        regions=cv2.connectedComponentsWithStats(joined)[2][1:]
    else:
        detail=ImageChops.subtract(gray,gray.filter(ImageFilter.MedianFilter(5)))
        binary=detail.point(lambda x:255 if x>22 else 0)
        joined=binary.resize((max(1,im.width//5),im.height),Image.Resampling.BOX).point(lambda v:255 if v>10 else 0).filter(ImageFilter.MaxFilter(3))
        from PIL import ImageDraw
        regions=[]
        for y in range(joined.height):
            for x in range(joined.width):
                if joined.getpixel((x,y))==255:
                    ImageDraw.floodfill(joined,(x,y),128)
                    # Only inspect the connected rows near the seed for text.
                    box=joined.crop((0,y,joined.width,min(joined.height,y+60))).point(lambda v:255 if v==128 else 0).getbbox()
                    ImageDraw.floodfill(joined,(x,y),0)
                    if box:regions.append((box[0]*5,y,(box[2]-box[0])*5,box[3],0))
    boxes=[]
    for x,y,w,h,_ in regions:
        x,y,w,h=map(int,(x,y,w,h))
        if 12<=w<im.width*.55 and 4<=h<im.height*.18 and 1.6<=w/h<=25:
            edge=max(abs((x+w/2)/im.width-.5),abs((y+h/2)/im.height-.5))
            density=ImageChops.difference(gray.crop((x,y,x+w,y+h)),gray.filter(ImageFilter.BoxBlur(2)).crop((x,y,x+w,y+h))).resize((1,1)).getpixel((0,0))
            if density<5:continue
            l=max(0,x-4);u=max(0,y-3)
            boxes.append((edge+min(.3,density/100),Box(x=l/im.width,y=u/im.height,w=(min(im.width,x+w+4)-l)/im.width,h=(min(im.height,y+h+3)-u)/im.height)))
    return sorted(boxes,key=lambda a:-a[0])[:8]

def detect(p,event,progress):
    path,m=linked(p);w=min(p.width,560);size=w,max(1,round(w*p.height/p.width))
    step=max(300000,math.ceil(m['duration']/180))
    samples=sorted(set(timestamp(m,t) for t in range(0,m['duration'],step)))
    groups=[]
    for i,t in enumerate(samples):
        if event.is_set():raise InterruptedError('Canceled')
        progress('Analyzing',i/len(samples))
        for score,b in candidates(sourceframe(p,path,m,t,size)):
            best=None;dist=.14
            for g in groups:
                prev=g[-1][2]
                d=math.hypot(b.x-prev.x,b.y-prev.y)+abs(b.w-prev.w)+abs(b.h-prev.h)
                if d<dist and t!=g[-1][0]:best=g;dist=d
            if best is None:groups.append([(t,score,b)])
            else:best.append((t,score,b))
    groups.sort(key=lambda g:-sum(x[1] for x in g))
    masks=[]
    for g in groups[:3]:
        if len(g)<max(1,min(3,len(samples)//2)) or statistics.mean(x[1] for x in g)<.48:continue
        ranges=[]
        for t,_,b in g:
            end=min(m['duration'],t+step)
            if ranges and t<=ranges[-1][1]+20000:ranges[-1][1]=end
            else:ranges.append([t,end])
        masks.append(Mask(id=uuid.uuid4().hex,name=f'Candidate {len(masks)+1}',box=g[0][2],ranges=ranges,keys=[dict(**b.model_dump(),t=t) for t,_,b in g]))
    return masks

def track(p,ident,anchor,event,progress):
    if cv2 is None:raise ValueError('Region tracking requires OpenCV. Run setup-ubuntu.sh.')
    mask=next(m for m in p.masks if m.id==ident)
    path,meta=linked(p);w=min(720,p.width);size=w,max(1,round(w*p.height/p.width))
    ticks=[t for t in meta['ticks'] if anchor<=t<p.interval[1]]
    if not ticks:raise ValueError('No frames after the anchor')
    box=geometry(mask,ticks[0]);keys=[];failures=0;confidence=[];lost=None
    def gray(t):return np.asarray(sourceframe(p,path,meta,t,size).convert('L'))
    prev=gray(ticks[0]);keys.append(dict(**Box.model_validate(box.model_dump(include=set(Box.model_fields))).model_dump(),t=ticks[0]))
    for i,t in enumerate(ticks[1:],1):
        if event.is_set():raise InterruptedError('Canceled')
        curr=gray(t);progress('Tracking',i/len(ticks))
        if cv2.absdiff(prev,curr).mean()>48:lost=t;break
        x,y,r,b=rect(box,size);region=np.zeros_like(prev);region[y:b,x:r]=255
        pts=cv2.goodFeaturesToTrack(prev,120,.008,3,mask=region);aff=None
        if pts is not None and len(pts)>=6:
            q,ok,_=cv2.calcOpticalFlowPyrLK(prev,curr,pts,None)
            back,ok2,_=cv2.calcOpticalFlowPyrLK(curr,prev,q,None)
            errors=np.linalg.norm(pts-back,axis=2).ravel();valid=(ok.ravel()>0)&(ok2.ravel()>0)&(errors<=1.5)
            if valid.sum()>=6:
                fit,inliers=cv2.estimateAffinePartial2D(pts[valid],q[valid],method=cv2.RANSAC)
                if fit is not None and inliers.sum()>=6 and inliers.mean()>=.5:
                    aff=fit;confidence.append(dict(t=t,ratio=float(inliers.mean()),error=float(np.median(errors[valid]))))
        if aff is None:
            template=prev[y:b,x:r]
            if template.std()>8 and min(template.shape)>=4:
                scores=cv2.matchTemplate(curr,template,cv2.TM_CCOEFF_NORMED);_,val,_,at=cv2.minMaxLoc(scores)
                if val>.9:aff=np.array([[1,0,at[0]-x],[0,1,at[1]-y]],dtype=float)
        if aff is None:
            failures+=1
            if failures>=3:lost=t;break
            continue
        failures=0;cx,cy=aff@np.array([(x+r)/2,(y+b)/2,1]);scale=math.hypot(*aff[0,:2])
        bw=min(1,max(1/w,box.w*scale));bh=min(1,max(1/size[1],box.h*scale))
        box=Box(x=max(0,min(1-bw,cx/w-bw/2)),y=max(0,min(1-bh,cy/size[1]-bh/2)),w=bw,h=bh,angle=box.angle+(math.degrees(math.atan2(aff[1,0],aff[0,0])) if mask.shape=='Freehand' else 0))
        keys.append(dict(**box.model_dump(),t=t));prev=curr
    return dict(keys=keys,confidence=confidence,lost=lost)
def temporal(original,spatial,alpha,p,neighbors):
    ref=np.asarray(original);dst=np.array(spatial);occupied=np.asarray(alpha)>0
    gray=cv2.cvtColor(ref,cv2.COLOR_RGB2GRAY)
    for t,neighbor in neighbors:
        img=np.asarray(neighbor.convert('RGB'));other=cv2.cvtColor(img,cv2.COLOR_RGB2GRAY)
        if cv2.absdiff(gray,other).mean()>30:continue
        shift,response=cv2.phaseCorrelate(other.astype('float32'),gray.astype('float32'))
        if response<.35:continue
        matrix=np.float32([[1,0,shift[0]],[0,1,shift[1]]])
        blocked=np.zeros(gray.shape,dtype='uint8')
        for m in p.masks:
            if m.enabled and m.mode!='Protect' and (not m.ranges or any(a<=t<b for a,b in m.ranges)):blocked=np.maximum(blocked,np.asarray(coverage(m,t,original.size,original.width/p.width)))
        valid=cv2.warpAffine(255-blocked,matrix,original.size)==255
        warped=cv2.warpAffine(img,matrix,original.size)
        difference=cv2.absdiff(ref,warped).mean(axis=2).astype('float32')
        context=(~occupied).astype('float32')
        weight=cv2.blur(context,(31,31))
        error=cv2.blur(difference*context,(31,31))/np.maximum(weight,.001)
        take=occupied&valid&(weight>.1)&(error<12)
        dst[take]=warped[take];occupied[take]=False
    return Image.fromarray(dst)
