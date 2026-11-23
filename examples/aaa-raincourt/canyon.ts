// The player follows the stone floor from the gallery to its broken descending balcony.
let distance=0,sideways=0,yawOffset=0,pitchOffset=0,auto=false;
let eye=[0,0,0],nextCheck=0;
const clusters:any[]=[];
function model(name:string,x:number,y:number,z:number,scale:number[],yaw:number):Entity {
  const e=world.importModel(name+".gltf",{asScene:true}).roots[0] as Entity;
  e.position=[x,y,z];e.scale=scale;e.rotation=[0,yaw,0];return e;
}
function plant(name:string,x:number,y:number,z:number,s:number,a:number):void {
  const e=model(name,x,y,z,[s,s,s],a);
  if(name==="heliconia"||name==="palm"||name==="fern")e.play("Breeze",{loop:true,speed:.85+.20*Math.abs(Math.sin(x+z))});
}
function pose():void {
  // The corner is rounded at the mouth; the far city remains in sight as the trail turns.
  const s=distance,k=Math.max(0,Math.min(1,(s-18)/5)),angle=k*Math.PI*.5;
  const after=Math.max(0,s-23);
  let x=-.25+(1-Math.cos(angle))*3.18+.98*after;
  let z=20-Math.min(18,s)-3.18*Math.sin(angle)-after*.26;
  x+=sideways*Math.cos(angle);z+=sideways*Math.sin(angle);
  const floor=s<18?11.99:11.98-.18*Math.max(0,x);
  eye=[x,floor+1.59,z];
  const progress=Math.max(0,Math.min(1,(s-17)/19));
  const aim=[-7+23*progress,19-16*progress,-62+7*progress];
  const dx=aim[0]-x,dz=aim[2]-z;
  const az=Math.atan2(dx,dz)+yawOffset;
  const pitch=Math.atan2(aim[1]-eye[1],Math.hypot(dx,dz))+pitchOffset;
  world.setCamera({eye:eye,target:[x+Math.sin(az)*Math.cos(pitch),eye[1]+Math.sin(pitch),z+Math.cos(az)*Math.cos(pitch)],fov:63,near:.07,far:310});
}
function demand():void {
  for(const c of clusters){
    const d=Math.hypot(c.x-eye[0],c.y-eye[1],c.z-eye[2]);
    if(!c.ticket&&d>c.load&&d<=c.load+6)c.ticket=assets.load(c.asset,"level");
    const outside=d>c.load+11;
    if(c.ticket&&outside&&c.outside){assets.release(c.ticket);c.ticket=0;}
    c.outside=outside;
  }
}
function onStart():void {
  for(const [a,k] of [["forward","w"],["back","s"],["left","a"],["right","d"],["fast","shift"],["tour","t"],["turnl","q"],["turnr","e"],["look","mouse_right"],["home","1"]])input.bind(a,[k]);
  world.setEnvironment({hdr:"ravine-day.hdr",intensity:.48,rotation_degrees:93,exposure:-.34,tonemap:"aces",
    temporal_aa:true,ambient_occlusion:.94,bloom:.032,shadow_distance:115,
    sun:{direction:[.53,-.77,-.49],color:[1,.85,.66],intensity:4.55,shadows:true},
    reflections:{steps:24,max_distance:38,thickness:.24,max_roughness:.30,intensity:.64},
    fog:{color:[.28,.40,.40],density:.0041,height_falloff:.038,base_height:-6,start_distance:18,max_opacity:.73,sun_scattering:.33}});
  pose();
  for(const n of ["gallery","gorge","cataract","pool"])model(n,0,0,0,[1,1,1],0);
  model("citadel",1.44,-3.78,2.84,[1.18,1.42,1.04],0);
  // Granitic buttresses behind and alongside the arcades; real scanned geometry with an irregular skyline.
  for(const b of [[-39,6,-81,2.5,2.8,2.9,12],[9,4,-92,3.4,4.5,3.0,-12],
    [-25,5,-101,3.9,6.1,3.2,16],[48,1,-84,3.1,4.15,3.9,-25],[-71,2,-67,3.2,4.45,4.2,26],
    [71,0,-105,5,5.7,4.0,-23],[-86,3,-119,5.6,6.5,5.0,13],
    [-28,-23,-53,1.75,4.3,2.1,11],[28,-22,-48,2.0,4.4,2.2,-18],
    [-26,-24,-10,1.1,5.4,2.3,98],[40,-25,-7,1.37,5.8,2.4,-85]])
    model("bluff",b[0],b[1],b[2],[b[3],b[4],b[5]],b[6]);
  for(const a of [[-4.9,11.3,.6,1.04,30],[-6.2,11.2,2.8,1.58,70],[6.4,11.2,2,.9,-40],[18.8,8.5,-6.4,1.51,32],
    [-15,12,-62,1.35,115],[-22,15,-65,1.1,30],[7,12,-65,1.2,50],[22,12,-63,.9,110],[-40,11,-65,1.3,180]])
    plant("palm",a[0],a[1],a[2],a[3],a[4]);
  for(const a of [[-3.5,11.4,.35,.85],[3.8,11.35,.35,.82],[13,9.35,-4.5,.74],[19.0,8.2,-5,1.1]])
    plant("heliconia",a[0],a[1],a[2],a[3],65);
  // Folded crowns and roots frame the viewing edge and lap over the far eaves.
  plant("palm",-6.0,10.5,-4.7,1.30,-48);
  plant("heliconia",-2.7,11.2,-2,.78,140);
  plant("fern",-2.1,11.9,-1.1,2.6,12);
  for(const p of [[-18,5,-60,1.5],[-27,4,-60,1.8],[5,4,-61,1.35],[15,3,-58,1.3],[25,4,-58.8,1.7],
    [-5,13,-68,1.05],[-18,15,-70,1.13],[7,9,-69,1.1],[-35,5,-61,1.5],[-47,6,-60,1.8]])
    model("creepers",p[0],p[1],p[2],[p[3],p[3],p[3]],0);
  for(const p of [[-12,-24,-55,1.9,3.7],[0,-24,-51,1.25,3.25],[18,-25,-58,1.35,4.1],[-45,-25,-54,1.4,4.1]])
    model("bluff",p[0],p[1],p[2],[p[3],p[4],1.6],p[0]*.45);
  let seed=74897;
  function rand():number{seed=(seed*1664525+1013904223)>>>0;return seed/4294967296;}
  for(let i=0;i<175;i++){
    const x=-90+rand()*180,z=-70-rand()*18;
    const y=7+.1*Math.abs(x)+2.1*Math.sin(x*.11);
    if(Math.abs(x+7)<17 && z>-80)continue;
    const s=1.3+rand()*2.5;
    plant(i%4===0?"tree":"tree-proxy",x,y,z,s,rand()*360);
  }
  for(const p of [[-17,14,-69,1.35],[4,14,-70,.94],[-30,19,-66,.95],[20,16,-76,1.35],[-15,21,-82,.9]])
    plant("tree",p[0],p[1],p[2],p[3],p[0]*13);
  for(const p of [[-7,9,-65.4,1.3],[17,10,-68.8,.89],[-28,11,-62.3,.82],[-13,20,-78.8,.86]])
    model("creepers",p[0],p[1],p[2],[p[3],p[3],p[3]],0);
  for(let i=0;i<28;i++){
    const x=(i%2?1:-1)*(4+rand()*25),z=3+rand()*25;
    plant(i%6===0?"palm":i%3===0?"tree":"heliconia",x,11.5,z,.75+rand()*1.0,rand()*360);
  }
  for(const p of [[-2.8,11.55,5,.88,45],[3.1,11.32,4.95,.63,180],[4.3,11.0,-.4,.71,62],[9.9,9.8,-3.2,.55,235]])
    plant("heliconia",p[0],p[1],p[2],p[3],p[4]);
  model("roots",-2.05,12.0,8,[.7,.78,1.8],45);
  model("strangler",-3.4,11.3,3.8,[.83,.90,.86],-29);
  for(const p of [[-25,12.1,-65],[21,12.9,-64],[-5,20,-83]])model("roots",p[0],p[1],p[2],[2.4,2.8,2.6],24);
  const layout=JSON.parse(fs.readText("layout.json"));
  for(const c of layout.cells){
    const x=c.center[0]*1.2,z=c.center[2]+18,y=11.5;
    const load= c.center[2]>0?23:20;
    clusters.push({asset:c.name+".gltf",x:x,y:y,z:z,load:load,ticket:0,outside:false});
    world.streamLevel(c.name,{source:c.name+".gltf",center:[x,y,z],transform:{translation:[x,y,z]},load_radius:load,unload_radius:load+7,pbr:true});
  }
}
function onUpdate(ms:number):void {
  const dt=Math.min(ms/1000,.08);
  if(input.pressed("home")){distance=0;sideways=0;yawOffset=0;pitchOffset=0;auto=false;}
  if(input.pressed("tour"))auto=!auto;
  const forward=input.axis("back","forward");
  if(forward)auto=false;
  distance=Math.max(0,Math.min(36.5,distance+(auto?3.8:forward*(input.isDown("fast")?6.6:3.1))*dt));
  sideways=Math.max(-.84,Math.min(.84,sideways+input.axis("left","right")*dt*1.8));
  yawOffset-=input.axis("turnl","turnr")*dt*1.3;
  if(input.isDown("look")){yawOffset-=input.pointerDelta[0]*.004;pitchOffset=Math.max(-1.1,Math.min(1.1,pitchOffset-input.pointerDelta[1]*.004));}
  pose();nextCheck-=dt;
  if(nextCheck<=0){if(dt>0)demand();nextCheck=.17;}
}
