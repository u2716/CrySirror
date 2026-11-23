#ifndef SENA_VITA_CPU_MEMORY_H
#define SENA_VITA_CPU_MEMORY_H
#include <stdbool.h>
#include <stddef.h>
size_t sena_vita_cpu_reserved(void);
size_t sena_vita_cpu_peak(void);
void *sena_vita_cpu_alloc(size_t bytes, bool zero);
void sena_vita_cpu_free(void *ptr);
#endif
