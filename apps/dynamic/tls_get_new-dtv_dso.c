#include <stdio.h>
#include <pthread.h>

// 定义线程局部存储变量
static __thread int tls_var = 0;

void f(void) {
    // 修改并打印线程局部存储变量
    tls_var++;
    printf("Thread %lu: tls_var = %d\n", (unsigned long)pthread_self(), tls_var);
}
