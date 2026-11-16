// SPDX-License-Identifier: MPL-2.0
// Native GXM admission/linkage and synthetic offscreen draws. No game data.
#include <vita2d.h>
#include <psp2/kernel/processmgr.h>
#include <psp2/kernel/threadmgr.h>
#include <psp2/io/fcntl.h>
#include <psp2/io/stat.h>
#include <malloc.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#define ROOT "ux0:data/crysirror_shader_validation"
#define W 64
#define MAX_PROGRAMS 256
unsigned int _newlib_heap_size_user = 128 * 1024 * 1024;
typedef struct {
    char name[128];
    SceGxmProgram *gxp;
    SceGxmShaderPatcherId id;
    SceGxmVertexProgram *vp;
    SceGxmVertexAttribute attrs[16];
    unsigned count;
    int vertex;
} Program;
static Program programs[MAX_PROGRAMS];
static unsigned count;
static FILE *report;
static SceGxmContext *ctx;
static SceGxmShaderPatcher *patcher;
static vita2d_texture *image, *target;
static unsigned failures;
#define LOG(...) do {fprintf(report,__VA_ARGS__); fputc('\n',report); fflush(report);} while(0)
static Program *find(const char *name) {
    for(unsigned i=0;i<count;i++) if(!strcmp(programs[i].name,name)) return &programs[i];
    LOG("MISSING %s",name);failures++;return NULL;
}
static int load(void) {
    FILE *index=fopen("app0:programs.txt","r");if(!index)return -1;
    char name[128],path[200];
    while(fgets(name,sizeof(name),index)) {
        name[strcspn(name,"\r\n")]=0;if(!*name)continue;
        if(count>=MAX_PROGRAMS || strspn(name,"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_")!=strlen(name))return -1;
        Program *p=&programs[count++];strcpy(p->name,name);
        snprintf(path,sizeof(path),"app0:gxp/%s.gxp",name);
        FILE *file=fopen(path,"rb");if(!file)return -1;
        fseek(file,0,SEEK_END);long size=ftell(file);rewind(file);
        if(size<64 || size>1024*1024)return -1;
        p->gxp=memalign(16,size);if(!p->gxp)return -1;
        if(fread(p->gxp,1,size,file)!=(size_t)size)return -1;
        fclose(file);
        int code=sceGxmProgramCheck(p->gxp);
        unsigned admitted=sceGxmProgramGetSize(p->gxp);
        if(!code && (admitted<64 || admitted>(unsigned)size))code=-2;
        if(admitted!=(unsigned)size)LOG("CONTAINER %s program=%u file=%ld",name,admitted,size);
        if(!code)code=sceGxmShaderPatcherRegisterProgram(patcher,p->gxp,&p->id);
        p->vertex=sceGxmProgramGetType(p->gxp)==SCE_GXM_VERTEX_PROGRAM;
        LOG("REGISTER %s %08x",name,code);if(code){failures++;continue;}
        if(p->vertex) {
            unsigned n=sceGxmProgramGetParameterCount(p->gxp);
            for(unsigned j=0;j<n;j++) {
                const SceGxmProgramParameter *a=sceGxmProgramGetParameter(p->gxp,j);
                if(sceGxmProgramParameterGetCategory(a)!=SCE_GXM_PARAMETER_CATEGORY_ATTRIBUTE)continue;
                unsigned components=sceGxmProgramParameterGetComponentCount(a);
                if(p->count==16 || components<1 || components>4)return -1;
                p->attrs[p->count]=(SceGxmVertexAttribute){.streamIndex=0,.offset=p->count*16,.format=SCE_GXM_ATTRIBUTE_FORMAT_F32,.componentCount=components,.regIndex=sceGxmProgramParameterGetResourceIndex(a)};
                LOG("ATTRIBUTE %s %s %u %u",name,sceGxmProgramParameterGetName(a),p->count*16,components);p->count++;
            }
            SceGxmVertexStream stream={.stride=256,.indexSource=SCE_GXM_INDEX_SOURCE_INDEX_16BIT};
            code=sceGxmShaderPatcherCreateVertexProgram(patcher,p->id,p->attrs,p->count,&stream,1,&p->vp);
            LOG("VERTEX %s %08x",name,code);if(code)failures++;
        }
    }
    fclose(index);return 0;
}
static const char *paired(const char *name) {
    if(strstr(name,"shared_quad_"))return "shared_quad_vs_main_v";
    if(strstr(name,"shared_wipe_"))return "shared_wipe_vs_main_v";
    if(strstr(name,"shared_puppet_fs_unpremultiply"))return "shared_puppet_vs_full_v";
    if(strstr(name,"shared_puppet_"))return "shared_puppet_vs_part_v";
    if(strstr(name,"shared_ordered_dither_"))return "shared_ordered_dither_vs_main_v";
    if(strstr(name,"shared_composite_filter_"))return "shared_present_vs_main_v";
    if(strstr(name,"shared_composite_"))return "shared_composite_vs_main_v";
    if(strstr(name,"shared_present_"))return "shared_present_vs_main_v";
    if(!strcmp(name,"ray_lite_f"))return "ray_lite_v";
    if(strstr(name,"lite_legacy_mesh"))return "lite_legacy_mesh_v";
    if(strstr(name,"lite_mesh"))return "lite_mesh_v";
    if(strstr(name,"lite_shadow"))return "lite_shadow_v";
    if(strstr(name,"lite_particle"))return "lite_particle_v";
    if(strstr(name,"lite_vfx_"))return "lite_vfx_sprite_v";
    return "lite_fullscreen_v";
}
static SceGxmFragmentProgram *link_program(Program *f,Program *v) {
    SceGxmFragmentProgram *fp=NULL;
    int code=sceGxmShaderPatcherCreateFragmentProgram(patcher,f->id,SCE_GXM_OUTPUT_REGISTER_FORMAT_UCHAR4,SCE_GXM_MULTISAMPLE_NONE,NULL,v->gxp,&fp);
    LOG("LINK %s %s %08x",f->name,v->name,code);if(code || !fp)failures++;
    return fp;
}
static void uniform(Program *p,void *buffer,const char *name,const float *values,unsigned n) {
    const SceGxmProgramParameter *a=sceGxmProgramFindParameterByName(p->gxp,name);
    if(!a)return;
    int code=sceGxmSetUniformDataF(buffer,a,0,n,values);
    if(code){LOG("UNIFORM_FAIL %s %s %08x",p->name,name,code);failures++;}
}
static void bind_uniforms(Program *p,int vertex,int sampling,int distortion) {
    if(!sceGxmProgramGetDefaultUniformBufferSize(p->gxp))return;
    void *buffer=NULL;
    int code=vertex?sceGxmReserveVertexDefaultUniformBuffer(ctx,&buffer):sceGxmReserveFragmentDefaultUniformBuffer(ctx,&buffer);
    if(code || !buffer){LOG("RESERVE_FAIL %s %08x",p->name,code);failures++;return;}
    float dims[]={W,W,1.f/W,1.f/W};
    unsigned n=sceGxmProgramGetParameterCount(p->gxp);
    for(unsigned j=0;j<n;j++) {
        const SceGxmProgramParameter *a=sceGxmProgramGetParameter(p->gxp,j);
        if(sceGxmProgramParameterGetCategory(a)!=SCE_GXM_PARAMETER_CATEGORY_UNIFORM)continue;
        const char *name=sceGxmProgramParameterGetName(a);
        unsigned len=sceGxmProgramParameterGetComponentCount(a)*sceGxmProgramParameterGetArraySize(a);
        if(len>64 || !len)continue;
        float data[64]={0};
        if(!strncmp(name,"cg_size_",8))memcpy(data,dims,sizeof(dims));
        if(!strcmp(name,"cg_loop_limits")){data[0]=10;data[1]=3;data[2]=3;data[3]=16;}
        if(!strcmp(name,"selected_filter.h"))data[1]=data[2]=data[3]=1;
        if(!strcmp(name,"selected_filter.f")){data[0]=.299f;data[1]=.587f;data[2]=.114f;}
        if(!strcmp(name,"selected_filter.c")){int flags[]={0,96,32,192,128};data[3]=flags[sampling];}
        if(!strcmp(name,"selected_filter.d")){data[2]=distortion;data[3]=1;}
        if(!strcmp(name,"selected_filter.e")){data[0]=1;data[1]=.5f;}
        if(!strcmp(name,"lite_view_proj")){data[0]=data[5]=data[10]=data[15]=1;}
        if(!strcmp(name,"wipe.kind_progress")){data[1]=.5f;data[2]=data[3]=W;}
        if(!strcmp(name,"ray_nodes")){data[0]=4;data[1]=1;data[2]=1;}
        if(!strcmp(name,"ray_triangles")){data[0]=8;data[1]=1;data[2]=1;}
        if(!strcmp(name,"ray_eye"))data[3]=.001f;
        if(!strcmp(name,"ray_corner")){data[0]=-1;data[1]=1;data[2]=-1;}
        if(!strcmp(name,"ray_dx"))data[0]=2;
        if(!strcmp(name,"ray_dy"))data[1]=-2;
        if(!strcmp(name,"ray_light")){data[2]=1;data[3]=100;}
        if(!strcmp(name,"ray_sun"))data[0]=data[1]=data[2]=.75f;
        if(!strcmp(name,"ray_ambient"))data[0]=data[1]=data[2]=.25f;
        if(!strcmp(name,"ray_sky_low")||!strcmp(name,"ray_sky_high")){data[1]=.25f;data[2]=.5f;}
        LOG("UNIFORM %s %s type=%u offset=%u components=%u array=%u values=%g,%g,%g,%g",p->name,name,sceGxmProgramParameterGetType(a),sceGxmProgramParameterGetResourceIndex(a),sceGxmProgramParameterGetComponentCount(a),sceGxmProgramParameterGetArraySize(a),data[0],data[1],data[2],data[3]);
        uniform(p,buffer,name,data,len);
    }
}
static void vertices(Program *v) {
    float *data=vita2d_pool_memalign(3*256,16);memset(data,0,3*256);
    const float xy[3][2]={{-1,1},{-1,-3},{3,1}},uv[3][2]={{0,0},{0,2},{2,0}};
    unsigned n=sceGxmProgramGetParameterCount(v->gxp),slot=0;
    for(unsigned j=0;j<n;j++) {
        const SceGxmProgramParameter *a=sceGxmProgramGetParameter(v->gxp,j);
        if(sceGxmProgramParameterGetCategory(a)!=SCE_GXM_PARAMETER_CATEGORY_ATTRIBUTE)continue;
        const char *name=sceGxmProgramParameterGetName(a);
        for(unsigned k=0;k<3;k++) {
            float *out=data+k*64+slot*4;
            if(strstr(name,"vertex_index"))out[0]=k;
            else if(strstr(name,"position")){
                const float mesh[3][2]={{-.8f,-.8f},{.8f,-.8f},{0,.8f}};
                int is_mesh=strstr(v->name,"lite_legacy")!=NULL;
                out[0]=is_mesh?mesh[k][0]:xy[k][0];out[1]=is_mesh?mesh[k][1]:xy[k][1];out[2]=.5f;out[3]=1;
            }
            else if(strstr(name,"uv")){out[0]=uv[k][0];out[1]=uv[k][1];}
            else if(strstr(name,"normal"))out[2]=1;
            else if(strstr(name,"color")){
                out[0]=out[1]=out[2]=out[3]=1;
                if(strstr(v->name,"lite_legacy")){out[0]=.2f;out[1]=.8f;out[2]=.4f;}
            }
            else if(strstr(name,"sample_bounds")||strstr(name,"clip_rect")){out[0]=out[1]=-100;out[2]=out[3]=100;}
        }
        slot++;
    }
    sceGxmSetVertexStream(ctx,0,data);
}
static void draw(const char *label,const char *f_name,int sampling,int distortion,vita2d_texture **textures,unsigned num) {
    Program *f=find(f_name),*v=find(paired(f_name));if(!f||!v||!v->vp)return;
    SceGxmFragmentProgram *fp=link_program(f,v);if(!fp)return;
    vita2d_pool_reset();
    memset(vita2d_texture_get_datap(target),0xcd,vita2d_texture_get_stride(target)*W);
    vita2d_start_drawing_advanced(target,0);
    vita2d_set_clear_color(RGBA8(17,23,37,255));
    vita2d_clear_screen();
    sceGxmSetVertexProgram(ctx,v->vp);sceGxmSetFragmentProgram(ctx,fp);
    sceGxmSetViewport(ctx,W/2.f,W/2.f,W/2.f,-W/2.f,0,1);
    sceGxmSetViewportEnable(ctx,SCE_GXM_VIEWPORT_ENABLED);sceGxmSetCullMode(ctx,SCE_GXM_CULL_NONE);
    sceGxmSetTwoSidedEnable(ctx,SCE_GXM_TWO_SIDED_DISABLED);
    sceGxmSetFrontStencilFunc(ctx,SCE_GXM_STENCIL_FUNC_ALWAYS,SCE_GXM_STENCIL_OP_KEEP,SCE_GXM_STENCIL_OP_KEEP,SCE_GXM_STENCIL_OP_KEEP,255,0);
    sceGxmSetFrontDepthFunc(ctx,SCE_GXM_DEPTH_FUNC_ALWAYS);sceGxmSetFrontDepthWriteEnable(ctx,SCE_GXM_DEPTH_WRITE_DISABLED);
    for(unsigned i=0;i<num;i++)sceGxmSetFragmentTexture(ctx,i,&textures[i]->gxm_tex);
    bind_uniforms(v,1,sampling,distortion);bind_uniforms(f,0,sampling,distortion);vertices(v);
    int code=sceGxmDraw(ctx,SCE_GXM_PRIMITIVE_TRIANGLES,SCE_GXM_INDEX_FORMAT_U16,vita2d_get_linear_indices(),3);
    vita2d_end_drawing();vita2d_wait_rendering_done();sceGxmFinish(ctx);
    // Vulkan surface read protection records a read request for the next
    // scene boundary. A second empty scene makes that readback explicit.
    volatile unsigned first=*(volatile unsigned *)vita2d_texture_get_datap(target);
    (void)first;
    vita2d_start_drawing_advanced(target,0);vita2d_end_drawing();sceGxmFinish(ctx);
    sceKernelDelayThread(20000);
    unsigned stride=vita2d_texture_get_stride(target)/4;
    volatile unsigned *src=vita2d_texture_get_datap(target);
    unsigned *copy=malloc(W*W*4);
    for(unsigned y=0;y<W;y++)for(unsigned x=0;x<W;x++)copy[y*W+x]=src[y*stride+x];
    char path[200];snprintf(path,sizeof(path),ROOT "/%s.rgba",label);
    FILE *out=fopen(path,"wb");if(out){fwrite(copy,4,W*W,out);fclose(out);}else failures++;
    LOG("DRAW %s %08x center=%08x corner=%08x stride=%u",label,code,copy[(W/2)*W+W/2],copy[0],stride);
    free(copy);if(code)failures++;
    // Keep patched programs/resources alive until process exit: Vita3K may
    // translate queued work after a native completion callback.
}
int main(void) {
    sceIoMkdir(ROOT,0777);report=fopen(ROOT "/report.txt","w");if(!report)return 1;
    LOG("START native GXM offscreen validation");
    if(vita2d_init_advanced(1024*1024)<0){LOG("INIT_FAIL");return 1;}
    vita2d_set_vblank_wait(0);ctx=vita2d_get_context();patcher=vita2d_get_shader_patcher();
    if(!ctx||!patcher||load()){LOG("LOAD_FAIL");return 1;}
    unsigned linked=0;
    for(unsigned i=0;i<count;i++)if(!programs[i].vertex){
        Program *v=find(paired(programs[i].name));if(!v)continue;
        SceGxmFragmentProgram *fp=link_program(&programs[i],v);
        if(fp){linked++;sceGxmShaderPatcherReleaseFragmentProgram(patcher,fp);}
    }
    LOG("CATALOG programs=%u linked_fragments=%u failures=%u",count,linked,failures);
    target=vita2d_create_empty_texture_rendertarget(W,W,SCE_GXM_TEXTURE_FORMAT_A8B8G8R8);
    image=vita2d_create_empty_texture(W,W);if(!target||!image){LOG("TARGET_FAIL");return 1;}
    unsigned *pixels=vita2d_texture_get_datap(image),stride=vita2d_texture_get_stride(image)/4;
    for(unsigned y=0;y<W;y++)for(unsigned x=0;x<W;x++)pixels[y*stride+x]=RGBA8(x*4,y*4,(x^y)*4,255);
    vita2d_texture_set_filters(image,SCE_GXM_TEXTURE_FILTER_POINT,SCE_GXM_TEXTURE_FILTER_POINT);
    vita2d_texture *tex[4]={image,image,image,image};
    draw("present","shared_present_fs_copy_f",0,0,tex,1);
    draw("quad","shared_quad_fs_main_f",0,0,tex,1);
    for(int sampling=0;sampling<5;sampling++) {
        char name[128],label[64];snprintf(name,sizeof(name),"shared_quad_fs_filtered_s%d_d0_f",sampling);snprintf(label,sizeof(label),"filtered-s%d",sampling);
        draw(label,name,sampling,0,tex,4);
    }
    draw("wipe","shared_wipe_fs_main_k0_f",0,0,tex,4);
    draw("mesh","lite_legacy_mesh_f",0,0,tex,1);
    vita2d_texture *nodes=vita2d_create_empty_texture_format(4,1,SCE_GXM_TEXTURE_FORMAT_F32F32_GR);
    vita2d_texture *triangles=vita2d_create_empty_texture_format(8,1,SCE_GXM_TEXTURE_FORMAT_F32F32_GR);
    if(nodes&&triangles){
        float n[]={-1,-1,-2,1,1,1,-2,1};float t[]={-1,-1,-2,1,1,-1,-2,0,0,1,-2,0,0,0,0,0};
        memcpy(vita2d_texture_get_datap(nodes),n,sizeof(n));memcpy(vita2d_texture_get_datap(triangles),t,sizeof(t));
        vita2d_texture_set_filters(nodes,SCE_GXM_TEXTURE_FILTER_POINT,SCE_GXM_TEXTURE_FILTER_POINT);vita2d_texture_set_filters(triangles,SCE_GXM_TEXTURE_FILTER_POINT,SCE_GXM_TEXTURE_FILTER_POINT);
        vita2d_texture *raytex[]={nodes,triangles};draw("ray","ray_lite_f",0,0,raytex,2);
    }else{LOG("RAY_TEXTURE_FAIL");failures++;}
    LOG("DONE failures=%u",failures);fclose(report);sceKernelExitProcess(failures?1:0);return 0;
}
