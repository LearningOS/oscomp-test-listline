#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/wait.h>

int 
main()
{
  int p[2];
  char byte = 'x';
  char buf[1024];
  pipe(p);
  if (fork() == 0) {
    while (read(p[0], &buf, 1024) > 0) {
      close(p[0]);
      printf("%d: received ping\n", getpid());
      write(p[1], &byte, 1);
      close(p[1]);
      exit(0);
    }
  }
  else {
    write(p[1], &byte, 1);
    close(p[1]);
    wait(0);
    while (read(p[0], &buf, 1024) > 0) {
      close(p[0]);
      printf("%d: received pong\n", getpid());
    }
  }
  exit(0);
}
