#pragma once
#include "../../common/console.h"

/* Uses vita2d's embedded GXP programs; no runtime shader compiler. */
bool sena_vita_scene_render(void *host, const SenaCursor *cursor);
void sena_vita_scene_free(void);
void sena_vita_scene_log_memory(const char *stage, uint64_t frame);
