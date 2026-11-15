// SPDX-License-Identifier: MPL-2.0
// Bounded static prepared-scene diagnostic. Not the shared engine player.
// Input is console_scene's little-endian CGS3 v1 packet, never native game code.
#include <vita2d.h>
#include <psp2/kernel/processmgr.h>
#include <psp2/kernel/threadmgr.h>
#include <psp2/io/stat.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <malloc.h>
#include <math.h>
#define ROOT "ux0:data/crysirror_scene_validation"
#define MAX_TEX 1024
#define MAX_MAT 4096
#define MAX_DRAW 4096
unsigned int _newlib_heap_size_user=64*1024*1024;
typedef struct { SceGxmProgram *code;SceGxmShaderPatcherId id;SceGxmVertexProgram *vp; } Program;
typedef struct { float rows[24];unsigned tex[7],two_sided; } Material;
typedef struct { unsigned material,nv,ni;float *v;unsigned *i;float depth; } Draw;
static FILE *report,*input;
static SceGxmContext *ctx;
static SceGxmShaderPatcher *patcher;
static vita2d_texture *textures[MAX_TEX],*defaults[3],*hdr,*output;
static Material materials[MAX_MAT];static Draw draws[MAX_DRAW];
static unsigned width,height,nt,nm,nd;
static unsigned vertex_total,index_total,texture_bytes,gpu_bytes;
static float frame[52];
static Program mesh,fragment,full,copy,tone;
static SceGxmFragmentProgram *mesh_fp[5],*copy_fp,*tone_fp;
#define LOG(...) do{fprintf(report,__VA_ARGS__);fputc('\n',report);fflush(report);}while(0)
#define REQUIRE(c,...) do{if(!(c)){LOG("FAIL " __VA_ARGS__);return -1;}}while(0)
static void memory(const char *stage){
    SceKernelFreeMemorySizeInfo info={.size=sizeof(info)};
    int code=sceKernelGetFreeMemorySize(&info);
    LOG("MEMORY %s code=%08x user=%d cdram=%d phycont=%d texture_payload=%u geometry_allocated=%u",stage,code,info.size_user,info.size_cdram,info.size_phycont,texture_bytes,gpu_bytes);
}
static int read_exact(void *data,size_t size){return fread(data,1,size,input)==size;}
static void *gpu_alloc(unsigned bytes){
    unsigned size=(bytes+4095u)&~4095u;void *data=NULL;
    SceUID id=sceKernelAllocMemBlock("crysirror-scene",SCE_KERNEL_MEMBLOCK_TYPE_USER_RW_UNCACHE,size,NULL);
    if(id<0){LOG("ALLOC_FAIL bytes=%u code=%08x",size,id);return NULL;}
    if(sceKernelGetMemBlockBase(id,&data)<0 || sceGxmMapMemory(data,size,SCE_GXM_MEMORY_ATTRIB_READ)<0){LOG("MAP_FAIL bytes=%u",size);return NULL;}
    gpu_bytes+=size;return data;
}
static int program(Program *p,const char *name,int vertex){
    char path[160];snprintf(path,sizeof(path),"app0:gxp/%s.gxp",name);
    FILE *f=fopen(path,"rb");REQUIRE(f,"program open %s",name);
    fseek(f,0,SEEK_END);long bytes=ftell(f);rewind(f);REQUIRE(bytes>=64&&bytes<=1024*1024,"program budget %s",name);
    p->code=memalign(16,bytes);REQUIRE(p->code&&fread(p->code,1,bytes,f)==(size_t)bytes,"program read %s",name);fclose(f);
    int code=sceGxmProgramCheck(p->code);REQUIRE(!code,"program check %s %08x",name,code);
    code=sceGxmShaderPatcherRegisterProgram(patcher,p->code,&p->id);REQUIRE(!code,"program register %s %08x",name,code);
    if(vertex){
        SceGxmVertexAttribute attrs[4];unsigned count=0;
        for(unsigned i=0;i<sceGxmProgramGetParameterCount(p->code);i++){
            const SceGxmProgramParameter *a=sceGxmProgramGetParameter(p->code,i);
            if(sceGxmProgramParameterGetCategory(a)!=SCE_GXM_PARAMETER_CATEGORY_ATTRIBUTE)continue;
            const char *n=sceGxmProgramParameterGetName(a);unsigned offset=0,components=0;
            if(!strcmp(n,"position")){offset=0;components=vertex==1?3:2;}
            else if(!strcmp(n,"normal")&&vertex==1){offset=12;components=3;}
            else if(!strcmp(n,"uv")){offset=vertex==1?24:8;components=2;}
            else if(!strcmp(n,"tangent")&&vertex==1){offset=32;components=4;}
            REQUIRE(components&&count<4,"unknown vertex attribute %s",n);
            attrs[count++]=(SceGxmVertexAttribute){.streamIndex=0,.offset=offset,.format=SCE_GXM_ATTRIBUTE_FORMAT_F32,.componentCount=components,.regIndex=sceGxmProgramParameterGetResourceIndex(a)};
        }
        SceGxmVertexStream stream={.stride=vertex==1?48:16,.indexSource=vertex==1?SCE_GXM_INDEX_SOURCE_INDEX_32BIT:SCE_GXM_INDEX_SOURCE_INDEX_16BIT};
        code=sceGxmShaderPatcherCreateVertexProgram(patcher,p->id,attrs,count,&stream,1,&p->vp);REQUIRE(!code,"vertex create %s %08x",name,code);
    }
    LOG("PROGRAM %s bytes=%ld",name,bytes);return 0;
}
static int pipeline(void){
    if(program(&mesh,"lite_mesh_v",1)||program(&fragment,"lite_mesh_f",0)||program(&full,"lite_fullscreen_v",2)||program(&copy,"lite_copy_f",0)||program(&tone,"lite_tonemap_f",0))return -1;
    for(unsigned i=0;i<5;i++){
        SceGxmBlendInfo blend={.colorMask=SCE_GXM_COLOR_MASK_ALL,.colorFunc=SCE_GXM_BLEND_FUNC_ADD,.alphaFunc=SCE_GXM_BLEND_FUNC_ADD,.colorSrc=i==4?SCE_GXM_BLEND_FACTOR_ONE:SCE_GXM_BLEND_FACTOR_SRC_ALPHA,.colorDst=i==3?SCE_GXM_BLEND_FACTOR_ONE:SCE_GXM_BLEND_FACTOR_ONE_MINUS_SRC_ALPHA,.alphaSrc=SCE_GXM_BLEND_FACTOR_ONE,.alphaDst=i==3?SCE_GXM_BLEND_FACTOR_ONE:SCE_GXM_BLEND_FACTOR_ONE_MINUS_SRC_ALPHA};
        int code=sceGxmShaderPatcherCreateFragmentProgram(patcher,fragment.id,SCE_GXM_OUTPUT_REGISTER_FORMAT_HALF4,SCE_GXM_MULTISAMPLE_NONE,i<2?NULL:&blend,mesh.code,&mesh_fp[i]);
        REQUIRE(!code,"fragment create mode=%u code=%08x",i,code);
    }
    int code=sceGxmShaderPatcherCreateFragmentProgram(patcher,copy.id,SCE_GXM_OUTPUT_REGISTER_FORMAT_HALF4,SCE_GXM_MULTISAMPLE_NONE,NULL,full.code,&copy_fp);REQUIRE(!code,"copy create %08x",code);
    code=sceGxmShaderPatcherCreateFragmentProgram(patcher,tone.id,SCE_GXM_OUTPUT_REGISTER_FORMAT_UCHAR4,SCE_GXM_MULTISAMPLE_NONE,NULL,full.code,&tone_fp);REQUIRE(!code,"tonemap create %08x",code);return 0;
}
static vita2d_texture *pixel(unsigned color){
    vita2d_texture *t=vita2d_create_empty_texture(1,1);if(t)*(unsigned *)vita2d_texture_get_datap(t)=color;return t;
}
static int load(void){
    unsigned h[7];REQUIRE(read_exact(h,sizeof(h))&&h[0]==0x33534743&&h[1]==1,"packet header");
    width=h[2];height=h[3];nt=h[4];nm=h[5];nd=h[6];
    REQUIRE(width>=64&&width<=960&&height>=64&&height<=544&&nt<=MAX_TEX&&nm>0&&nm<=MAX_MAT&&nd>0&&nd<=MAX_DRAW,"packet counts");
    REQUIRE(read_exact(frame,sizeof(frame)),"packet frame");for(unsigned i=0;i<52;i++)REQUIRE(isfinite(frame[i]),"nonfinite frame");
    vita2d_texture_set_alloc_memblock_type(SCE_KERNEL_MEMBLOCK_TYPE_USER_CDRAM_RW);
    for(unsigned i=0;i<nt;i++){
        unsigned p[3];REQUIRE(read_exact(p,sizeof(p))&&p[0]>0&&p[0]<=2048&&p[1]>0&&p[1]<=2048&&p[2]<=1,"texture header %u",i);
        unsigned bytes=p[0]*p[1]*4;REQUIRE(texture_bytes+bytes<=128*1024*1024,"texture total budget");
        textures[i]=vita2d_create_empty_texture(p[0],p[1]);REQUIRE(textures[i],"texture allocation %u %ux%u",i,p[0],p[1]);
        unsigned char *data=vita2d_texture_get_datap(textures[i]);unsigned stride=vita2d_texture_get_stride(textures[i]);
        for(unsigned y=0;y<p[1];y++)REQUIRE(read_exact(data+y*stride,p[0]*4),"texture data %u",i);
        sceGxmTextureSetGammaMode(&textures[i]->gxm_tex,p[2]?SCE_GXM_TEXTURE_GAMMA_BGR:SCE_GXM_TEXTURE_GAMMA_NONE);
        vita2d_texture_set_filters(textures[i],SCE_GXM_TEXTURE_FILTER_LINEAR,SCE_GXM_TEXTURE_FILTER_LINEAR);
        texture_bytes+=bytes;
    }
    memory("textures");
    REQUIRE(read_exact(materials,nm*sizeof(Material)),"materials");
    for(unsigned i=0;i<nm;i++){
        for(unsigned j=0;j<24;j++)REQUIRE(isfinite(materials[i].rows[j]),"material finite");
        REQUIRE(materials[i].rows[12]>=0&&materials[i].rows[12]<=4,"material mode");
        for(unsigned j=0;j<7;j++)REQUIRE(materials[i].tex[j]==~0u||materials[i].tex[j]<nt,"material texture index");
    }
    for(unsigned i=0;i<nd;i++){
        Draw *d=&draws[i];unsigned h[3];REQUIRE(read_exact(h,sizeof(h)),"draw header %u",i);d->material=h[0];d->nv=h[1];d->ni=h[2];
        REQUIRE(d->material<nm&&d->nv>0&&d->nv<=2*1024*1024&&d->ni>0&&d->ni<=6*1024*1024&&d->ni%3==0,"draw counts %u",i);
        vertex_total+=d->nv;index_total+=d->ni;REQUIRE(vertex_total<=2*1024*1024&&index_total<=6*1024*1024,"geometry total budget");
        d->v=gpu_alloc(d->nv*48);d->i=gpu_alloc(d->ni*4);REQUIRE(d->v&&d->i,"geometry allocation %u",i);
        REQUIRE(read_exact(d->v,d->nv*48)&&read_exact(d->i,d->ni*4),"draw payload %u",i);
        for(unsigned j=0;j<d->nv*12;j++)REQUIRE(isfinite(d->v[j]),"nonfinite vertex");
        for(unsigned j=0;j<d->ni;j++)REQUIRE(d->i[j]<d->nv,"index range %u",i);
        float x=d->v[0]-frame[32],y=d->v[1]-frame[33],z=d->v[2]-frame[34];d->depth=x*x+y*y+z*z;
    }
    REQUIRE(fgetc(input)==EOF,"trailing packet bytes");fclose(input);input=NULL;
    LOG("SCENE textures=%u materials=%u draws=%u vertices=%u triangles=%u",nt,nm,nd,vertex_total,index_total/3);memory("geometry");return 0;
}
static void set(Program *p,void *b,const char *name,const float *v,unsigned n){
    const SceGxmProgramParameter *a=sceGxmProgramFindParameterByName(p->code,name);
    if(a){int code=sceGxmSetUniformDataF(b,a,0,n,v);if(code)LOG("UNIFORM_FAIL %s %08x",name,code);}
}
static const float identity[16]={1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1};
static void uniforms(Program *p,int vertex,Material *m,int unlit,int reverse){
    void *b=NULL;if(vertex)sceGxmReserveVertexDefaultUniformBuffer(ctx,&b);else sceGxmReserveFragmentDefaultUniformBuffer(ctx,&b);
    if(!b)return;
    // GXM allocators do not initialize unbound uniform bytes.
    memset(b,0,sceGxmProgramGetDefaultUniformBufferSize(p->code));
    if(vertex){
        set(p,b,"lite_view_proj",frame,16);set(p,b,"lite_view",frame+16,16);set(p,b,"lite_model",identity,16);
        float normal[16];memcpy(normal,identity,sizeof(normal));if(reverse)normal[0]=normal[5]=normal[10]=-1;
        set(p,b,"lite_normal",normal,16);set(p,b,"lite_cascade0",identity,16);set(p,b,"lite_cascade1",identity,16);
    }else if(m){
        set(p,b,"lite_camera",frame+32,4);set(p,b,"lite_sun_dir",frame+36,4);set(p,b,"lite_sun_color",frame+40,4);set(p,b,"lite_env",frame+44,4);
        float sh[36]={3.14159265f,3.14159265f,3.14159265f,0};set(p,b,"lite_sh",sh,36);
        float rows[24];memcpy(rows,m->rows,sizeof(rows));if(unlit)rows[14]=1;set(p,b,"lite_material",rows,24);
    }else if(p==&tone){set(p,b,"lite_post",frame+48,4);}
}
static void full_draw(Program *f,SceGxmFragmentProgram *fp,vita2d_texture *source){
    float *v=vita2d_pool_memalign(48,16);const float data[12]={-1,1,0,0,-1,-3,0,2,3,1,2,0};memcpy(v,data,sizeof(data));
    sceGxmSetVertexProgram(ctx,full.vp);sceGxmSetFragmentProgram(ctx,fp);sceGxmSetVertexStream(ctx,0,v);
    sceGxmSetCullMode(ctx,SCE_GXM_CULL_NONE);sceGxmSetFrontDepthFunc(ctx,SCE_GXM_DEPTH_FUNC_ALWAYS);sceGxmSetFrontDepthWriteEnable(ctx,SCE_GXM_DEPTH_WRITE_DISABLED);
    sceGxmSetFragmentTexture(ctx,0,&source->gxm_tex);sceGxmSetFragmentTexture(ctx,1,&defaults[1]->gxm_tex);sceGxmSetFragmentTexture(ctx,2,&defaults[0]->gxm_tex);uniforms(f,0,NULL,0,0);
    sceGxmDraw(ctx,SCE_GXM_PRIMITIVE_TRIANGLES,SCE_GXM_INDEX_FORMAT_U16,vita2d_get_linear_indices(),3);
}
static int compare_draw(const void *a,const void *b){
    const Draw *x=a,*y=b;int tx=materials[x->material].rows[12]>=2,ty=materials[y->material].rows[12]>=2;
    if(tx!=ty)return tx-ty;return tx?((y->depth>x->depth)-(y->depth<x->depth)):0;
}
static int render(int unlit){
    vita2d_pool_reset();vita2d_start_drawing_advanced(hdr,0);
    sceGxmSetViewport(ctx,width*.5f,width*.5f,height*.5f,-height*.5f,0,1);
    sceGxmSetViewportEnable(ctx,SCE_GXM_VIEWPORT_ENABLED);sceGxmSetTwoSidedEnable(ctx,SCE_GXM_TWO_SIDED_DISABLED);
    sceGxmSetFrontStencilFunc(ctx,SCE_GXM_STENCIL_FUNC_ALWAYS,SCE_GXM_STENCIL_OP_KEEP,SCE_GXM_STENCIL_OP_KEEP,SCE_GXM_STENCIL_OP_KEEP,255,0);
    full_draw(&copy,copy_fp,defaults[1]);
    unsigned passes=0;
    for(unsigned i=0;i<nd;i++){
        Draw *d=&draws[i];Material *m=&materials[d->material];unsigned mode=(unsigned)m->rows[12];
        vita2d_texture *slots[12]={defaults[0],defaults[0],defaults[2],defaults[0],defaults[0],defaults[1],defaults[0],defaults[0],defaults[1],defaults[1],defaults[0],defaults[0]};
        const unsigned map[7]={0,1,2,3,4,10,11};for(unsigned j=0;j<7;j++)if(m->tex[j]!=~0u)slots[map[j]]=textures[m->tex[j]];
        if(m->tex[4]==~0u)slots[4]=defaults[0];
        for(unsigned pass=0;pass<(m->two_sided?2u:1u);pass++){
            sceGxmSetVertexProgram(ctx,mesh.vp);sceGxmSetFragmentProgram(ctx,mesh_fp[mode]);sceGxmSetVertexStream(ctx,0,d->v);
            sceGxmSetCullMode(ctx,pass?SCE_GXM_CULL_CCW:SCE_GXM_CULL_CW);sceGxmSetFrontDepthFunc(ctx,SCE_GXM_DEPTH_FUNC_LESS_EQUAL);sceGxmSetFrontDepthWriteEnable(ctx,mode<2?SCE_GXM_DEPTH_WRITE_ENABLED:SCE_GXM_DEPTH_WRITE_DISABLED);
            for(unsigned j=0;j<12;j++)sceGxmSetFragmentTexture(ctx,j,&slots[j]->gxm_tex);
            uniforms(&mesh,1,NULL,0,pass);uniforms(&fragment,0,m,unlit,0);
            int code=sceGxmDraw(ctx,SCE_GXM_PRIMITIVE_TRIANGLES,SCE_GXM_INDEX_FORMAT_U32,d->i,d->ni);REQUIRE(!code,"draw %u %08x",i,code);passes++;
        }
        if(i%256==0)LOG("DRAW_PROGRESS mode=%s draw=%u",unlit?"unlit":"lit",i);
    }
    vita2d_end_drawing();sceGxmFinish(ctx);
    vita2d_start_drawing_advanced(output,0);sceGxmSetViewport(ctx,width*.5f,width*.5f,height*.5f,-height*.5f,0,1);
    full_draw(&tone,tone_fp,hdr);vita2d_end_drawing();sceGxmFinish(ctx);
    volatile unsigned first=*(volatile unsigned *)vita2d_texture_get_datap(output);(void)first;
    vita2d_start_drawing_advanced(output,0);vita2d_end_drawing();sceGxmFinish(ctx);sceKernelDelayThread(20000);
    char path[180];snprintf(path,sizeof(path),ROOT "/scene-%s.rgba",unlit?"unlit":"lit");FILE *f=fopen(path,"wb");REQUIRE(f,"capture open");
    unsigned char *data=vita2d_texture_get_datap(output);unsigned stride=vita2d_texture_get_stride(output);
    for(unsigned y=0;y<height;y++)REQUIRE(fwrite(data+y*stride,4,width,f)==width,"capture write");fclose(f);
    LOG("CAPTURE %s %ux%u draw_passes=%u",path,width,height,passes);memory(unlit?"unlit":"lit");return 0;
}
int main(void){
    sceIoMkdir(ROOT,0777);report=fopen(ROOT "/report.txt","w");if(!report)return 1;
    LOG("START prepared static scene; heap=67108864; no original executable; no game VM");memory("entry");
    if(vita2d_init_advanced(2*1024*1024)<0){LOG("FAIL GXM init");return 1;}ctx=vita2d_get_context();patcher=vita2d_get_shader_patcher();memory("gxm");
    input=fopen(ROOT "/scene.bin","rb");if(!input){LOG("FAIL packet open");return 1;}
    if(load()||pipeline()){memory("failed");return 1;}
    defaults[0]=pixel(RGBA8(255,255,255,255));defaults[1]=pixel(RGBA8(0,0,0,255));defaults[2]=pixel(RGBA8(128,128,255,255));
    hdr=vita2d_create_empty_texture_rendertarget(width,height,SCE_GXM_TEXTURE_FORMAT_F16F16F16F16_ABGR);
    output=vita2d_create_empty_texture_rendertarget(width,height,SCE_GXM_TEXTURE_FORMAT_A8B8G8R8);
    if(!hdr||!output||!defaults[0]||!defaults[1]||!defaults[2]){LOG("FAIL targets");memory("failed");return 1;}
    sceGxmDepthStencilSurfaceSetBackgroundDepth(&hdr->gxm_sfd,1.f);sceGxmColorSurfaceSetGammaMode(&output->gxm_sfc,SCE_GXM_COLOR_SURFACE_GAMMA_BGR);
    qsort(draws,nd,sizeof(Draw),compare_draw);memory("ready");
    if(render(1)||render(0)){LOG("FAIL render");return 1;}
    LOG("DONE");fclose(report);sceKernelExitProcess(0);return 0;
}
