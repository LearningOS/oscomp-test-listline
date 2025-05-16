#include <sys/stat.h>
#include <errno.h>
#include <string.h>
#include <stdio.h>
#include <time.h>
#include <stdint.h>
#include <unistd.h>
#include <fcntl.h>
#include <unistd.h>
#include <stdarg.h>
#include <unistd.h>

#define T_LOC2(l) __FILE__ ":" #l
#define T_LOC1(l) T_LOC2(l)
#define t_error(...) t_printf(T_LOC1(__LINE__) ": " __VA_ARGS__)
#define DISABLE_SLOW_TESTS

int t_status = 0;

int t_printf(const char *s, ...)
{
	va_list ap;
	char buf[512];
	int n;

	t_status = 1;
	va_start(ap, s);
	n = vsnprintf(buf, sizeof buf, s, ap);
	va_end(ap);
	if (n < 0)
		n = 0;
	else if (n >= sizeof buf) {
		n = sizeof buf;
		buf[n - 1] = '\n';
		buf[n - 2] = '.';
		buf[n - 3] = '.';
		buf[n - 4] = '.';
	}
	return write(1, buf, n);
}

#define TEST(c, ...) ((c) ? 1 : (t_error(#c" failed: " __VA_ARGS__),0))
#define TESTVAL(v,op,x) TEST(v op x, "%jd\n", (intmax_t)(v))

int main(void)
{
	struct stat st;
	int fd;
	time_t t;

	TEST(utimensat(AT_FDCWD, "/dev/null/invalid", ((struct timespec[2]){{.tv_nsec=UTIME_OMIT},{.tv_nsec=UTIME_OMIT}}), 0)==0 || errno==ENOTDIR,
		"%s\n", strerror(errno));
	TEST(futimens(-1, ((struct timespec[2]){{.tv_nsec=UTIME_OMIT},{.tv_nsec=UTIME_OMIT}}))==0 || errno==EBADF,
		"%s\n", strerror(errno));

	if (!TEST(fd = open("./stat_c", O_RDWR))) return t_status;

	TEST(futimens(fd, (struct timespec[2]){0}) == 0, "\n");
	TEST(fstat(fd, &st) == 0, "\n");
	TESTVAL(st.st_atim.tv_sec,==,0);
	TESTVAL(st.st_atim.tv_nsec,==,0);
	TESTVAL(st.st_mtim.tv_sec,==,0);
	TESTVAL(st.st_mtim.tv_nsec,==,0);

	TEST(futimens(fd, ((struct timespec[2]){{.tv_sec=1,.tv_nsec=UTIME_OMIT},{.tv_sec=1,.tv_nsec=UTIME_OMIT}})) == 0, "\n");
	TEST(fstat(fd, &st) == 0, "\n");
	TESTVAL(st.st_atim.tv_sec,==,0);
	TESTVAL(st.st_atim.tv_nsec,==,0);
	TESTVAL(st.st_mtim.tv_sec,==,0);
	TESTVAL(st.st_mtim.tv_nsec,==,0);

	t = time(0);

	TEST(futimens(fd, ((struct timespec[2]){{.tv_nsec=UTIME_NOW},{.tv_nsec=UTIME_OMIT}})) == 0, "\n");
	TEST(fstat(fd, &st) == 0, "\n");
	TESTVAL(st.st_atim.tv_sec,>=,t);
	TESTVAL(st.st_mtim.tv_sec,==,0);
	TESTVAL(st.st_mtim.tv_nsec,==,0);
	
	TEST(futimens(fd, (struct timespec[2]){0}) == 0, "\n");
	TEST(futimens(fd, ((struct timespec[2]){{.tv_nsec=UTIME_OMIT},{.tv_nsec=UTIME_NOW}})) == 0, "\n");
	TEST(fstat(fd, &st) == 0, "\n");
	TESTVAL(st.st_atim.tv_sec,==,0);
	TESTVAL(st.st_mtim.tv_sec,>=,t);

	TEST(futimens(fd, ((struct timespec[2]){{.tv_nsec=UTIME_NOW},{.tv_nsec=UTIME_OMIT}})) == 0, "\n");
	TEST(fstat(fd, &st) == 0, "\n");
	TESTVAL(st.st_atim.tv_sec,>=,t);
	TESTVAL(st.st_mtim.tv_sec,>=,t);

  // lwext4 only support u32
	if (TEST((time_t)(1LL<<32) == (1LL<<32), "implementation has Y2038 EOL\n")) {
		if (TEST(futimens(fd, ((struct timespec[2]){{.tv_sec=1LL<<16},{.tv_sec=1LL<<16}})) == 0, "%s\n", strerror(errno))) {
			TEST(fstat(fd, &st) == 0, "\n");
			TESTVAL(st.st_atim.tv_sec, ==, 1LL<<16);
			TESTVAL(st.st_mtim.tv_sec, ==, 1LL<<16);
		}
	}

	close(fd);

  if (t_status == 0) 
    printf("Pass!");
	return t_status;
}
