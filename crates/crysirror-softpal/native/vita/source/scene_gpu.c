#include "cpu_memory.h"
#include "scene_gpu.h"
#include <stdlib.h>
#include <stdio.h>
#include <malloc.h>
/* newlib malloc.h defines quad as quad_t; the shared scene has a quad field. */
#ifdef quad
#undef quad
#endif
#include <psp2/kernel/sysmem.h>
#include <vita2d.h>

enum { MaxTextures = 1024, MaxDraws = 4096, MaxRetired = 4096 };
typedef struct {
  uint64_t id, generation;
  uint32_t width, height;
  bool used, seen;
  vita2d_texture *texture;
} Texture;
static Texture textures[MaxTextures];
static vita2d_texture *white;
static SenaSceneDraw draws[MaxDraws];
static struct { vita2d_texture *texture; uint64_t frame; } retired[MaxRetired];
static uint64_t frame_number;
/* Leave CDRAM for vita2d framebuffers and shader pools, and uncached user RAM
 * for stacks and drivers beside the separately reserved 336 MiB CPU heap.
 * Both budgets include textures waiting for their four-frame retirement. */
enum { CdramBudget = 80 * 1024 * 1024, UncachedBudget = 8 * 1024 * 1024 };
static size_t cdram_reserved, uncached_reserved, cdram_peak, uncached_peak;

static bool texture_block(vita2d_texture *texture, SceKernelMemBlockInfo *info) {
  *info = (SceKernelMemBlockInfo){.size = sizeof(*info)};
  return sceKernelGetMemBlockInfoByAddr(vita2d_texture_get_datap(texture), info) >= 0;
}

static void free_texture(vita2d_texture *texture) {
  if (!texture)
    return;
  SceKernelMemBlockInfo info;
  if (texture_block(texture, &info)) {
    size_t *reserved = info.type == SCE_KERNEL_MEMBLOCK_TYPE_USER_CDRAM_RW
                           ? &cdram_reserved : &uncached_reserved;
    *reserved = *reserved >= info.mappedSize ? *reserved - info.mappedSize : 0;
  }
  vita2d_free_texture(texture);
}

/* Reuse vita2d's linked GXP programs; the GXM patcher only supplies blend state.
 * No Cg compiler or libshacccg module is needed at build time or runtime. */
extern const unsigned char texture_v_gxp_start[];
extern const unsigned char texture_tint_f_gxp_start[];
extern SceGxmFragmentProgram *_vita2d_textureTintFragmentProgram;
static SceGxmShaderPatcherId blend_program_id;
static SceGxmFragmentProgram *blend_programs[12];

static void release_blend_programs(void) {
  SceGxmShaderPatcher *patcher = vita2d_get_shader_patcher();
  for (unsigned mode = 1; mode <= 11; ++mode) {
    if (blend_programs[mode])
      sceGxmShaderPatcherReleaseFragmentProgram(patcher, blend_programs[mode]);
    blend_programs[mode] = NULL;
  }
  if (blend_program_id)
    sceGxmShaderPatcherUnregisterProgram(patcher, blend_program_id);
  blend_program_id = NULL;
}

static bool init_blend_programs(void) {
  if (blend_programs[1])
    return true;
  SceGxmShaderPatcher *patcher = vita2d_get_shader_patcher();
  if (sceGxmShaderPatcherRegisterProgram(patcher,
      (const SceGxmProgram *)texture_tint_f_gxp_start, &blend_program_id) < 0)
    return false;
  for (unsigned mode = 1; mode <= 11; ++mode) {
    SceGxmBlendInfo blend = {
        .colorMask = SCE_GXM_COLOR_MASK_ALL,
        .colorFunc = SCE_GXM_BLEND_FUNC_ADD,
        .alphaFunc = SCE_GXM_BLEND_FUNC_ADD,
        .colorSrc = SCE_GXM_BLEND_FACTOR_SRC_ALPHA,
        .colorDst = SCE_GXM_BLEND_FACTOR_ONE_MINUS_SRC_ALPHA,
        .alphaSrc = SCE_GXM_BLEND_FACTOR_ONE,
        .alphaDst = SCE_GXM_BLEND_FACTOR_ONE_MINUS_SRC_ALPHA};
    switch (mode) {
    case 2: blend.colorDst = SCE_GXM_BLEND_FACTOR_ONE; break;
    case 3: case 5:
      blend.colorSrc = SCE_GXM_BLEND_FACTOR_ZERO;
      blend.colorDst = SCE_GXM_BLEND_FACTOR_SRC_COLOR;
      break;
    case 4:
      blend.colorSrc = SCE_GXM_BLEND_FACTOR_ZERO;
      blend.colorDst = SCE_GXM_BLEND_FACTOR_DST_COLOR;
      break;
    case 6:
      blend.colorSrc = SCE_GXM_BLEND_FACTOR_ONE_MINUS_DST_COLOR;
      blend.colorDst = SCE_GXM_BLEND_FACTOR_ONE;
      break;
    case 7: case 8: case 9: case 10:
      blend.colorSrc = blend.colorDst = SCE_GXM_BLEND_FACTOR_ONE;
      blend.colorFunc = mode == 7 ? SCE_GXM_BLEND_FUNC_MIN
          : mode == 8 ? SCE_GXM_BLEND_FUNC_MAX
          : mode == 10 ? SCE_GXM_BLEND_FUNC_REVERSE_SUBTRACT : SCE_GXM_BLEND_FUNC_ADD;
      break;
    case 11:
      blend.colorFunc = blend.alphaFunc = SCE_GXM_BLEND_FUNC_NONE;
      break;
    }
    if (sceGxmShaderPatcherCreateFragmentProgram(patcher, blend_program_id,
        SCE_GXM_OUTPUT_REGISTER_FORMAT_UCHAR4, SCE_GXM_MULTISAMPLE_NONE,
        &blend, (const SceGxmProgram *)texture_v_gxp_start, &blend_programs[mode]) < 0) {
      release_blend_programs();
      return false;
    }
  }
  return true;
}

static vita2d_texture *create_texture(uint32_t width, uint32_t height) {
  SceKernelMemBlockType previous = vita2d_texture_get_alloc_memblock_type();
  size_t bytes = (size_t)((width + 7) & ~7u) * height * 4;
  size_t cdram_size = (bytes + 0x3ffff) & ~(size_t)0x3ffff;
  size_t uncached_size = (bytes + 0xfff) & ~(size_t)0xfff;
  vita2d_texture *texture = NULL;
  if (bytes >= 256 * 1024 && cdram_size <= CdramBudget - cdram_reserved) {
    vita2d_texture_set_alloc_memblock_type(SCE_KERNEL_MEMBLOCK_TYPE_USER_CDRAM_RW);
    texture = vita2d_create_empty_texture(width, height);
  }
  if (!texture && uncached_size <= UncachedBudget - uncached_reserved) {
    // Match the upstream allocator: CDRAM exhaustion can use uncached RAM.
    vita2d_texture_set_alloc_memblock_type(SCE_KERNEL_MEMBLOCK_TYPE_USER_RW_UNCACHE);
    texture = vita2d_create_empty_texture(width, height);
  }
  vita2d_texture_set_alloc_memblock_type(previous);
  if (texture) {
    // Older vita2d leaves these non-render-target fields uninitialized.
    texture->gxm_rtgt = NULL;
    texture->depth_UID = 0;
    SceKernelMemBlockInfo info;
    if (!texture_block(texture, &info)) {
      vita2d_free_texture(texture);
      return NULL;
    }
    if (info.type == SCE_KERNEL_MEMBLOCK_TYPE_USER_CDRAM_RW) {
      cdram_reserved += info.mappedSize;
      if (cdram_reserved > cdram_peak) cdram_peak = cdram_reserved;
    } else {
      uncached_reserved += info.mappedSize;
      if (uncached_reserved > uncached_peak) uncached_peak = uncached_reserved;
    }
  }
  return texture;
}

void sena_vita_scene_log_memory(const char *stage, uint64_t frame) {
  SceKernelFreeMemorySizeInfo free = {.size = sizeof(free)};
  int rc = sceKernelGetFreeMemorySize(&free);
  struct mallinfo heap = mallinfo();
  size_t live_bytes = 0, retired_bytes = 0, live_count = 0, retired_count = 0;
  for (size_t i = 0; i < MaxTextures; ++i) {
    if (textures[i].texture) {
      live_bytes += (size_t)vita2d_texture_get_stride(textures[i].texture) *
                    textures[i].height;
      ++live_count;
    }
  }
  for (size_t i = 0; i < MaxRetired; ++i) {
    if (retired[i].texture) {
      retired_bytes += (size_t)vita2d_texture_get_stride(retired[i].texture) *
                       vita2d_texture_get_height(retired[i].texture);
      ++retired_count;
    }
  }
  fprintf(stderr, "[memory] stage=%s frame=%llu rc=%x heap_arena=%u heap_used=%u heap_free=%u "
          "user_free=%d cdram_free=%d phycont_free=%d textures=%u texture_bytes=%u "
          "retired=%u retired_bytes=%u gpu_cdram=%u gpu_uncached=%u "
          "peak_cdram=%u peak_uncached=%u cpu_large=%u peak_cpu_large=%u\n", stage, (unsigned long long)frame, rc,
          (unsigned)heap.arena, (unsigned)heap.uordblks, (unsigned)heap.fordblks, free.size_user,
          free.size_cdram, free.size_phycont, (unsigned)live_count,
          (unsigned)live_bytes, (unsigned)retired_count, (unsigned)retired_bytes,
          (unsigned)cdram_reserved, (unsigned)uncached_reserved,
          (unsigned)cdram_peak, (unsigned)uncached_peak,
          (unsigned)sena_vita_cpu_reserved(), (unsigned)sena_vita_cpu_peak());
  fflush(stderr);
}

/* The original launcher supplied this engine diagnostic. Keep it with the
 * retained output driver so it does not require the standalone app main. */
void sena_vita_image_allocation(size_t bytes) {
  fprintf(stderr, "[image-allocation] frame=%llu requested=%u\n",
          (unsigned long long)frame_number, (unsigned)bytes);
  sena_vita_scene_log_memory("before-image", frame_number);
}

static bool retire_texture(vita2d_texture *texture) {
  if (!texture)
    return true;
  for (size_t i = 0; i < MaxRetired; ++i) {
    if (!retired[i].texture) {
      retired[i].texture = texture;
      retired[i].frame = frame_number;
      return true;
    }
  }
  return false;
}

static Texture *find_texture(uint64_t id) {
  for (size_t i = 0; i < MaxTextures; ++i)
    if (textures[i].used && textures[i].id == id)
      return &textures[i];
  return NULL;
}

/* Called after vita2d_wait_rendering_done: no outstanding GPU reads may
 * overlap texture writes or eviction. Only textures referenced this frame
 * occupy the cache; off-screen PAL assets remain owned by the Rust core. */
static bool sync_textures(void *host, size_t count) {
  for (size_t i = 0; i < MaxTextures; ++i)
    textures[i].seen = false;
  for (size_t i = 0; i < count; ++i) {
    Texture *t = find_texture(draws[i].texture_id);
    if (t)
      t->seen = true;
  }
  for (size_t i = 0; i < MaxTextures; ++i) {
    if (textures[i].used && !textures[i].seen) {
      if (!retire_texture(textures[i].texture))
        return false;
      textures[i] = (Texture){0};
    }
  }
  for (size_t i = 0, n = sena_host_scene_texture_count(host); i < n; ++i) {
    SenaSceneTexture src;
    if (!sena_host_scene_texture(host, i, &src))
      return false;
    bool needed = false;
    for (size_t j = 0; j < count; ++j)
      if (draws[j].texture_id == src.id && src.id != 0)
        needed = true;
    if (!needed)
      continue;
    if (!src.rgba || !src.width || !src.height || src.width > 4096 ||
        src.height > 4096 || src.byte_len != (size_t)src.width * src.height * 4)
      return false;
    Texture *dst = find_texture(src.id);
    if (!dst) {
      for (size_t j = 0; j < MaxTextures; ++j) {
        if (!textures[j].used) {
          dst = &textures[j];
          *dst = (Texture){.id = src.id, .used = true};
          break;
        }
      }
    }
    if (!dst)
      return false;
    if (!dst->texture || dst->width != src.width || dst->height != src.height ||
        dst->generation != src.generation) {
      vita2d_texture *replacement = create_texture(src.width, src.height);
      if (!replacement)
        return false;
      if (!retire_texture(dst->texture)) {
        free_texture(replacement);
        return false;
      }
      dst->texture = replacement;
      dst->width = src.width;
      dst->height = src.height;
      dst->generation = ~src.generation;
      sceGxmTextureSetUAddrMode(&dst->texture->gxm_tex, SCE_GXM_TEXTURE_ADDR_CLAMP);
      sceGxmTextureSetVAddrMode(&dst->texture->gxm_tex, SCE_GXM_TEXTURE_ADDR_CLAMP);
    }
    if (dst->generation != src.generation) {
      uint8_t *pixels = vita2d_texture_get_datap(dst->texture);
      size_t stride = vita2d_texture_get_stride(dst->texture);
      for (uint32_t y = 0; y < src.height; ++y)
        memcpy(pixels + y * stride, src.rgba + (size_t)y * src.width * 4,
               (size_t)src.width * 4);
      dst->generation = src.generation;
    }
  }
  for (size_t i = 0; i < count; ++i) {
    Texture *t = find_texture(draws[i].texture_id);
    if (draws[i].texture_id && (!t || !t->texture))
      return false;
  }
  return true;
}

static uint32_t color_u32(const float color[4]) {
  uint32_t packed = 0;
  for (unsigned i = 0; i < 4; ++i)
    packed |= (uint32_t)lroundf(fminf(1, fmaxf(0, color[i])) * 255) << (i * 8);
  return packed;
}

static void draw_quad(const SenaSceneDraw *draw, vita2d_texture_vertex *vertices,
                      SenaViewport v, int lw, int lh) {
  Texture *t = find_texture(draw->texture_id);
  vita2d_texture *texture = t ? t->texture : white;
  float uv[4][2];
  memcpy(uv, draw->uv, sizeof(uv));
  bool smooth = false;
  if (t && draw->smooth_upscale) {
    float dx = hypotf(draw->quad[1][0] - draw->quad[0][0],
                      draw->quad[1][1] - draw->quad[0][1]);
    float dy = hypotf(draw->quad[3][0] - draw->quad[0][0],
                      draw->quad[3][1] - draw->quad[0][1]);
    float du = uv[1][0] - uv[0][0], dv = uv[3][1] - uv[0][1];
    smooth = dx * v.w / lw > du * t->width * 1.1f ||
             dy * v.h / lh > dv * t->height * 1.1f;
    if (smooth) {
      float iu = fminf(0.5f / t->width, du * 0.25f);
      float iv = fminf(0.5f / t->height, dv * 0.25f);
      uv[0][0] += iu;
      uv[3][0] += iu;
      uv[1][0] -= iu;
      uv[2][0] -= iu;
      uv[0][1] += iv;
      uv[1][1] += iv;
      uv[2][1] -= iv;
      uv[3][1] -= iv;
    }
  }
  SceGxmTextureFilter filter = smooth ? SCE_GXM_TEXTURE_FILTER_LINEAR
                                    : SCE_GXM_TEXTURE_FILTER_POINT;
  vita2d_texture_set_filters(texture, filter, filter);
  const unsigned order[4] = {0, 1, 3, 2};
  for (unsigned i = 0; i < 4; ++i) {
    unsigned j = order[i];
    vertices[i] = (vita2d_texture_vertex){
        .x = v.x + draw->quad[j][0] * v.w / lw,
        .y = v.y + draw->quad[j][1] * v.h / lh,
        .z = 0.5f, .u = uv[j][0], .v = uv[j][1]};
  }
  unsigned mode = draw->render_mode >= 1 && draw->render_mode <= 11 ? draw->render_mode : 1;
  float color[4];
  memcpy(color, draw->color, sizeof(color));
  if (draw->texture_rgb || mode == 11)
    color[0] = color[1] = color[2] = 1;
  SceGxmFragmentProgram *previous = _vita2d_textureTintFragmentProgram;
  _vita2d_textureTintFragmentProgram = blend_programs[draw->blend_disabled ? 11 : mode];
  vita2d_draw_array_textured(texture, SCE_GXM_PRIMITIVE_TRIANGLE_STRIP,
                            vertices, 4, color_u32(color));
  _vita2d_textureTintFragmentProgram = previous;
}

bool sena_vita_scene_render(void *host, const SenaCursor *cursor) {
  ++frame_number;
  // Match siglus_rs: Vita3K may retain a texture beyond sceGxmFinish.
  // Replacement also allocates fresh storage, avoiding writes to old readers.
  for (size_t i = 0; i < MaxRetired; ++i) {
    if (retired[i].texture && frame_number - retired[i].frame >= 4) {
      free_texture(retired[i].texture);
      retired[i].texture = NULL;
    }
  }
  size_t count = sena_host_scene_draw_count(host);
  const float *clear = sena_host_scene_clear_color(host);
  int lw = sena_host_frame_width(host), lh = sena_host_frame_height(host);
  if (!clear || lw <= 0 || lh <= 0 || count > MaxDraws)
    return false;
  if (!init_blend_programs())
    return false;
  for (size_t i = 0; i < count; ++i) {
    if (!sena_host_scene_draw(host, i, &draws[i]) || draws[i].raster[3] != 0)
      return false;
  }
  if (!white) {
    white = create_texture(1, 1);
    if (!white)
      return false;
    *(uint32_t *)vita2d_texture_get_datap(white) = 0xffffffff;
  }
  if (!sync_textures(host, count))
    return false;
  SenaViewport viewport = sena_fit(960, 544, lw, lh);
  vita2d_start_drawing();
  vita2d_texture_vertex *vertices = vita2d_pool_memalign(
      (count + 1) * 4 * sizeof(vita2d_texture_vertex), 16);
  if (!vertices) {
    vita2d_end_drawing();
    vita2d_wait_rendering_done();
    return false;
  }
  vita2d_disable_clipping();
  vita2d_clear_screen();
  vita2d_set_blend_mode_add(0);
  vita2d_set_clip_rectangle(viewport.x, viewport.y,
                           viewport.x + viewport.w, viewport.y + viewport.h);
  vita2d_enable_clipping();
  SenaSceneDraw background = {
      .quad = {{0, 0}, {lw, 0}, {lw, lh}, {0, lh}},
      .color = {clear[0], clear[1], clear[2], 1}};
  draw_quad(&background, vertices, viewport, lw, lh);
  for (size_t i = 0; i < count; ++i)
    draw_quad(&draws[i], vertices + (i + 1) * 4, viewport, lw, lh);
  vita2d_disable_clipping();
  vita2d_draw_rectangle(cursor->x - 5, cursor->y, 11, 1, 0xffffffff);
  vita2d_draw_rectangle(cursor->x, cursor->y - 5, 1, 11, 0xffffffff);
  vita2d_end_drawing();
  vita2d_swap_buffers();
  return true;
}

void sena_vita_scene_free(void) {
  vita2d_wait_rendering_done();
  release_blend_programs();
  for (size_t i = 0; i < MaxTextures; ++i) {
    if (textures[i].texture)
      free_texture(textures[i].texture);
    textures[i] = (Texture){0};
  }
  if (white)
    free_texture(white);
  white = NULL;
  for (size_t i = 0; i < MaxRetired; ++i) {
    if (retired[i].texture)
      free_texture(retired[i].texture);
    retired[i].texture = NULL;
  }
}
