#ifndef SENA_CONSOLE_H
#define SENA_CONSOLE_H
#include "sena.h"
#include <math.h>
#include <stdbool.h>
#include <string.h>

typedef struct {
  int x, y, w, h;
} SenaViewport;
typedef struct {
  double x, y;
  bool left, right, skip, hide;
} SenaCursor;

static inline SenaViewport sena_fit(int w, int h, int sw, int sh) {
  if (sw <= 0 || sh <= 0)
    return (SenaViewport){0, 0, w, h};
  double scale = fmin((double)w / sw, (double)h / sh);
  int dw = (int)(sw * scale), dh = (int)(sh * scale);
  return (SenaViewport){(w - dw) / 2, (h - dh) / 2, dw, dh};
}

/* Every host uses the same letterbox rectangle for rendering and input. */
static inline void sena_cursor_update(void *host, SenaCursor *c, SenaViewport v,
                                      int lw, int lh, double dx, double dy,
                                      bool touch, double tx, double ty,
                                      bool confirm, bool cancel, bool skip,
                                      bool hide) {
  touch = touch && tx >= v.x && tx < v.x + v.w && ty >= v.y && ty < v.y + v.h;
  if (touch) {
    c->x = tx;
    c->y = ty;
  } else {
    c->x += dx;
    c->y += dy;
  }
  c->x = fmax(v.x, fmin(v.x + v.w - 1, c->x));
  c->y = fmax(v.y, fmin(v.y + v.h - 1, c->y));
  sena_host_resize(host, lw, lh);
  sena_host_touch(host, 1, (c->x - v.x) * lw / v.w, (c->y - v.y) * lh / v.h);
  bool left = confirm || touch;
  if (left != c->left)
    sena_host_mouse_button(host, 0, left);
  if (cancel != c->right)
    sena_host_mouse_button(host, 1, cancel);
  if (skip != c->skip)
    sena_host_key(host, "Control", skip);
  if (hide != c->hide)
    sena_host_key(host, "Space", hide);
  c->left = left;
  c->right = cancel;
  c->skip = skip;
  c->hide = hide;
}

static inline double sena_axis(double value) {
  return fabs(value) < 0.18 ? 0.0 : value;
}

/* The simple console host presents the shared PAL software compositor. */
static inline void sena_blit(uint8_t *dst, size_t pitch, int w, int h,
                             const uint8_t *src, int sw, int sh, SenaViewport v,
                             const SenaCursor *cursor) {
  for (int y = 0; y < h; ++y) {
    uint32_t *row = (uint32_t *)(dst + y * pitch);
    for (int x = 0; x < w; ++x)
      row[x] = 0xff000000;
    if (!src || y < v.y || y >= v.y + v.h)
      continue;
    int sy = (int)((int64_t)(y - v.y) * sh / v.h);
    for (int x = 0; x < v.w; ++x) {
      int sx = (int)((int64_t)x * sw / v.w);
      memcpy(&row[v.x + x], src + ((size_t)sy * sw + sx) * 4, 4);
    }
  }
  int cx = (int)cursor->x, cy = (int)cursor->y;
  for (int d = -6; d <= 6; ++d) {
    if (cx + d >= 0 && cx + d < w && cy >= 0 && cy < h)
      ((uint32_t *)(dst + cy * pitch))[cx + d] = 0xffffffff;
    if (cy + d >= 0 && cy + d < h && cx >= 0 && cx < w)
      ((uint32_t *)(dst + (cy + d) * pitch))[cx] = 0xffffffff;
  }
}
#endif
