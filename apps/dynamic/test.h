#ifndef TEST_H
#define TEST_H

#include <stdio.h>
#include <stdlib.h>

extern int t_status;

#define t_error(fmt, ...) (fprintf(stderr, fmt, ##__VA_ARGS__), (t_status = 1))

#endif
