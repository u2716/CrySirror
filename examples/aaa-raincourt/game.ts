// The Rain Court. Coordinates are metres. Three composed views share one live world.
const views = [
  {eye:[8.2, 3.15, 18.2], aim:[-.65, 4.25, -2.7], fov:51},
  {eye:[-6.9, 1.46, 12.05], aim:[1.45, 2.55, -2.05], fov:62},
  {eye:[-3.1, 3.5, 2.65], aim:[.8, 6.17, -3.7], fov:67},
];
let eye = views[0].eye.slice(), target = views[0].aim.slice();
let view = 0, fov = views[0].fov, time = 0, tour = false, walk = false;
let bounce: Entity, liveBounce = false;
let turn = 0, pitch = 0, nextLod = 0, lastTour = -1;
const grove: any[] = [];
// A request owns a CPU ticket, separate from the streamer's entity/lease.
// Start workers in the approach band; leave visibility to world.streamLevel.
const prefetchCells: any[] = [];
const loadRadius = 34, unloadRadius = 41, prepareRadius = 40, releaseRadius = 44;
function prepareNearbyCells(): void {
  for (const c of prefetchCells) {
    const distance = Math.hypot(eye[0]-c.center[0], eye[1]-c.center[1], eye[2]-c.center[2]);
    if (!c.ticket && distance > loadRadius && distance <= prepareRadius) {
      c.ticket = assets.load(c.name+".gltf","level");
    }
    // The world unload runs after onUpdate. Require two outside observations
    // so that its pin has gone even after teleporting from an active cell.
    if (c.ticket && distance > releaseRadius && c.wasOutside) {
      assets.release(c.ticket); c.ticket = 0;
    }
    c.wasOutside = distance > releaseRadius;
  }
}

function setView(n: number): void {
  view = n; eye = views[n].eye.slice(); target = views[n].aim.slice(); fov = views[n].fov;
  turn = Math.atan2(target[0]-eye[0], target[2]-eye[2]);
  pitch = Math.atan2(target[1]-eye[1], Math.hypot(target[0]-eye[0],target[2]-eye[2]));
  walk = false;
}
function place(name: string, p: number[], size: number[], angle: number): Entity {
  const e = world.importModel(name+".gltf", {asScene:true}).roots[0] as Entity;
  e.position = p; e.scale = size; e.rotation = [0,angle,0]; return e;
}
function lod(): void {
  for (const t of grove) {
    const d = Math.hypot(eye[0]-t.x,eye[2]-t.z);
    const near = d < (t.near ? 23 : 19);
    if (near !== t.near || !t.ready) {
      t.full.visible = near; t.proxy.visible = !near; t.near = near; t.ready = true;
    }
  }
}
function camera(): void {
  world.setCamera({eye:eye,target:target,fov:fov,near:.08,far:170});
}
function onStart(): void {
  for (const pair of [["bounce","4"],["wide","1"],["brook","2"],["dial","3"],["tour","t"],["forward","w"],
                      ["back","s"],["left","a"],["right","d"],["ccw","q"],["cw","e"],["fast","shift"],["look","mouse_right"]]) {
    input.bind(pair[0],[pair[1]]);
  }
  world.setEnvironment({
    hdr:"rainforest.hdr",intensity:.32,rotation_degrees:100,exposure:-.31,tonemap:"aces",
    ambient_occlusion:.85,temporal_aa:true,bloom:.025,shadow_distance:49,
    sun:{direction:[.57,-.91,-.56],color:[1,.85,.64],intensity:4.8,shadows:true},
    reflections:{steps:24,max_distance:25,thickness:.24,max_roughness:.30,intensity:.68},
    fog:{color:[.32,.45,.40],density:.009,height_falloff:.072,base_height:.6,start_distance:13,max_opacity:.70,sun_scattering:.38},
  });
  bounce=world.spawn({name:"court indirect irradiance",position:[0,4,-1.5]});
  setView(0); camera();
  for (const name of ["earth","sanctuary","creepers","strangler","horizon-0","horizon-1","horizon-2","horizon-3"]) {
    place(name,[0,0,0],[1,1,1],0);
  }
  place("guardian",[0,6.31,-.04],[4.65,4.65,4.65],0);
  place("creek",[0,0,0],[1,1,1],0).play("Current",{loop:true});
  const layout=JSON.parse(fs.readText("layout.json"));
  let i=0;
  for (const item of layout.props) {
    const plant=place(item.model,item.position,item.scale,item.yaw);
    if (item.model==="palm" || item.model==="fern" || item.model==="heliconia") plant.play("Breeze",{loop:true,speed:.74+(i%7)*.08});
    i++;
  }
  for (const tree of layout.trees) {
    const s=[tree.scale,tree.scale,tree.scale];
    const full=place("tree",tree.position,s,tree.yaw);
    const proxy=place("tree-proxy",tree.position,s,tree.yaw);
    grove.push({full:full,proxy:proxy,x:tree.position[0],z:tree.position[2],near:false,ready:false});
  }
  for (const cell of layout.cells) {
    prefetchCells.push({name:cell.name,center:cell.center,ticket:0,wasOutside:false});
    world.streamLevel(cell.name,{source:cell.name+".gltf",center:cell.center,
      transform:{translation:cell.center},load_radius:loadRadius,unload_radius:unloadRadius,pbr:true});
  }
  // Local sunlit mist below gaps in the crown. The opaque scene occludes each proxy.
  const openings=JSON.parse(fs.readText("shaft-depth.json"));
  for(const opening of openings){
    const {start,end,radius,depth}=opening;
    const mid=start.map((v:number,k:number)=>(v+end[k])*.5);
    const size=start.map((v:number,k:number)=>Math.abs(v-end[k])+2*radius);
    world.spawn({name:"lit canopy opening",position:mid,scale:size,geometry:"cube",
      material:{alpha_mode:"additive",unlit:true,double_sided:false,base_color_factor:[1,1,1,1],
        effect:{kind:"custom",asset:"sunshaft.wgsl",parameters:[[...start,radius],[...end,.14],depth.slice(0,4),depth.slice(4,8)]}}});
  }
  lod();
}
function onUpdate(dtMs: number): void {
  const dt=Math.min(dtMs*.001,.08);time+=dt;
  if (input.pressed("bounce")) {
    liveBounce=!liveBounce;
    bounce.setIrradianceVolume(liveBounce ? {extent:[6,3.9,5.7],counts:[3,2,3],intensity:.86,
      visibility:true,probes_per_frame:1,idle_probes_per_frame:0} : null);
  }
  if (input.pressed("wide")) {tour=false;setView(0);}
  if (input.pressed("brook")) {tour=false;setView(1);}
  if (input.pressed("dial")) {tour=false;setView(2);}
  if (input.pressed("tour")) {tour=!tour;time=0;lastTour=-1;}
  if (tour) {
    const n=Math.floor(time/7)%3;
    if(n!==lastTour){setView(n);lastTour=n;}
    const phase=(time%7)/7;
    eye=[views[n].eye[0]-.8*phase,views[n].eye[1]+.18*phase,views[n].eye[2]-.72*phase];
  }
  const forward=input.axis("back","forward"), side=input.axis("left","right");
  const spin=input.axis("ccw","cw"), look=input.isDown("look");
  if (forward||side||spin||look) {tour=false;walk=true;}
  if (walk) {
    turn-=spin*dt*1.3;
    if(look){turn-=input.pointerDelta[0]*.004;pitch=Math.max(-1.1,Math.min(1.1,pitch-input.pointerDelta[1]*.004));}
    const speed=dt*(input.isDown("fast")?6.3:3.15);
    const x=Math.max(-28,Math.min(28,eye[0]+speed*(forward*Math.sin(turn)-side*Math.cos(turn))));
    const z=Math.max(-21,Math.min(27,eye[2]+speed*(forward*Math.cos(turn)+side*Math.sin(turn))));
    const channel=9.5+1.85*Math.sin(x*.165)+1.2*Math.sin(x*.076+.6);
    let floor=.15+.12*Math.sin(x*.43+z*.16)+.14*Math.sin(z*.34)-1.9*Math.exp(-Math.pow(Math.abs(z-channel)/3.5,3));
    floor=Math.max(-.72,floor);
    if(Math.abs(x)<3.55&&z>1&&z<11.5)floor=Math.max(floor,-.27+.18*Math.round((10.9-z)/.76));
    if(Math.abs(x)<8.35&&z<=1.05&&z>-9.2)floor=Math.max(floor,1.42);
    eye=[x,eye[1]+(floor+1.58-eye[1])*Math.min(1,dt*8),z];
    target=[x+Math.sin(turn)*Math.cos(pitch),eye[1]+Math.sin(pitch),z+Math.cos(turn)*Math.cos(pitch)];
  }
  nextLod-=dt;
  if(nextLod<=0){lod();if(dt>0)prepareNearbyCells();nextLod=.25;}
  camera();
}
