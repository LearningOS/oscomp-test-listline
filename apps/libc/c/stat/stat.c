#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <string.h>

#define FILE_PATH "test_file.txt"

int main() {
    // Create the file
    int fd = open(FILE_PATH, O_CREAT | O_RDWR | O_TRUNC, S_IRUSR | S_IWUSR);
    if (fd == -1) {
        perror("Failed to create file");
        exit(EXIT_FAILURE);
    }
   // Data to write into the file
    const char *data = "Hello, world!\n";
    size_t len = strlen(data);

    // Write the data to the file
    ssize_t bytes_written = write(fd, data, len);
    if (bytes_written == -1) {
        perror("Failed to write to file");
        close(fd);
        exit(EXIT_FAILURE);
    }

    char readbuf[20];
    ssize_t bytes_read = read(fd, readbuf, len);
    if (bytes_read == -1) {
        perror("Failed to read to file");
        close(fd);
        exit(EXIT_FAILURE);
    }

    // Get file status
    struct stat sb;
    if (stat(FILE_PATH, &sb) == -1) {
        perror("Failed to stat file");
        exit(EXIT_FAILURE);
    }
    close(fd);
    return 0;
}
